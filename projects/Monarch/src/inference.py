#!/usr/bin/env python3
"""Run inference with trained Monarch model."""

import torch
import json
import argparse
from pathlib import Path
from typing import Optional
from transformers import AutoTokenizer, AutoModelForCausalLM
from peft import PeftModel


class MonarchInference:
    """Run inference with Monarch."""

    def __init__(
        self,
        model_path: str = "models/monarch_lora",
        base_model: Optional[str] = None,
    ):
        self.model_path = Path(model_path)
        self.device = torch.device("cuda" if torch.cuda.is_available() else "cpu")

        # Load config to get base model if not provided
        config_path = self.model_path / "monarch_config.json"
        if config_path.exists() and not base_model:
            with open(config_path, 'r') as f:
                config = json.load(f)
                base_model = config["base_model"]

        self.base_model = base_model or "TinyLlama/TinyLlama-1.1B-Chat-v1.0"
        self.tokenizer = None
        self.model = None

        self.load_model()

    def load_model(self) -> None:
        """Load base model and LoRA weights."""
        print(f"Loading base model: {self.base_model}")

        self.tokenizer = AutoTokenizer.from_pretrained(
            self.base_model,
            trust_remote_code=True,
        )

        base = AutoModelForCausalLM.from_pretrained(
            self.base_model,
            torch_dtype=torch.float16 if torch.cuda.is_available() else torch.float32,
            device_map="auto" if torch.cuda.is_available() else None,
            trust_remote_code=True,
        )

        print(f"[OK] Base model loaded")
        print(f"Loading LoRA weights from {self.model_path}")

        self.model = PeftModel.from_pretrained(base, self.model_path)
        self.model.to(self.device)

        print(f"[OK] LoRA weights loaded")

    def generate(
        self,
        prompt: str,
        max_length: int = 256,
        temperature: float = 0.7,
        top_p: float = 0.9,
    ) -> str:
        """Generate response from prompt."""
        inputs = self.tokenizer(
            prompt,
            return_tensors="pt",
            truncation=True,
            max_length=512,
        ).to(self.device)

        with torch.no_grad():
            outputs = self.model.generate(
                **inputs,
                max_length=max_length,
                temperature=temperature,
                top_p=top_p,
                do_sample=True,
                pad_token_id=self.tokenizer.eos_token_id,
            )

        response = self.tokenizer.decode(outputs[0], skip_special_tokens=True)
        return response

    def chat(self) -> None:
        """Interactive chat with Monarch."""
        print("\nMonarch - Interactive Mode")
        print("Type 'quit' to exit\n")

        while True:
            prompt = input("You: ").strip()

            if prompt.lower() == 'quit':
                print("Goodbye!")
                break

            if not prompt:
                continue

            response = self.generate(prompt)
            print(f"\nMonarch: {response}\n")


def main():
    """Main inference entry point."""
    parser = argparse.ArgumentParser(description="Run Monarch inference")
    parser.add_argument("--model", default="models/monarch_lora",
                        help="Path to Monarch LoRA model")
    parser.add_argument("--base-model", default=None,
                        help="Base model (if different from config)")
    parser.add_argument("--prompt", default=None,
                        help="Prompt to generate from (interactive if not provided)")
    parser.add_argument("--max-length", type=int, default=256,
                        help="Max generation length")
    parser.add_argument("--temperature", type=float, default=0.7,
                        help="Temperature for sampling")

    args = parser.parse_args()

    inference = MonarchInference(
        model_path=args.model,
        base_model=args.base_model,
    )

    if args.prompt:
        response = inference.generate(
            args.prompt,
            max_length=args.max_length,
            temperature=args.temperature,
        )
        print(f"\n{response}")
    else:
        inference.chat()


if __name__ == "__main__":
    main()
