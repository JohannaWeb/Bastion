#!/usr/bin/env python3
"""Train Monarch using LoRA on a base model."""

import argparse
import json
from pathlib import Path
from typing import Optional

import torch
from datasets import Dataset
from peft import LoraConfig, TaskType, get_peft_model
from transformers import (
    AutoModelForCausalLM,
    AutoTokenizer,
    DataCollatorForLanguageModeling,
    Trainer,
    TrainingArguments,
)


class MonarchTrainer:
    """Handle training of Monarch with LoRA."""

    def __init__(
        self,
        base_model: str = "meta-llama/Llama-2-7b",
        lora_rank: int = 8,
        lora_alpha: int = 16,
        lora_dropout: float = 0.05,
        output_dir: str = "models/monarch_lora",
    ):
        self.base_model = base_model
        self.lora_rank = lora_rank
        self.lora_alpha = lora_alpha
        self.lora_dropout = lora_dropout
        self.output_dir = Path(output_dir)
        self.output_dir.mkdir(parents=True, exist_ok=True)

        self.device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
        print(f"Using device: {self.device}")

        self.tokenizer = None
        self.model = None

    def load_model(self) -> None:
        """Load base model and tokenizer."""
        print(f"Loading base model: {self.base_model}")

        self.tokenizer = AutoTokenizer.from_pretrained(
            self.base_model, trust_remote_code=True, padding_side="left"
        )
        self.tokenizer.pad_token = self.tokenizer.eos_token

        self.model = AutoModelForCausalLM.from_pretrained(
            self.base_model,
            torch_dtype=torch.float16 if torch.cuda.is_available() else torch.float32,
            device_map="auto" if torch.cuda.is_available() else None,
            trust_remote_code=True,
        )

        print(f"[OK] Model loaded successfully")

    def setup_lora(self) -> None:
        """Configure and apply LoRA."""
        print(f"\nConfiguring LoRA (rank={self.lora_rank}, alpha={self.lora_alpha})")

        lora_config = LoraConfig(
            task_type=TaskType.CAUSAL_LM,
            r=self.lora_rank,
            lora_alpha=self.lora_alpha,
            lora_dropout=self.lora_dropout,
            bias="none",
            target_modules=["q_proj", "v_proj"],
        )

        self.model = get_peft_model(self.model, lora_config)
        self.model.print_trainable_parameters()
        print(f"[OK] LoRA applied successfully")

    def prepare_dataset(
        self, data_path: str = "data/processed/texts.txt", max_length: int = 512
    ) -> Dataset:
        """Prepare dataset for training."""
        print(f"Preparing dataset from {data_path}")

        if not Path(data_path).exists():
            print(f"[ERROR] Dataset file not found: {data_path}")
            return None

        # Load texts
        with open(data_path, "r") as f:
            texts = f.read().split("---")
            texts = [t.strip() for t in texts if t.strip()]

        print(f"[OK] Loaded {len(texts)} documents")

        # Tokenize
        def tokenize_function(examples):
            return self.tokenizer(
                examples["text"],
                padding="max_length",
                truncation=True,
                max_length=max_length,
                return_tensors="pt",
            )

        dataset = Dataset.from_dict({"text": texts})
        tokenized_dataset = dataset.map(
            tokenize_function,
            batched=True,
            remove_columns=["text"],
        )

        print(f"[OK] Tokenized {len(tokenized_dataset)} examples")
        return tokenized_dataset

    def train(
        self,
        dataset,
        num_epochs: int = 3,
        batch_size: int = 4,
        learning_rate: float = 2e-4,
        warmup_steps: int = 100,
        save_steps: int = 50,
    ) -> None:
        """Fine-tune model with LoRA."""
        if dataset is None:
            print("[ERROR] No dataset provided")
            return

        print(f"Starting training...")
        print(f"  Epochs: {num_epochs}")
        print(f"  Batch size: {batch_size}")
        print(f"  Learning rate: {learning_rate}")

        training_args = TrainingArguments(
            output_dir=str(self.output_dir),
            num_train_epochs=num_epochs,
            per_device_train_batch_size=batch_size,
            gradient_accumulation_steps=2,
            learning_rate=learning_rate,
            warmup_steps=warmup_steps,
            weight_decay=0.01,
            save_steps=save_steps,
            save_total_limit=2,
            logging_steps=5,
            max_grad_norm=1.0,
            optim="paged_adamw_8bit" if torch.cuda.is_available() else "adamw_torch",
            bf16=False,
            fp16=torch.cuda.is_available(),
            gradient_checkpointing=True,
        )

        trainer = Trainer(
            model=self.model,
            args=training_args,
            train_dataset=dataset,
            data_collator=DataCollatorForLanguageModeling(self.tokenizer, mlm=False),
        )

        trainer.train()

        print(f"Training complete!")
        self.save_model()

    def save_model(self) -> None:
        """Save model and configuration."""
        print(f"Saving model to {self.output_dir}")

        self.model.save_pretrained(self.output_dir)
        self.tokenizer.save_pretrained(self.output_dir)

        # Save config
        config = {
            "base_model": self.base_model,
            "lora_rank": self.lora_rank,
            "lora_alpha": self.lora_alpha,
            "lora_dropout": self.lora_dropout,
            "device": str(self.device),
        }

        with open(self.output_dir / "monarch_config.json", "w") as f:
            json.dump(config, f, indent=2)

        print(f"[OK] Model saved successfully")


def main():
    """Main training entry point."""
    parser = argparse.ArgumentParser(description="Train Monarch with LoRA")
    parser.add_argument(
        "--base-model",
        default="TinyLlama/TinyLlama-1.1B-Chat-v1.0",
        help="Base model to fine-tune",
    )
    parser.add_argument(
        "--data", default="data/processed/texts.txt", help="Path to training data"
    )
    parser.add_argument(
        "--epochs", type=int, default=3, help="Number of training epochs"
    )
    parser.add_argument("--batch-size", type=int, default=4, help="Training batch size")
    parser.add_argument("--lora-rank", type=int, default=8, help="LoRA rank")
    parser.add_argument("--lora-alpha", type=int, default=16, help="LoRA alpha")
    parser.add_argument(
        "--output-dir",
        default="models/monarch_lora",
        help="Output directory for trained model",
    )
    parser.add_argument(
        "--learning-rate", type=float, default=2e-4, help="Learning rate"
    )

    args = parser.parse_args()

    print("Monarch: Fine-tuning with LoRA\n")

    trainer = MonarchTrainer(
        base_model=args.base_model,
        lora_rank=args.lora_rank,
        lora_alpha=args.lora_alpha,
        output_dir=args.output_dir,
    )

    trainer.load_model()
    trainer.setup_lora()
    dataset = trainer.prepare_dataset(args.data)

    if dataset:
        trainer.train(
            dataset,
            num_epochs=args.epochs,
            batch_size=args.batch_size,
            learning_rate=args.learning_rate,
        )


if __name__ == "__main__":
    main()
