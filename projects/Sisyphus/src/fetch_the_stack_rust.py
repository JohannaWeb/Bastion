#!/usr/bin/env python3
"""Fetch Rust code from The Stack (bigcode/the-stack) HuggingFace dataset."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

try:
    from datasets import load_dataset
except ImportError:
    print("datasets library required. Install with: pip install datasets")
    sys.exit(1)


def fetch_the_stack_rust(
    output_dir: Path,
    max_files: int = 50000,
    max_total_chars: int = 100000000,
) -> None:
    """Fetch Rust code from The Stack dataset.

    Args:
        output_dir: Directory to save Rust files
        max_files: Maximum number of files to fetch
        max_total_chars: Maximum total characters to collect
    """
    output_dir.mkdir(parents=True, exist_ok=True)

    print(f"Loading The Stack dataset (rust subset)...")
    print("This may take a few minutes on first run (downloads ~5-20GB)...")

    # Try main dataset first, fall back to pre-cleaned if gated/unavailable
    dataset = None
    try:
        print("Attempting to load bigcode/the-stack (gated dataset)...")
        dataset = load_dataset(
            "bigcode/the-stack",
            data_dir="data/rust",
            split="train",
            streaming=True,
        )
        print("✓ Loaded bigcode/the-stack")
    except Exception as e:
        print(f"Note: {e}")
        print("\nFalling back to pre-cleaned dataset (ammarnasr/the-stack-rust-clean)...")
        try:
            dataset = load_dataset(
                "ammarnasr/the-stack-rust-clean",
                split="train",
                streaming=True,
            )
            print("✓ Loaded the-stack-rust-clean")
        except Exception as e2:
            print(f"Error loading alternative dataset: {e2}")
            sys.exit(1)

    file_count = 0
    total_chars = 0
    skipped_binary = 0
    skipped_short = 0

    files_dir = output_dir / "files"
    files_dir.mkdir(parents=True, exist_ok=True)
    metadata_list = []

    print(f"Streaming and saving Rust files...")

    for idx, example in enumerate(dataset):
        if file_count >= max_files or total_chars >= max_total_chars:
            break

        # The Stack has 'content' and 'metadata' fields
        content = example.get("content", "")
        if not content or len(content) < 40:
            skipped_short += 1
            continue

        # Skip if appears to be binary
        if "\x00" in content:
            skipped_binary += 1
            continue

        # Save file
        file_path = files_dir / f"file_{file_count:06d}.rs"
        try:
            file_path.write_text(content, encoding="utf-8", errors="ignore")
            total_chars += len(content)

            # Track metadata
            metadata_list.append({
                "file_id": file_count,
                "source": "the-stack",
                "size_chars": len(content),
                "metadata": {k: v for k, v in example.items() if k != "content"},
            })

            file_count += 1

            if (file_count + 1) % 1000 == 0:
                print(f"  Saved {file_count} files ({total_chars / 1e6:.1f}M chars)...")
        except Exception as e:
            print(f"  Error saving file {file_count}: {e}")
            continue

    # Save metadata
    metadata_path = output_dir / "metadata.jsonl"
    with metadata_path.open("w") as f:
        for item in metadata_list:
            f.write(json.dumps(item) + "\n")

    print(f"\n✓ Completed: {file_count} files saved")
    print(f"✓ Total characters: {total_chars / 1e6:.1f}M")
    print(f"✓ Saved to: {files_dir}")
    print(f"✓ Metadata: {metadata_path}")
    print(f"⚠ Skipped: {skipped_binary} binary, {skipped_short} too short")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Fetch Rust code from The Stack HuggingFace dataset"
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path("data/external/the-stack-rust"),
        help="Directory to save files",
    )
    parser.add_argument(
        "--max-files",
        type=int,
        default=50000,
        help="Maximum number of files to fetch",
    )
    parser.add_argument(
        "--max-chars",
        type=int,
        default=100000000,
        help="Maximum total characters to collect",
    )
    args = parser.parse_args()

    fetch_the_stack_rust(args.output_dir, args.max_files, args.max_chars)


if __name__ == "__main__":
    main()
