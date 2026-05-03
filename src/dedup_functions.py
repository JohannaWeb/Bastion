#!/usr/bin/env python3
"""Post-process corpus.txt to deduplicate function definitions across Rust files."""

from __future__ import annotations

import argparse
import hashlib
import re
from pathlib import Path
from typing import Iterator


def extract_functions(source: str) -> list[tuple[int, int, str]]:
    """Extract (start_pos, end_pos, function_text) for each fn definition in Rust source.

    Uses a state machine to correctly handle:
    - String literals (double and raw strings)
    - Line comments (//)
    - Block comments (/* */)
    - Brace-counted function body boundaries
    """
    results = []
    i = 0
    n = len(source)

    while i < n:
        # Skip strings, comments, and characters until we find "fn "
        char = source[i]

        # Handle strings
        if char == '"':
            if i > 0 and source[i - 1] == 'r':
                # Raw string r"..."
                i += 1
                while i < n and source[i] != '"':
                    if source[i] == '\\':
                        i += 2
                    else:
                        i += 1
                if i < n:
                    i += 1  # consume closing quote
            else:
                # Regular string "..."
                i += 1
                while i < n and source[i] != '"':
                    if source[i] == '\\':
                        i += 2
                    else:
                        i += 1
                if i < n:
                    i += 1  # consume closing quote
            continue

        # Handle line comments
        if i < n - 1 and source[i : i + 2] == "//":
            i += 2
            while i < n and source[i] not in "\n\r":
                i += 1
            if i < n:
                i += 1  # consume newline
            continue

        # Handle block comments
        if i < n - 1 and source[i : i + 2] == "/*":
            i += 2
            while i < n - 1:
                if source[i : i + 2] == "*/":
                    i += 2
                    break
                i += 1
            continue

        # Look for "fn " (fn keyword followed by whitespace)
        if char == "f" and i < n - 2 and source[i : i + 2] == "fn":
            # Check that next char is word boundary (space, tab, or paren for closure)
            next_char = source[i + 2] if i + 2 < n else " "
            if next_char in " \t\n\r(":
                fn_start = i
                i += 2  # Skip "fn"

                # Find opening brace
                brace_start = source.find("{", i)
                if brace_start == -1:
                    i += 1
                    continue

                # Count braces from opening to closing
                depth = 0
                pos = brace_start
                fn_end = -1

                while pos < n:
                    char_at = source[pos]

                    # Handle strings inside the function body
                    if char_at == '"':
                        pos += 1
                        while pos < n and source[pos] != '"':
                            if source[pos] == '\\':
                                pos += 2
                            else:
                                pos += 1
                        if pos < n:
                            pos += 1
                        continue

                    # Handle comments inside function body
                    if pos < n - 1 and source[pos : pos + 2] == "//":
                        pos += 2
                        while pos < n and source[pos] not in "\n\r":
                            pos += 1
                        if pos < n:
                            pos += 1
                        continue

                    if pos < n - 1 and source[pos : pos + 2] == "/*":
                        pos += 2
                        while pos < n - 1 and source[pos : pos + 2] != "*/":
                            pos += 1
                        if pos < n - 1:
                            pos += 2
                        continue

                    # Count braces
                    if char_at == "{":
                        depth += 1
                    elif char_at == "}":
                        depth -= 1
                        if depth == 0:
                            fn_end = pos + 1
                            break

                    pos += 1

                if fn_end != -1:
                    fn_text = source[fn_start : fn_end]
                    results.append((fn_start, fn_end, fn_text))
                    i = fn_end
                else:
                    i += 1
            else:
                i += 1
        else:
            i += 1

    return results


def normalize_fn(fn_text: str) -> str:
    """Normalize function text for comparison: collapse whitespace, strip comments, dedent."""
    lines = []
    for line in fn_text.splitlines():
        # Remove line comments from this line
        if "//" in line:
            line = line[: line.index("//")]
        # Collapse whitespace within line
        line = " ".join(line.split())
        if line:
            lines.append(line)
    return "\n".join(lines).strip()


def parse_corpus_entries(corpus_path: Path) -> Iterator[dict]:
    """Parse corpus.txt entry by entry. Yields dicts with source, path, content."""
    with corpus_path.open("r", encoding="utf-8") as f:
        current_source = None
        current_path = None
        content_lines = []
        in_entry = False

        for line in f:
            # Check for FILE header
            if line.startswith("<FILE"):
                if in_entry and current_source is not None:
                    # Yield previous entry
                    yield {
                        "source": current_source,
                        "path": current_path,
                        "content": "".join(content_lines),
                    }
                    content_lines = []

                # Parse new header
                match = re.search(r'source="([^"]+)"', line)
                source = match.group(1) if match else "unknown"
                match = re.search(r'path="([^"]+)"', line)
                path = match.group(1) if match else "unknown"

                current_source = source
                current_path = path
                in_entry = True
            elif line.startswith("</FILE>"):
                if in_entry and current_source is not None:
                    yield {
                        "source": current_source,
                        "path": current_path,
                        "content": "".join(content_lines),
                    }
                    content_lines = []
                    in_entry = False
            elif in_entry:
                content_lines.append(line)


def remove_duplicate_functions(
    content: str, path: str, seen_hashes: set[str]
) -> tuple[str, int, int]:
    """Remove duplicate functions from content. Returns (filtered_content, total, removed)."""
    if not path.endswith(".rs"):
        return content, 0, 0

    functions = extract_functions(content)
    if not functions:
        return content, 0, 0

    total = len(functions)
    removed = 0
    filtered_content = content
    offset = 0  # Track position changes after replacements

    for start, end, fn_text in sorted(functions, reverse=True):
        normalized = normalize_fn(fn_text)
        fn_hash = hashlib.sha256(normalized.encode("utf-8")).hexdigest()

        if fn_hash in seen_hashes:
            # Extract function name for the placeholder
            fn_name_match = re.search(r"fn\s+(\w+)", fn_text)
            fn_name = fn_name_match.group(1) if fn_name_match else "unknown"
            placeholder = f"// [dedup: fn {fn_name} removed - duplicate]\n"

            # Replace in filtered_content (working backwards to preserve positions)
            filtered_content = (
                filtered_content[: start + offset]
                + placeholder
                + filtered_content[end + offset :]
            )
            offset += len(placeholder) - (end - start)
            removed += 1
        else:
            seen_hashes.add(fn_hash)

    return filtered_content, total, removed


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Deduplicate function definitions across Rust files in corpus.txt"
    )
    parser.add_argument(
        "--corpus",
        type=Path,
        default=Path("data/processed/corpus.txt"),
        help="Path to input corpus.txt",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("data/processed/corpus_deduped.txt"),
        help="Path to output deduped corpus",
    )
    args = parser.parse_args()

    if not args.corpus.exists():
        print(f"Error: corpus file not found at {args.corpus}")
        return

    seen_hashes: set[str] = set()
    total_entries = 0
    rust_entries = 0
    total_functions = 0
    total_removed = 0
    removed_by_source: dict[str, int] = {}

    args.output.parent.mkdir(parents=True, exist_ok=True)

    with args.output.open("w", encoding="utf-8") as out:
        for entry in parse_corpus_entries(args.corpus):
            source = entry["source"]
            path = entry["path"]
            content = entry["content"]
            total_entries += 1

            if path.endswith(".rs"):
                rust_entries += 1
                filtered, total, removed = remove_duplicate_functions(
                    content, path, seen_hashes
                )
                total_functions += total
                total_removed += removed
                removed_by_source[source] = removed_by_source.get(source, 0) + removed
            else:
                filtered = content

            # Write entry
            out.write(f'<FILE source="{source}" path="{path}">\n')
            out.write(filtered)
            if not filtered.endswith("\n"):
                out.write("\n")
            out.write("</FILE>\n\n")

    print(f"\n✓ Function-level deduplication complete")
    print(f"  Processed {total_entries} files ({rust_entries} .rs files)")
    print(f"  Found {total_functions} total function definitions")
    print(f"  Removed {total_removed} duplicate functions ({100*total_removed/max(1,total_functions):.1f}%)")
    print(f"\nDuplicates removed by source:")
    for source in sorted(removed_by_source.keys()):
        count = removed_by_source[source]
        if count > 0:
            print(f"  {source:30s}: {count:5d}")
    print(f"\n✓ Output written to {args.output}")


if __name__ == "__main__":
    main()
