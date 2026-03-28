#!/usr/bin/env python3
"""Prepare training dataset from extracted ProjectFalcon data."""

import json
from pathlib import Path
from typing import List, Dict, Any
import random


class MonarchDataset:
    """Prepare and manage training data for Monarch."""

    def __init__(self, data_dir: str = "data/raw", output_dir: str = "data/processed"):
        self.data_dir = Path(data_dir)
        self.output_dir = Path(output_dir)
        self.output_dir.mkdir(parents=True, exist_ok=True)
        self.texts = []

    def load_documentation(self) -> List[str]:
        """Load documentation files."""
        docs_file = self.data_dir / "documentation.jsonl"
        texts = []

        if docs_file.exists():
            with open(docs_file, 'r') as f:
                for line in f:
                    try:
                        data = json.loads(line)
                        texts.append(data["content"])
                    except json.JSONDecodeError:
                        pass

        print(f"[OK] Loaded {len(texts)} documentation files")
        return texts

    def load_conversations(self) -> List[str]:
        """Load synthetic conversations."""
        conv_file = self.data_dir / "conversations.txt"
        texts = []

        if conv_file.exists():
            with open(conv_file, 'r') as f:
                content = f.read()
                # Split by double newlines
                conversations = content.split("\n\n")
                texts = [c.strip() for c in conversations if c.strip()]

        print(f"[OK] Loaded {len(texts)} conversations")
        return texts

    def load_code_patterns(self) -> List[str]:
        """Load code patterns as documentation."""
        code_file = self.data_dir / "code_patterns.jsonl"
        texts = []

        if code_file.exists():
            with open(code_file, 'r') as f:
                for line in f:
                    try:
                        data = json.loads(line)
                        content = data.get("content", "")
                        if content:
                            texts.append(f"Code Pattern ({data.get('file', 'unknown')}):\n{content}")
                    except json.JSONDecodeError:
                        pass

        print(f"[OK] Loaded {len(texts)} code patterns")
        return texts

    def create_instruction_pairs(self, texts: List[str]) -> List[Dict[str, str]]:
        """Create instruction-response pairs for fine-tuning."""
        pairs = []

        instructions = [
            "Explain this concept:",
            "How would Juntos approach this:",
            "What is the principle behind:",
            "Summarize the following:",
            "What are the key insights in:",
        ]

        for text in texts:
            if len(text.split()) < 20:
                continue

            # Split longer texts into chunks
            sentences = text.split(". ")
            if len(sentences) > 3:
                chunk_size = max(1, len(sentences) // 3)
                for i in range(0, len(sentences) - chunk_size, chunk_size):
                    chunk = ". ".join(sentences[i:i + chunk_size])
                    if chunk.strip():
                        instruction = random.choice(instructions)
                        pairs.append({
                            "instruction": instruction,
                            "input": chunk,
                            "output": chunk  # For now, use as both input and output
                        })
            else:
                instruction = random.choice(instructions)
                pairs.append({
                    "instruction": instruction,
                    "input": text,
                    "output": text
                })

        return pairs

    def prepare_training_data(self) -> None:
        """Prepare all training data."""
        print("\nPreparing training dataset...\n")

        # Load all texts
        docs = self.load_documentation()
        conversations = self.load_conversations()
        code = self.load_code_patterns()

        all_texts = docs + conversations + code
        print(f"[OK] Total documents loaded: {len(all_texts)}")

        # Create instruction pairs
        pairs = self.create_instruction_pairs(all_texts)
        print(f"[OK] Created {len(pairs)} instruction-response pairs")

        # Save instruction dataset
        output_file = self.output_dir / "instructions.jsonl"
        with open(output_file, 'w') as f:
            for pair in pairs:
                f.write(json.dumps(pair) + "\n")

        # Save raw texts for pretraining
        text_file = self.output_dir / "texts.txt"
        with open(text_file, 'w') as f:
            for text in all_texts:
                f.write(text + "\n\n---\n\n")

        # Create metadata
        metadata = {
            "total_documents": len(all_texts),
            "instruction_pairs": len(pairs),
            "text_file": str(text_file),
            "instructions_file": str(output_file)
        }

        with open(self.output_dir / "dataset_metadata.json", 'w') as f:
            json.dump(metadata, f, indent=2)

        print(f"\nDataset prepared!")
        print(f"  - {output_file}")
        print(f"  - {text_file}")


def main():
    """Prepare training dataset."""
    dataset = MonarchDataset()
    dataset.prepare_training_data()


if __name__ == "__main__":
    main()
