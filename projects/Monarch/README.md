# Monarch

Expert LLM fine-tuned specifically for Project Falcon / Juntos — the LGBT+ decentralized community platform.

## Overview

Monarch is a specialized language model trained on Project Falcon's:
- **Philosophy & Values** - sovereignty, transparency, decentralization
- **Architecture** - AT Protocol, DIDs, decentralized identity
- **Domain Models** - servers, channels, members, messages
- **Research** - distributed AI coordination, trust systems

Built with:
- **PyTorch** - for model training
- **LoRA** (Low-Rank Adaptation) - efficient fine-tuning
- **Transformers** - HuggingFace stack
- **ProjectFalcon** - training data source

## Quick Start

### 1. Install Dependencies
```bash
pip install -r requirements.txt
```

### 2. Train Monarch
Full pipeline (extract data → prepare → train):
```bash
bash train_monarch.sh
```

Or run each step individually:

**Extract training data:**
```bash
python src/data_extractor.py
```

**Prepare dataset:**
```bash
python src/dataset.py
```

**Train with LoRA:**
```bash
python src/train.py \
  --base-model TinyLlama/TinyLlama-1.1B-Chat-v1.0 \
  --data data/processed/texts.txt \
  --epochs 3 \
  --batch-size 4
```

### 3. Run Inference

**Interactive chat:**
```bash
python src/inference.py --model models/monarch_lora
```

**Single prompt:**
```bash
python src/inference.py \
  --model models/monarch_lora \
  --prompt "What is Juntos?"
```

## Architecture

```
Monarch/
├── src/
│   ├── data_extractor.py    # Extract from ProjectFalcon
│   ├── dataset.py           # Prepare training data
│   ├── train.py             # Training with LoRA
│   └── inference.py         # Run the model
├── data/
│   ├── raw/                 # Extracted from ProjectFalcon
│   │   ├── documentation.jsonl
│   │   ├── code_patterns.jsonl
│   │   ├── conversations.txt
│   │   └── metadata.json
│   └── processed/           # Prepared for training
│       ├── instructions.jsonl
│       ├── texts.txt
│       └── dataset_metadata.json
├── models/
│   └── monarch_lora/        # Trained LoRA weights
├── config.yaml              # Configuration
├── requirements.txt         # Dependencies
└── README.md                # This file
```

## Training Details

### LoRA Configuration
- **Rank**: 8 (memory-efficient)
- **Alpha**: 16
- **Dropout**: 0.05
- **Target Modules**: q_proj, v_proj

### Model Options
- **Default (fast)**: TinyLlama-1.1B (minimal resources)
- **Production**: Llama-2-7B (better quality, more VRAM needed)

### Training Hyperparameters
- **Epochs**: 3
- **Batch Size**: 4
- **Learning Rate**: 2e-4
- **Gradient Accumulation**: 4
- **Optimizer**: AdamW (8-bit with CUDA)

## Personality

Monarch embodies the Juntos ethos:

**Direct and precise.** No fluff, no filler.
**Technically rigorous** but never condescending.
**Neutral on content,** principled on safety.
**Confident in analysis,** honest about uncertainty.

## Values

- Sovereignty is non-negotiable. No platform owns identity, data, or intelligence.
- Trust is computed, not assigned.
- Transparency over black boxes — every decision is auditable.
- Decentralization is a value, not a feature.
- The protocol outlasts the product.

## Device Support

Automatically detects and uses:
- **CUDA** if available (faster training, GPU memory optimizations)
- **CPU** fallback (slower but works)

For M1/M2 Mac: Install `pytorch` with Metal support for acceleration.

## Hardware Requirements

### Minimum (TinyLlama-1.1B)
- CPU: 4+ cores
- RAM: 16GB
- Storage: 2GB

### Recommended (Llama-2-7B)
- GPU: RTX 3060+ (12GB VRAM)
- CPU: 8+ cores
- RAM: 32GB
- Storage: 20GB

## Customization

Edit `config.yaml` to adjust:
- Base model
- LoRA parameters
- Training hyperparameters
- Data paths

## Integration with ProjectFalcon

Monarch can be integrated with Juntos's AI SIV (Sovereign Integration Vessel):
- Drop `monarch_lora/` into the Juntos model directory
- Update AiContextService to use Monarch
- All responses are signed and auditable

## Development

To extend Monarch:
1. Add more training data to `data/raw/`
2. Modify data extraction in `src/data_extractor.py`
3. Adjust instruction templates in `src/dataset.py`
4. Retrain with `train.py`

## License

MIT (same as ProjectFalcon)

---

**Built by Johanna for Project Falcon**
