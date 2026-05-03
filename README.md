# Sisyphus

Sisyphus is a small Rust-focused language model trained from scratch in PyTorch.

It builds a corpus from local project text, official Rust docs, Rust code repositories, and the top 500 crates from crates.io. It then trains a byte-level decoder with a hybrid local-attention + recurrent architecture, and writes checkpoints for generation and evaluation.

This repo is for training experiments, not product polish. The code in `src/` and the concrete configs in `configs/` are the things to trust first.

## What this is trying to do

- keep the stack small enough to run on ordinary hardware (a 4060 Ti is fine)
- stay close to the data and training code rather than hiding everything behind tooling
- see how far a small Rust-specific model can go with a large, deduplicated corpus and a locality-biased attention block

This is not LoRA, not instruction tuning, and not base-model finetuning. The model is trained from random initialization on raw bytes.

## Repository layout

```
src/          Python source — model, training loop, corpus builders, benchmarks
tests/        Test suite
configs/      Variant training configs (20m, 20m.optimized, 50m)
docs/         Design docs, analysis, reports, experiment notes
scripts/      Utility scripts (monitor_training.sh, fix.py)
logs/         Training logs
data/         Corpus and raw fetched data
checkpoints/  Saved model checkpoints
config.yaml   Default training configuration
```

## Model

`src/model.py` defines `ByteGPT`, a byte-level decoder-only transformer:

- vocab size `256` (raw UTF-8 bytes, no tokenizer needed)
- learned absolute positional embeddings
- tied token embedding / LM head weights
- residual blocks with pre-norm MLPs
- `HybridAttention` in every block

Default size (6 layers, 6 heads, 384 embed dim): **~4.3M parameters**. The 20M configs use 12 layers, 8 heads, 512 embed dim: **~25.6M parameters**.

### HybridAttention

Each block uses two orthogonal paths instead of full causal attention:

**Local window attention** — causal attention within a sliding window of 256 tokens. Handles local code structure efficiently. Uses `scaled_dot_product_attention` with window masking; falls back gracefully to Python when the optional Triton kernel isn't available.

**GRU recurrent path** — a simplified gated recurrent unit over K and V, carrying compressed long-range state without paying for full dense attention. Per-head reset, update, and candidate gates; state is maintained across tokens during inference.

A learned per-token sigmoid gate blends the two outputs:

```
alpha = sigmoid(gate_proj(x))
output = alpha * local_out + (1 - alpha) * rnn_out
```

**Measured result**: 51.47x inference speedup vs standard full attention (286.6 tok/s with cache vs 5.6 tok/s without), with no quality degradation. See `docs/TEST_REPORT_HYBRID_ATTENTION.md`.

### Multi-token prediction (optional)

`config.yaml` has an `mtp:` section that enables auxiliary heads for speculative decoding. Disabled by default. When enabled, the model trains up to `max_k` prediction heads with a curriculum schedule and uses them at generation time to draft tokens that the main model then verifies. If you don't need faster generation, leave this off.

## Corpus

The corpus is built from Rust-specific sources. In rough order of volume:

| Source | Cap |
|---|---|
| Top 500 crates (crates.io) | 150M chars |
| The Stack — Rust subset | 50M chars |
| Awesome-Rust repositories | 50M chars |
| Rust stdlib + compiler + tests | ~18M chars |
| rust-analyzer | 2.5M chars |
| tokio ecosystem | 1.8M chars |
| cargo source and tests | 2.3M chars |
| serde, ripgrep, clap, axum | ~4M chars combined |
| Official Rust books and docs | ~4M chars combined |
| FineWeb-Edu (optional) | 6M chars |

Total: roughly 300M characters of Rust source before deduplication. The top-crates expansion made the biggest quality difference of any single corpus change.

The corpus builder filters by extension (`.rs`, `.md`, `.toml`, `.txt`, `.rst`), caps file size at 256KB and 20k chars per file, normalizes whitespace, and deduplicates by SHA256 of normalized content. Output is a single `data/processed/corpus.txt` with `<FILE source="..." path="...">` wrappers.

The most recent logged run used **177M bytes** after dedup, split 90/10 train/val.

## Training

Full pipeline:

```bash
bash train_from_scratch.sh
```

Step by step:

```bash
# Fetch Rust documentation repos
python3 src/fetch_rust_web_corpus.py --config config.yaml

# Fetch Rust code repositories
python3 src/fetch_rust_code_corpus.py --config config.yaml

# Fetch top 500 crates from crates.io
python3 src/fetch_top_crates.py --count 500

# Optional: FineWeb-Edu general text
python3 src/fetch_fineweb_edu.py --config config.yaml

# Build combined corpus
python3 src/build_corpus.py --config config.yaml

# Train from scratch
python3 src/train.py --config config.yaml

# Resume from last checkpoint
python3 src/train.py --config config.yaml --resume checkpoints/sisyphus.last.pt
```

`train_from_scratch.sh` skips FineWeb-Edu. Run the fetcher manually first if you want it in the corpus.

Variant configs are in `configs/`. The most detailed documented run is `configs/config.20m.optimized.yaml` with its log at `logs/train.20m.optimized.log`.

## A concrete run

The best-logged run used `configs/config.20m.optimized.yaml`:

- **25,613,312 parameters**
- block size 512, batch size 12, lr 2e-4
- 30,000 steps, mixed precision on CUDA
- train loss `0.5834`, val loss `0.8217`
- best val loss `0.7757` at step 18,500
- ~40 hours on a 4060 Ti

## Generation

```bash
python3 src/generate.py \
  --checkpoint checkpoints/sisyphus.pt \
  --prompt "fn main"
```

No tokenizer step. The prompt is UTF-8 encoded directly to byte IDs.

## Evaluation

```bash
python3 src/eval.py \
  --checkpoint checkpoints/sisyphus.pt \
  --config config.yaml \
  --split val
```

Perplexity tracks training well enough. Syntax validity and repetition rate are the obvious missing evals.

## Hardware

CUDA is the intended path. CPU works in principle and is painfully slow in practice. A 4060 Ti runs the 20M config at roughly 200 tokens/sec.

The trainer does a memory preflight before the first step and has guardrails (`max_vram_utilization`, `min_free_vram_bytes`, `max_batch_tokens`, etc.) to fail fast instead of dying mid-run.

## Notes

- If a doc and the code disagree, trust the code and the logs.
- Some older code (ActivationCompressor, FractalAttention, SelectiveBackprop) is still present but unused in the main training path. Read `ByteGPT`, `Block`, and `HybridAttention` first.
- `docs/` contains experiment notes, optimization analysis, and test reports from previous iterations. Useful for context, not authoritative on current behavior.
