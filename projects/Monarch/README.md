# Monarch

Monarch is a local model training and inference project for Bastion-specific language behavior.

At the moment, this repository is best understood as a practical LoRA fine-tuning pipeline plus a small UI layer, not as a finished model platform.

## What Is Here

- `src/data_extractor.py`: extracts source material from another project
- `src/dataset.py`: prepares training text and instruction data
- `src/train.py`: fine-tuning entrypoint
- `src/inference.py`: local inference entrypoint
- `train_monarch.sh`: convenience wrapper for the pipeline
- `config.yaml`: model, LoRA, and training configuration
- `ui/`: a separate lightweight UI/runtime layer

## Current Scope

The current implementation is mostly:

- extracting local project text into training data
- preparing a small instruction-style dataset
- LoRA fine-tuning against a base model such as TinyLlama
- running local inference against the produced adapter

There are bigger integration ideas around it, but those should be read as direction rather than current product surface.

## Quick Start

Install dependencies:

```bash
pip install -r requirements.txt
```

Run the pipeline:

```bash
bash train_monarch.sh
```

Train manually:

```bash
python src/train.py \
  --base-model TinyLlama/TinyLlama-1.1B-Chat-v1.0 \
  --data data/processed/texts.txt \
  --epochs 3 \
  --batch-size 4
```

Run inference:

```bash
python src/inference.py --model models/monarch_lora
```

## Configuration

The default configuration in `config.yaml` currently targets:

- TinyLlama 1.1B as the base model
- LoRA rank 8 / alpha 16 / dropout 0.05
- 3 epochs with batch size 4

The configured source project path is local to the original development machine, so you may need to update it before running extraction.

## Notes

- Hardware requirements depend heavily on the chosen base model.
- The checked-in docs discuss stronger integration ideas than the current scripts implement.
- `QUICKSTART.md` and `ui/README.md` contain more operational detail if you are working directly on this project.

## License

MIT
