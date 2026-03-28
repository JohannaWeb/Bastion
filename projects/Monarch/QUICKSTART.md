# Monarch - Quick Start Guide

## Current Status ✅

✓ Directory structure created
✓ Data extraction pipeline complete (659 lines extracted)
✓ Training dataset prepared (34 instruction-response pairs, 826 lines)
✓ LoRA training script ready
✓ Inference pipeline configured
✓ All dependencies documented

## Next Steps

### Option 1: Full Automated Training
```bash
cd /home/johanna/projects/Monarch
bash train_monarch.sh
```

This runs:
1. Data extraction from ProjectFalcon ✅ (already done)
2. Dataset preparation ✅ (already done)
3. Model training with LoRA (coming next)

### Option 2: Manual Step-by-Step

**Install dependencies first:**
```bash
pip install -r requirements.txt
```

**Train the model:**
```bash
python3 src/train.py \
  --base-model TinyLlama/TinyLlama-1.1B-Chat-v1.0 \
  --data data/processed/texts.txt \
  --epochs 3 \
  --batch-size 4
```

**Run inference:**
```bash
# Interactive mode
python3 src/inference.py --model models/monarch_lora

# Single prompt
python3 src/inference.py \
  --model models/monarch_lora \
  --prompt "What is Juntos?"
```

## Data Used for Training

### From ProjectFalcon:
- ✓ 11 documentation files (README, ARCHITECTURE, RESEARCH_NOTES, etc.)
- ✓ 4 domain models (Message, Channel, Server, Member)
- ✓ 9 AT Protocol lexicons
- ✓ 15 synthetic Q&A conversations
- ✓ Agent personality & soul definitions

### Training Dataset:
- **Total documents**: 30
- **Instruction-response pairs**: 34
- **Approximate tokens**: ~12,000+

## Model Choice

**Current (Recommended for Quick Start):**
- **TinyLlama-1.1B** - Fast, low memory, good for testing
- Memory: ~4GB VRAM or 16GB RAM
- Training time: ~10-30 minutes on GPU, ~2-5 hours on CPU

**Production:**
- **Llama-2-7B** - Better quality
- Memory: ~16GB VRAM recommended
- Training time: ~2-6 hours on RTX 3090+

To upgrade, just change the `--base-model` parameter.

## Configuration

Edit `config.yaml` to customize:
- Base model
- LoRA rank/alpha
- Learning rate
- Number of epochs
- Batch size

## Integration with ProjectFalcon

Once trained, you can integrate Monarch with Juntos:

1. Copy trained model to Juntos:
   ```bash
   cp -r models/monarch_lora /path/to/ProjectFalcon/models/
   ```

2. Update Juntos AiContextService to use Monarch:
   ```java
   String monarchModelPath = "models/monarch_lora";
   Model model = loadMonarch(monarchModelPath);
   ```

3. Deploy as an AI SIV (Sovereign Integration Vessel)

## Hardware Requirements

| Config | RAM | VRAM | Training Time |
|--------|-----|------|---|
| CPU only | 16GB | - | 2-5 hours |
| RTX 3060 | 16GB | 12GB | 30-60 min |
| RTX 3090 | 32GB | 24GB | 10-20 min |
| Apple M1/M2 | 16GB | shared | 1-3 hours |

## Troubleshooting

**Issue: "No module named 'torch'"**
```bash
pip install torch transformers peft datasets
```

**Issue: Out of memory**
- Reduce `--batch-size` (try 2 or 1)
- Reduce `--lora-rank` (try 4)
- Use smaller base model (TinyLlama instead of Llama-2)

**Issue: CUDA not detected**
- Check: `python3 -c "import torch; print(torch.cuda.is_available())"`
- Falls back to CPU automatically

## Next: Expand Training Data

To improve Monarch:
1. Add more ProjectFalcon documentation
2. Create more synthetic conversations in `src/dataset.py`
3. Include real conversation examples (if available)
4. Retrain with `train.py`

## Philosophy

Monarch embodies the Juntos values:
- **Sovereignty** - No platform owns your data or intelligence
- **Transparency** - Every decision is auditable
- **Decentralization** - The protocol outlasts the product
- **Trust** - Computed, not assigned

---

Ready to train? Start with:
```bash
bash train_monarch.sh
```

Or manually:
```bash
pip install -r requirements.txt
python3 src/train.py --base-model TinyLlama/TinyLlama-1.1B-Chat-v1.0
```
