#!/usr/bin/env python3
"""Benchmark speculative decoding: measure acceptance rates and theoretical speedup."""

from __future__ import annotations

import argparse
from pathlib import Path

import torch

from model import ByteGPT, GPTConfig


def theoretical_speedup(k: int, acceptance_rate: float) -> float:
    """Compute theoretical speedup given k and acceptance rate.

    E[tokens_per_step] = 1 + k * alpha
    Speedup = E[tokens_per_step] = 1 + k * alpha
    """
    return 1.0 + k * acceptance_rate


def forward_passes_per_token(k: int, acceptance_rate: float) -> float:
    """Compute forward passes per token generated.

    Given acceptance rate alpha and k drafts:
    - We do 1 main forward pass + 1 verification forward pass per step
    - Expected tokens per step = 1 + k * alpha
    - Forward passes per token = 2 / (1 + k * alpha)
    """
    if k == 0:
        return 1.0
    expected_tokens = 1.0 + k * acceptance_rate
    return 2.0 / expected_tokens


def benchmark_k(
    model: ByteGPT,
    prompts: list[torch.Tensor],
    k: int,
    n_tokens: int,
    temperature: float = 0.9,
    top_k: int = 50,
) -> dict:
    """Benchmark speculative decoding at a specific k value.

    Returns:
        dict with keys: accept_total, draft_total, tokens_generated, acceptance_rate
    """
    model.eval()

    accept_total = 0
    draft_total = 0
    tokens_generated = 0

    with torch.no_grad():
        for prompt in prompts:
            batch = prompt.unsqueeze(0)  # (1, T)

            # Generate n_tokens with speculative_k=k
            output = model.generate(
                batch,
                max_new_tokens=n_tokens,
                temperature=temperature,
                top_k=top_k,
                use_cache=True,
                speculative_k=k,
            )

            # Track acceptance stats from _last_n_accepted
            if hasattr(model, '_last_n_accepted'):
                accept_total += model._last_n_accepted
                draft_total += k * n_tokens  # k drafts attempted per token
                tokens_generated += n_tokens

    tokens_generated = max(tokens_generated, 1)
    acceptance_rate = accept_total / draft_total if draft_total > 0 else 0.0

    return {
        'accept_total': accept_total,
        'draft_total': draft_total,
        'tokens_generated': tokens_generated,
        'acceptance_rate': acceptance_rate,
    }


def main() -> None:
    parser = argparse.ArgumentParser(
        description='Benchmark speculative decoding acceptance rates and theoretical speedup'
    )
    parser.add_argument(
        '--checkpoint',
        type=Path,
        default=None,
        help='Path to model checkpoint (if not provided, uses random init)',
    )
    parser.add_argument(
        '--k-values',
        type=int,
        nargs='+',
        default=[1, 2, 3],
        help='k values to benchmark (default: 1 2 3)',
    )
    parser.add_argument(
        '--n-tokens',
        type=int,
        default=100,
        help='Number of tokens to generate per sample (default: 100)',
    )
    parser.add_argument(
        '--n-samples',
        type=int,
        default=10,
        help='Number of prompts to sample (default: 10)',
    )
    parser.add_argument(
        '--prompt-length',
        type=int,
        default=64,
        help='Length of random prompts (default: 64)',
    )
    parser.add_argument(
        '--temperature',
        type=float,
        default=0.9,
        help='Sampling temperature (default: 0.9)',
    )
    parser.add_argument(
        '--top-k',
        type=int,
        default=50,
        help='Top-k sampling (default: 50)',
    )
    parser.add_argument(
        '--device',
        type=str,
        default='cuda' if torch.cuda.is_available() else 'cpu',
        help='Device to run on (default: cuda if available, else cpu)',
    )
    args = parser.parse_args()

    # Load or initialize model
    print('Loading model...', flush=True)
    if args.checkpoint and args.checkpoint.exists():
        checkpoint = torch.load(args.checkpoint, map_location=args.device)
        config_dict = checkpoint['config']
        # Filter to only valid GPTConfig fields
        valid_fields = {
            'vocab_size', 'block_size', 'n_layer', 'n_head', 'n_embd', 'dropout',
            'use_kv_cache', 'window_size', 'fractal_depth',
            'mtp_max_k', 'mtp_share_unembedding', 'mtp_label_smoothing'
        }
        filtered_config = {k: v for k, v in config_dict.items() if k in valid_fields}
        config = GPTConfig(**filtered_config)
        model = ByteGPT(config)
        # Handle both checkpoint formats
        state_dict_key = 'model_state_dict' if 'model_state_dict' in checkpoint else 'model'
        model.load_state_dict(checkpoint[state_dict_key])
        print(f'Loaded checkpoint: {args.checkpoint}')
    else:
        config = GPTConfig(mtp_max_k=max(args.k_values) if args.k_values else 3)
        model = ByteGPT(config)
        print(f'Using randomly initialized model with mtp_max_k={config.mtp_max_k}')

    model.to(args.device)
    model.eval()

    # Generate random prompts
    print(f'Generating {args.n_samples} random prompts (length {args.prompt_length})...', flush=True)
    prompts = [
        torch.randint(0, config.vocab_size, (args.prompt_length,), device=args.device)
        for _ in range(args.n_samples)
    ]

    # Benchmark each k value
    print(f'\nBenchmarking speculative decoding (k={args.k_values})')
    print(f'Prompt length: {args.prompt_length} tokens | Eval tokens: {args.n_tokens} per sample | Samples: {args.n_samples}')
    print('━' * 80)
    print(f'{"k":>3} | {"accept%":>8} | {"speedup":>9} | {"fwd/tok":>8} | {"drafts":>8} | {"accepted":>8}')
    print('━' * 80)

    results_by_k = {}

    # Baseline: k=0 (standard autoregressive)
    result = benchmark_k(model, prompts, k=0, n_tokens=args.n_tokens,
                        temperature=args.temperature, top_k=args.top_k)
    results_by_k[0] = result
    speedup = 1.0
    fwd_per_tok = 1.0
    print(f'{0:3d} | {"—":>8} | {speedup:>8.2f}x | {fwd_per_tok:>8.2f} | {"—":>8} | {"—":>8}')

    # Benchmark each k value
    max_speedup = 1.0
    best_k = 0
    for k in sorted(args.k_values):
        result = benchmark_k(model, prompts, k=k, n_tokens=args.n_tokens,
                            temperature=args.temperature, top_k=args.top_k)
        results_by_k[k] = result

        accept_pct = result['acceptance_rate'] * 100
        speedup = theoretical_speedup(k, result['acceptance_rate'])
        fwd_per_tok = forward_passes_per_token(k, result['acceptance_rate'])

        if speedup > max_speedup:
            max_speedup = speedup
            best_k = k

        speedup_str = f'{speedup:.2f}x'
        if speedup == max_speedup and k > 0:
            speedup_str += ' ★'

        print(
            f'{k:3d} | {accept_pct:>7.1f}% | {speedup_str:>9} | {fwd_per_tok:>8.2f} | '
            f'{result["draft_total"]:>8d} | {result["accept_total"]:>8d}'
        )

    print('━' * 80)
    print(f'\nPeak speedup: {max_speedup:.2f}x at k={best_k}')
    print(f'Note: Speedup = 1 + k * acceptance_rate (assumes 2 FWD passes per step)')
    print(f'      fwd/tok = 2 / (1 + k * alpha)')


if __name__ == '__main__':
    main()
