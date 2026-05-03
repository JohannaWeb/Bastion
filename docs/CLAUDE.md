# Sisyphus

ML project for training a language model from scratch on Rust web documentation.

## Project Structure

- `src/` — Python: `train.py`, `model.py`, `generate.py`, corpus builders, benchmarks
- `tests/` — Test suite
- `docs/` — Design docs, analysis, and reports
- `configs/` — Variant training configs (`config.20m.yaml`, `config.50m.yaml`, etc.)
- `scripts/` — Utility shell/Python scripts (`monitor_training.sh`, `fix.py`)
- `logs/` — Training logs
- `data/` — Training data (Rust web book)
- `checkpoints/` — Model checkpoints
- `config.yaml` — Default training configuration
- `train_from_scratch.sh` — Main training pipeline script

## Key Commands

### Full pipeline (wrapper, skips optional FineWeb step):
```bash
bash train_from_scratch.sh
```

### Individual steps (if you need more control):
```bash
# Fetch Rust documentation repos
python3 src/fetch_rust_web_corpus.py --config config.yaml

# Fetch Rust code repositories
python3 src/fetch_rust_code_corpus.py --config config.yaml

# Optional: Fetch FineWeb-Edu corpus
python3 src/fetch_fineweb_edu.py --config config.yaml

# Build combined corpus
python3 src/build_corpus.py --config config.yaml

# Train from scratch
python3 src/train.py --config config.yaml

# Resume training from last checkpoint
python3 src/train.py --config config.yaml --resume checkpoints/sisyphus.last.pt

# Generate text
python3 src/generate.py --checkpoint checkpoints/sisyphus.pt --prompt "fn main"
```

## Notes

- Uses PyTorch for the transformer model
- Corpus built from Rust web documentation and code repositories
- Byte-level tokenization (256 vocab, no separate tokenizer needed)
- Small model by design (2M parameters, fits in 4GB VRAM)
- Training saves checkpoints: `sisyphus.pt` (best) and `sisyphus.last.pt` (most recent)