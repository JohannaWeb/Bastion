#!/usr/bin/env python3
"""Small GPT-style byte-level language model with Monarch optimizations."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Optional

import torch
from torch import nn
from torch.nn import functional as F

# Register custom ops
try:
    import pytorch_fork  # noqa: F401
    HAS_TRITON_OPS = True
except ImportError:
    HAS_TRITON_OPS = False


@dataclass
class GPTConfig:
    vocab_size: int = 256
    block_size: int = 256
    n_layer: int = 6
    n_head: int = 6
    n_embd: int = 384
    dropout: float = 0.1
    # Monarch optimizations
    use_kv_cache: bool = False
    window_size: int = 256
    fractal_depth: int = 2
    # Multi-Token Prediction
    mtp_max_k: int = 0
    mtp_share_unembedding: bool = False
    mtp_label_smoothing: float = 0.0


class KVCache:
    """Fast CUDA KV cache with NES-inspired paging, TurboQuant, and chronological order."""

    def __init__(self, max_len: int, n_heads: int, head_dim: int, device: str, hot_window: int = 512):
        self.max_len = max_len
        self.n_heads = n_heads
        self.head_dim = head_dim
        self.device = device
        self.hot_window = hot_window
        
        self.k_hot = torch.zeros(1, n_heads, hot_window, head_dim, device=device)
        self.v_hot = torch.zeros(1, n_heads, hot_window, head_dim, device=device)
        self.hot_tokens = 0
        
        self.k_cold_ang = []
        self.k_cold_mag = []
        self.v_cold_ang = []
        self.v_cold_mag = []

    def append(self, k: torch.Tensor, v: torch.Tensor) -> None:
        seq_len = k.shape[2]
        
        if self.hot_tokens + seq_len > self.hot_window:
            evict_size = min(self.hot_tokens + seq_len - self.hot_window, self.hot_tokens)
            if evict_size > 0:
                k_evict = self.k_hot[:, :, :evict_size]
                v_evict = self.v_hot[:, :, :evict_size]
                
                k_mag = k_evict.norm(dim=-1, keepdim=True)
                v_mag = v_evict.norm(dim=-1, keepdim=True)
                k_ang = ((k_evict / (k_mag + 1e-8) + 1) * 127.5).clamp(0, 255).to(torch.uint8)
                v_ang = ((v_evict / (v_mag + 1e-8) + 1) * 127.5).clamp(0, 255).to(torch.uint8)
                
                self.k_cold_mag.append(k_mag)
                self.k_cold_ang.append(k_ang)
                self.v_cold_mag.append(v_mag)
                self.v_cold_ang.append(v_ang)
                
                self.k_hot = torch.roll(self.k_hot, -evict_size, dims=2)
                self.v_hot = torch.roll(self.v_hot, -evict_size, dims=2)
                self.hot_tokens -= evict_size
                
        self.k_hot[:, :, self.hot_tokens:self.hot_tokens+seq_len] = k
        self.v_hot[:, :, self.hot_tokens:self.hot_tokens+seq_len] = v
        self.hot_tokens += seq_len
        
    def _decompress(self, i: int):
        k_ang = self.k_cold_ang[i].float() / 127.5 - 1.0
        v_ang = self.v_cold_ang[i].float() / 127.5 - 1.0
        return k_ang * self.k_cold_mag[i], v_ang * self.v_cold_mag[i]

    def promote(self, q: torch.Tensor, threshold: float = 0.02, top_k: int = 2) -> tuple[torch.Tensor, torch.Tensor]:
        k_h = self.k_hot[:, :, :self.hot_tokens]
        v_h = self.v_hot[:, :, :self.hot_tokens]
        
        if not self.k_cold_mag:
            return k_h, v_h

        p_k_list, p_v_list = [], []
        # Iterate backwards to prefer recent cold blocks
        for i in range(len(self.k_cold_mag) - 1, -1, -1):
            kc, vc = self._decompress(i)
            score = torch.matmul(q, kc.transpose(-1, -2)).max()
            if score > threshold:
                p_k_list.append((i, kc, vc))
                if len(p_k_list) >= top_k:
                    break
        
        if not p_k_list:
            return k_h, v_h
            
        # Sort chronologically
        p_k_list.sort(key=lambda x: x[0])
        p_k = torch.cat([x[1] for x in p_k_list], dim=2)
        p_v = torch.cat([x[2] for x in p_k_list], dim=2)
        
        return torch.cat([p_k, k_h], dim=2), torch.cat([p_v, v_h], dim=2)
        
    def clear(self) -> None:
        self.hot_tokens = 0
        self.k_hot.zero_()
        self.v_hot.zero_()
        self.k_cold_ang.clear()
        self.k_cold_mag.clear()
        self.v_cold_ang.clear()
        self.v_cold_mag.clear()

    @property
    def total_tokens(self) -> int:
        return self.hot_tokens + sum(m.shape[2] for m in self.k_cold_mag)

# Removed: FractalAttention (dead code, never called in forward pass)


class HybridAttention(nn.Module):
    """Local window attention + GRU state for efficient context modeling.

    Combines O(n·W) local windowed attention with O(n·D) GRU-based
    recurrent state for handling long-range dependencies.
    """

    def __init__(self, config: GPTConfig) -> None:
        super().__init__()
        assert config.n_embd % config.n_head == 0
        self.config = config
        self.n_head = config.n_head
        self.head_dim = config.n_embd // config.n_head
        self.window_size = config.window_size  # 128

        # QKV projection (same as CausalSelfAttention)
        self.qkv = nn.Linear(config.n_embd, 3 * config.n_embd)
        self.proj = nn.Linear(config.n_embd, config.n_embd)
        self.attn_dropout = nn.Dropout(config.dropout)
        self.resid_dropout = nn.Dropout(config.dropout)

        # GRU parameters (per-head, shape (head_dim, head_dim))
        D = self.head_dim
        self.rnn_Wr = nn.Linear(D, D, bias=False)  # reset gate on h
        self.rnn_Ur = nn.Linear(D, D, bias=False)  # reset gate on k
        self.rnn_Wz = nn.Linear(D, D, bias=False)  # update gate on h
        self.rnn_Wn = nn.Linear(D, D, bias=False)  # candidate on h
        self.rnn_Un = nn.Linear(D, D, bias=False)  # candidate on k

        # Blending gate: per-head sigmoid
        self.gate_proj = nn.Linear(config.n_embd, config.n_head, bias=True)
        nn.init.ones_(self.gate_proj.bias)  # initialize to 1 → sigmoid(1) ≈ 0.73

        # Inference state (replaces KVCache)
        self.rnn_state: Optional[torch.Tensor] = None  # (B, H, D)
        self.local_kv_buf_k: Optional[torch.Tensor] = None  # (B, H, W, D) rolling buffer
        self.local_kv_buf_v: Optional[torch.Tensor] = None
        self.local_buf_pos: int = 0  # position within rolling buffer

    def _gated_rnn_step(
        self,
        k: torch.Tensor,  # (B, H, D)
        v: torch.Tensor,  # (B, H, D)
        h_prev: torch.Tensor,  # (B, H, D)
    ) -> torch.Tensor:  # (B, H, D) — new state
        """Single GRU step: h_t = GRU(h_{t-1}, k_t, v_t)."""
        # Reset gate: r_t = sigmoid(Wr @ h_prev + Ur @ k_t)
        r_t = torch.sigmoid(self.rnn_Wr(h_prev) + self.rnn_Ur(k))  # (B, H, D)

        # Update gate: z_t = sigmoid(Wz @ h_prev + k_t)
        z_t = torch.sigmoid(self.rnn_Wz(h_prev) + k)  # (B, H, D)

        # Candidate: n_t = tanh(Wn @ (r_t * h_prev) + Un @ k_t)
        n_t = torch.tanh(self.rnn_Wn(r_t * h_prev) + self.rnn_Un(k))  # (B, H, D)

        # New state: h_t = (1-z_t)*h_prev + z_t*(n_t * v_t)
        h_new = (1.0 - z_t) * h_prev + z_t * (n_t * v)  # (B, H, D)

        return h_new

    def forward(
        self,
        x: torch.Tensor,
        use_cache: bool = False,
    ) -> torch.Tensor:
        B, T, C = x.shape

        # QKV projection
        q, k, v = self.qkv(x).chunk(3, dim=2)  # each (B, T, C)
        q = q.view(B, T, self.n_head, self.head_dim).transpose(1, 2)  # (B, H, T, D)
        k = k.view(B, T, self.n_head, self.head_dim).transpose(1, 2)
        v = v.view(B, T, self.n_head, self.head_dim).transpose(1, 2)

        # --- Local window attention path (O(n·W)) ---
        if T == 1 and use_cache and self.local_kv_buf_k is not None:
            # Autoregressive single-token decode: use rolling buffer
            local_k = self.local_kv_buf_k  # (B, H, W, D)
            local_v = self.local_kv_buf_v  # (B, H, W, D)
            local_out = F.scaled_dot_product_attention(
                q, local_k, local_v,
                dropout_p=self.attn_dropout.p if self.training else 0.0,
                is_causal=False,
            )  # (B, H, 1, D)

            # Update rolling buffer
            self.local_kv_buf_k[:, :, self.local_buf_pos % self.window_size] = k[:, :, 0]
            self.local_kv_buf_v[:, :, self.local_buf_pos % self.window_size] = v[:, :, 0]
            self.local_buf_pos += 1
        else:
            # Training or initial forward: use Triton kernel if available, else Python fallback
            if HAS_TRITON_OPS and T > 1 and q.is_cuda and k.is_cuda and v.is_cuda:
                # Triton kernel (faster, but only for full sequences T > 1)
                local_out, lse = torch.ops.sisyphus.local_window_attention(q, k, v, self.window_size)
            else:
                # Python fallback: apply window mask to full sequence
                # Mask: query at position i can attend to key at j iff j in [i-W+1, i]
                scale = self.head_dim ** -0.5
                scores = torch.matmul(q, k.transpose(-1, -2)) * scale  # (B, H, T, T)

                # Build causal + window mask
                causal_mask = torch.arange(T, device=x.device).unsqueeze(0) >= torch.arange(T, device=x.device).unsqueeze(1)
                window_mask = (torch.arange(T, device=x.device).unsqueeze(0) - torch.arange(T, device=x.device).unsqueeze(1)) < self.window_size
                combined_mask = causal_mask & window_mask  # (T, T)

                # Apply window + causal mask
                scores_masked = scores.clone()
                scores_masked[:, :, ~combined_mask] = float('-inf')

                # Softmax and apply dropout
                attn_weights = F.softmax(scores_masked, dim=-1)
                attn_weights = attn_weights.nan_to_num(0.0)
                attn_weights = self.attn_dropout(attn_weights) if self.training else attn_weights

                # Attention output
                local_out = torch.matmul(attn_weights, v)  # (B, H, T, D)

            # Store initial KV buffer for inference
            if use_cache and self.local_kv_buf_k is None:
                self.local_kv_buf_k = torch.zeros(B, self.n_head, self.window_size, self.head_dim,
                                                  device=x.device, dtype=x.dtype)
                self.local_kv_buf_v = torch.zeros(B, self.n_head, self.window_size, self.head_dim,
                                                  device=x.device, dtype=x.dtype)
                self.local_buf_pos = 0
                # Prefill buffer with current k, v (up to window_size)
                fill_size = min(T, self.window_size)
                self.local_kv_buf_k[:, :, :fill_size] = k[:, :, -fill_size:]
                self.local_kv_buf_v[:, :, :fill_size] = v[:, :, -fill_size:]
                self.local_buf_pos = fill_size

        # --- GRU state path (O(n·D)) ---
        # Initialize or retrieve RNN state
        if use_cache and self.rnn_state is not None:
            h_prev = self.rnn_state  # (B, H, D)
        else:
            h_prev = torch.zeros(B, self.n_head, self.head_dim, device=x.device, dtype=x.dtype)

        # Process each token through GRU
        rnn_outputs = []
        for t in range(T):
            k_t = k[:, :, t]  # (B, H, D)
            v_t = v[:, :, t]  # (B, H, D)
            h_t = self._gated_rnn_step(k_t, v_t, h_prev)  # (B, H, D)
            rnn_outputs.append(h_t.unsqueeze(2))  # (B, H, 1, D)
            h_prev = h_t

        rnn_out = torch.cat(rnn_outputs, dim=2)  # (B, H, T, D)

        # Save state for next call in use_cache mode
        if use_cache:
            self.rnn_state = h_prev.detach()  # (B, H, D)

        # --- Blending gate ---
        alpha = torch.sigmoid(self.gate_proj(x))  # (B, T, n_head)
        alpha = alpha.permute(0, 2, 1).unsqueeze(-1)  # (B, H, T, 1)

        # Blend local and RNN outputs
        y = alpha * local_out + (1.0 - alpha) * rnn_out  # (B, H, T, D)

        # Output projection
        y = y.transpose(1, 2).contiguous().view(B, T, C)  # (B, T, C)
        return self.resid_dropout(self.proj(y))

    def clear_state(self) -> None:
        """Clear inference state (call at start of generate())."""
        self.rnn_state = None
        self.local_kv_buf_k = None
        self.local_kv_buf_v = None
        self.local_buf_pos = 0


class CausalSelfAttention(nn.Module):
    def __init__(self, config: GPTConfig) -> None:
        super().__init__()
        assert config.n_embd % config.n_head == 0
        self.config = config
        self.n_head = config.n_head
        self.head_dim = config.n_embd // config.n_head
        self.qkv = nn.Linear(config.n_embd, 3 * config.n_embd)
        self.proj = nn.Linear(config.n_embd, config.n_embd)
        self.attn_dropout = nn.Dropout(config.dropout)
        self.resid_dropout = nn.Dropout(config.dropout)

        # KV cache will be initialized on first use with correct device
        self.kv_cache: Optional[KVCache] = None

    def forward(
        self,
        x: torch.Tensor,
        use_cache: bool = False,
    ) -> torch.Tensor:
        batch, steps, channels = x.shape
        q, k, v = self.qkv(x).chunk(3, dim=2)
        q = q.view(batch, steps, self.n_head, self.head_dim).transpose(1, 2)
        k = k.view(batch, steps, self.n_head, self.head_dim).transpose(1, 2)
        v = v.view(batch, steps, self.n_head, self.head_dim).transpose(1, 2)

        # Update KV cache if enabled
        if use_cache and self.kv_cache is not None:
            self.kv_cache.append(k, v)
            # NES-inspired promotion
            k, v = self.kv_cache.promote(q, threshold=0.02)

            y = F.scaled_dot_product_attention(
                q, k, v,
                dropout_p=self.attn_dropout.p if self.training else 0.0,
                is_causal=(steps > 1) # Mask handles it
            )
        else:
            # Standard training or non-cached inference
            y = F.scaled_dot_product_attention(
                q, k, v,
                dropout_p=self.attn_dropout.p if self.training else 0.0,
                is_causal=True
            )

        y = y.transpose(1, 2).contiguous().view(batch, steps, channels)
        return self.resid_dropout(self.proj(y))


class MLP(nn.Module):
    def __init__(self, config: GPTConfig) -> None:
        super().__init__()
        hidden = 4 * config.n_embd
        self.net = nn.Sequential(
            nn.Linear(config.n_embd, hidden),
            nn.GELU(),
            nn.Linear(hidden, config.n_embd),
            nn.Dropout(config.dropout),
        )

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        return self.net(x)


class Block(nn.Module):
    def __init__(self, config: GPTConfig) -> None:
        super().__init__()
        self.ln_1 = nn.LayerNorm(config.n_embd)
        self.attn = HybridAttention(config)  # Hybrid local + RNN attention
        self.ln_2 = nn.LayerNorm(config.n_embd)
        self.mlp = MLP(config)

    def forward(self, x: torch.Tensor, use_cache: bool = False) -> torch.Tensor:
        x = x + self.attn(self.ln_1(x), use_cache=use_cache)
        x = x + self.mlp(self.ln_2(x))
        return x


class MTPHead(nn.Module):
    """Extra prediction head for Multi-Token Prediction at offset k."""

    def __init__(self, n_embd: int, vocab_size: int, shared_weight: torch.Tensor | None = None) -> None:
        super().__init__()
        self.proj = nn.Linear(n_embd, n_embd, bias=False)
        self.ln = nn.LayerNorm(n_embd)
        self.out = nn.Linear(n_embd, vocab_size, bias=False)
        if shared_weight is not None:
            self.out.weight = shared_weight

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        return self.out(self.ln(self.proj(x)))


class ByteGPT(nn.Module):
    def __init__(self, config: GPTConfig) -> None:
        super().__init__()
        self.config = config
        self.token_embedding = nn.Embedding(config.vocab_size, config.n_embd)
        self.position_embedding = nn.Embedding(config.block_size, config.n_embd)
        self.dropout = nn.Dropout(config.dropout)
        self.blocks = nn.ModuleList(Block(config) for _ in range(config.n_layer))
        self.ln_f = nn.LayerNorm(config.n_embd)
        self.lm_head = nn.Linear(config.n_embd, config.vocab_size, bias=False)
        self.token_embedding.weight = self.lm_head.weight

        # Cache position indices to avoid reallocation every forward
        self.register_buffer('_position_ids', torch.arange(config.block_size))

        self.apply(self._init_weights)

        # Multi-Token Prediction heads
        self.mtp_heads = nn.ModuleList()
        if config.mtp_max_k > 0:
            for _ in range(config.mtp_max_k):
                shared_w = self.lm_head.weight if config.mtp_share_unembedding else None
                self.mtp_heads.append(MTPHead(config.n_embd, config.vocab_size, shared_weight=shared_w))

    def _init_weights(self, module: nn.Module) -> None:
        if isinstance(module, nn.Linear):
            nn.init.normal_(module.weight, mean=0.0, std=0.02)
            if module.bias is not None:
                nn.init.zeros_(module.bias)
        elif isinstance(module, nn.Embedding):
            nn.init.normal_(module.weight, mean=0.0, std=0.02)

    def forward(
        self, idx: torch.Tensor, targets: torch.Tensor | None = None, use_cache: bool = False
    ) -> tuple[torch.Tensor, torch.Tensor | None]:
        batch, steps = idx.shape
        
        if use_cache and hasattr(self.blocks[0].attn, 'local_buf_pos'):
            # HybridAttention: position tracking via local_buf_pos (rolling window position)
            # For longer sequences, just use the sequence length
            positions = self._position_ids[:steps]
        else:
            if steps > self.config.block_size:
                raise ValueError("sequence length exceeds block size")
            # Use cached position IDs instead of allocating new tensor
            positions = self._position_ids[:steps]

        x = self.token_embedding(idx) + self.position_embedding(positions)
        x = self.dropout(x)
        for block in self.blocks:
            x = block(x, use_cache=use_cache)
        x = self.ln_f(x)
        logits = self.lm_head(x)

        loss = None
        if targets is not None:
            ntp_loss = F.cross_entropy(
                logits.view(batch * steps, self.config.vocab_size),
                targets.view(batch * steps),
            )
            loss = ntp_loss
            head_losses = {"ntp": ntp_loss.detach().item()}
            head_entropies = {}

            # Compute NTP entropy for collapse detection
            ntp_probs = F.softmax(logits, dim=-1)
            ntp_entropy = -(ntp_probs * torch.log(ntp_probs + 1e-10)).sum(dim=-1).mean()
            head_entropies["ntp"] = ntp_entropy.detach().item()

            # Multi-Token Prediction: compute losses and entropies for extra heads
            active_k: int = getattr(self, '_active_k', 0)
            if active_k > 0 and self.mtp_heads:
                mtp_weight: float = getattr(self, '_mtp_head_weight', 0.3)
                for i, head in enumerate(self.mtp_heads[:active_k]):
                    offset = i + 1
                    if steps - offset < 1:
                        break
                    head_logits = head(x[:, :steps - offset])  # (B, T-offset, V)
                    head_targets = targets[:, offset:]           # (B, T-offset)
                    head_loss = F.cross_entropy(
                        head_logits.reshape(-1, self.config.vocab_size),
                        head_targets.reshape(-1),
                        label_smoothing=self.config.mtp_label_smoothing,
                    )
                    loss = loss + mtp_weight * head_loss
                    head_losses[f"mtp_{offset}"] = head_loss.detach().item()

                    # Compute entropy for this head
                    head_probs = F.softmax(head_logits, dim=-1)
                    head_entropy = -(head_probs * torch.log(head_probs + 1e-10)).sum(dim=-1).mean()
                    head_entropies[f"mtp_{offset}"] = head_entropy.detach().item()

            # Expose per-head losses and entropies for logging
            self._last_head_losses = head_losses
            self._last_head_entropies = head_entropies

        # Cache hidden states for speculative decoding
        if use_cache:
            self._last_hidden = x.detach()

        return logits, loss

    @torch.no_grad()
    def generate(
        self,
        idx: torch.Tensor,
        max_new_tokens: int,
        temperature: float = 1.0,
        top_k: int | None = None,
        use_cache: bool = True,
        speculative_k: int = 0,
    ) -> torch.Tensor:
        # Clear HybridAttention state (RNN + local KV buffer) if requested
        if use_cache:
            for block in self.blocks:
                if hasattr(block.attn, 'clear_state'):
                    block.attn.clear_state()

            # Prefill: process entire prompt in one forward pass
            # This populates rnn_state in each HybridAttention layer
            self(idx, use_cache=True)

        for _ in range(max_new_tokens):
            if use_cache:
                # In cache mode, we only pass the LAST token to the model
                idx_cond = idx[:, -1:]
                logits, _ = self(idx_cond, use_cache=True)
            else:
                idx_cond = idx[:, -self.config.block_size :]
                logits, _ = self(idx_cond)

            # Speculative decoding: use MTP heads as draft models
            if speculative_k > 0 and len(self.mtp_heads) >= speculative_k and hasattr(self, '_last_hidden'):
                x_last = self._last_hidden[:, -1:]  # (B, 1, C) — last token hidden state

                # 1. Draft speculative_k tokens with probabilities
                draft_tokens = []
                draft_probs = []
                for head in self.mtp_heads[:speculative_k]:
                    d_logits = head(x_last)[:, -1, :] / max(temperature, 1e-5)
                    d_logits = d_logits.clamp(max=20)  # Prevent overflow in softmax
                    if top_k is not None:
                        vals, _ = torch.topk(d_logits, min(top_k, d_logits.size(-1)))
                        # Create mask instead of setting to -inf
                        mask = d_logits < vals[:, [-1]]
                        d_logits = d_logits.masked_fill(mask, float('-inf'))
                    d_probs = F.softmax(d_logits, dim=-1)
                    # Ensure no NaN from softmax of all -inf
                    d_probs = torch.where(d_probs.isnan(), torch.ones_like(d_probs) / d_logits.size(-1), d_probs)
                    d_tok = torch.multinomial(d_probs, 1)
                    draft_tokens.append(d_tok)
                    draft_probs.append(d_probs.gather(1, d_tok).clamp(min=1e-10))

                # 2. Main model: sample main token from current logits
                main_logits = logits[:, -1, :] / max(temperature, 1e-5)
                main_logits = main_logits.clamp(max=20)
                if top_k is not None:
                    vals, _ = torch.topk(main_logits, min(top_k, main_logits.size(-1)))
                    mask = main_logits < vals[:, [-1]]
                    main_logits = main_logits.masked_fill(mask, float('-inf'))
                main_probs = F.softmax(main_logits, dim=-1)
                main_probs = torch.where(main_probs.isnan(), torch.ones_like(main_probs) / main_logits.size(-1), main_probs)
                main_tok = torch.multinomial(main_probs, 1)

                # 3. Verify drafts: run one forward pass over main_tok + draft tokens
                verify_input = torch.cat([idx[:, -1:], main_tok] + draft_tokens, dim=1)
                verify_logits, _ = self(verify_input, use_cache=False)

                # 4. Accept/reject drafts via speculative sampling
                accepted = [main_tok]
                self._last_n_accepted = 1
                for j, (d_tok, d_prob) in enumerate(zip(draft_tokens, draft_probs)):
                    v_logits = verify_logits[:, j + 1, :] / max(temperature, 1e-5)
                    v_logits = v_logits.clamp(max=20)
                    if top_k is not None:
                        vals, _ = torch.topk(v_logits, min(top_k, v_logits.size(-1)))
                        mask = v_logits < vals[:, [-1]]
                        v_logits = v_logits.masked_fill(mask, float('-inf'))
                    v_probs = F.softmax(v_logits, dim=-1)
                    v_probs = torch.where(v_probs.isnan(), torch.ones_like(v_probs) / v_logits.size(-1), v_probs)
                    v_prob_at_tok = v_probs.gather(1, d_tok).clamp(min=1e-10)
                    acceptance = (v_prob_at_tok / d_prob).clamp(max=1.0)
                    u = torch.rand_like(acceptance)
                    if (u < acceptance).all():
                        accepted.append(d_tok)
                        self._last_n_accepted += 1
                    else:
                        # Reject: sample from adjusted distribution (verify - draft).clamp(0)
                        adjusted = (v_probs - d_prob.expand_as(v_probs)).clamp(min=0)
                        adj_sum = adjusted.sum(dim=-1, keepdim=True)
                        # If adjusted distribution is all zeros, use uniform distribution
                        adjusted = torch.where(
                            adj_sum > 1e-10,
                            adjusted / (adj_sum + 1e-10),
                            torch.ones_like(adjusted) / adjusted.size(-1)
                        )
                        accepted.append(torch.multinomial(adjusted, 1))
                        break  # Stop accepting after first rejection

                idx = torch.cat([idx] + accepted, dim=1)
            else:
                # Standard autoregressive decoding
                logits = logits[:, -1, :] / max(temperature, 1e-5)
                if top_k is not None:
                    values, _ = torch.topk(logits, min(top_k, logits.size(-1)))
                    logits[logits < values[:, [-1]]] = float("-inf")
                probs = F.softmax(logits, dim=-1)
                next_token = torch.multinomial(probs, num_samples=1)
                idx = torch.cat((idx, next_token), dim=1)
        return idx
