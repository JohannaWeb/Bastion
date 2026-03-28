#!/bin/bash
set -e

echo "Monarch: Full Training Pipeline"
echo "=================================="

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

# Step 1: Extract data from ProjectFalcon
echo -e "\n${BLUE}[1/3] Extracting training data from ProjectFalcon...${NC}"
python src/data_extractor.py

# Step 2: Prepare dataset
echo -e "\n${BLUE}[2/3] Preparing training dataset...${NC}"
python src/dataset.py

# Step 3: Train model
echo -e "\n${BLUE}[3/3] Training Monarch with LoRA...${NC}"
python src/train.py \
    --base-model "TinyLlama/TinyLlama-1.1B-Chat-v1.0" \
    --data "data/processed/texts.txt" \
    --epochs 3 \
    --batch-size 4 \
    --lora-rank 8 \
    --lora-alpha 16 \
    --output-dir "models/monarch_lora" \
    --learning-rate 2e-4

echo -e "\n${GREEN}[OK] Monarch training complete!${NC}"
echo -e "Model saved to: models/monarch_lora"
echo -e "\nRun inference with:"
echo -e "  python src/inference.py --model models/monarch_lora"
echo -e "  python src/inference.py --model models/monarch_lora --prompt 'Your question here'"
