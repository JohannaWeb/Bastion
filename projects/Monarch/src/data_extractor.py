#!/usr/bin/env python3
"""Extract and parse training data from ProjectFalcon."""

import os
import json
from pathlib import Path
from typing import List, Dict, Any
import re

FALCON_PATH = Path("/home/johanna/projects/ProjectFalcon")
OUTPUT_DIR = Path("/home/johanna/projects/Monarch/data/raw")


def extract_markdown_files() -> Dict[str, str]:
    """Extract all markdown documentation from ProjectFalcon."""
    docs = {}

    md_files = [
        "README.md",
        "ARCHITECTURE.md",
        "RESEARCH_NOTES.md",
        "FOUNDER_STORY.md",
        "MANIFESTO.md",
        "USER_JOURNEY.md",
        "SECURITY.md",
        "ROADMAP.md",
    ]

    agent_files = [
        "agent/personality.md",
        "agent/soul.md",
        "agent/persistence.md",
    ]

    for md_file in md_files + agent_files:
        path = FALCON_PATH / md_file
        if path.exists():
            with open(path, 'r') as f:
                docs[md_file] = f.read()
                print(f"[OK] Extracted {md_file}")

    return docs


def extract_java_patterns() -> List[Dict[str, str]]:
    """Extract domain model patterns from Java source."""
    patterns = []
    java_files = [
        "juntos-alpha/src/main/java/app/juntos/alpha/domain/Message.java",
        "juntos-alpha/src/main/java/app/juntos/alpha/domain/Channel.java",
        "juntos-alpha/src/main/java/app/juntos/alpha/domain/Server.java",
        "juntos-alpha/src/main/java/app/juntos/alpha/domain/Member.java",
    ]

    for java_file in java_files:
        path = FALCON_PATH / java_file
        if path.exists():
            with open(path, 'r') as f:
                content = f.read()
                patterns.append({
                    "file": java_file,
                    "content": content,
                    "type": "domain_model"
                })
                print(f"[OK] Extracted {java_file}")

    return patterns


def extract_lexicons() -> List[Dict[str, Any]]:
    """Extract AT Protocol lexicons."""
    lexicons = []
    lexicon_dir = FALCON_PATH / "lexicons"

    if lexicon_dir.exists():
        for lex_file in lexicon_dir.glob("*.json"):
            try:
                with open(lex_file, 'r') as f:
                    lex_data = json.load(f)
                    lexicons.append({
                        "file": lex_file.name,
                        "data": lex_data
                    })
                    print(f"[OK] Extracted {lex_file.name}")
            except json.JSONDecodeError:
                print(f"[FAIL] Failed to parse {lex_file.name}")

    return lexicons


def create_synthetic_conversations(docs: Dict[str, str]) -> List[str]:
    """Create synthetic training conversations based on extracted knowledge."""
    conversations = []

    # Extract key principles
    soul = docs.get("agent/soul.md", "")
    personality = docs.get("agent/personality.md", "")

    principles = [
        "Sovereignty is non-negotiable. No platform owns identity, data, or intelligence.",
        "Trust is computed, not assigned.",
        "Transparency over black boxes — every decision is auditable.",
        "Decentralization is a value, not a feature.",
        "The protocol outlasts the product.",
        "Direct and precise. No fluff, no filler.",
        "Technically rigorous but never condescending.",
        "Neutral on content, principled on safety.",
        "Confident in analysis, honest about uncertainty."
    ]

    # Create Q&A pairs
    qa_pairs = [
        ("What is Juntos?", "Juntos is a decentralized LGBT community platform built on the AT Protocol. It operates as safe spaces for queer people, entirely independent of centralized governance."),
        ("How does identity work here?", "Identity is anchored in DIDs (Decentralized Identifiers) on the AT Protocol. Your handle is your key, and your social graph is portable. If you move your data, Juntos recognizes the new resolution."),
        ("What makes this different?", "Traditional platforms centralize identity, compute, and trust decisions. Juntos distributes all three. Trust is verified cryptographically, not imposed by a platform."),
        ("How is moderation handled?", "Moderation is transparent and auditable. Every decision is signed and stored as an AiFact. The community can verify actions without trusting a central authority."),
        ("Is this secure?", "Security is rooted in DID resolution and zero-trust verification. We use cryptographic proof instead of session tokens. Every request is verified independently."),
        ("Can I own my data?", "Yes. With AT Protocol, your data lives on your PDS (Personal Data Server). Juntos is just an application layer — you control the underlying data."),
    ]

    for question, answer in qa_pairs:
        conversations.append(f"Q: {question}\nA: {answer}")

    # Add principle-based completions
    for principle in principles:
        conversations.append(f"Principle: {principle}")

    print(f"[OK] Generated {len(conversations)} synthetic conversations")
    return conversations


def save_training_data(docs: Dict[str, str], patterns: List[Dict],
                       lexicons: List[Dict], conversations: List[str]) -> None:
    """Save all extracted data to files."""
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    # Save documentation
    with open(OUTPUT_DIR / "documentation.jsonl", 'w') as f:
        for filename, content in docs.items():
            f.write(json.dumps({
                "source": filename,
                "type": "documentation",
                "content": content
            }) + "\n")

    # Save code patterns
    with open(OUTPUT_DIR / "code_patterns.jsonl", 'w') as f:
        for pattern in patterns:
            f.write(json.dumps(pattern) + "\n")

    # Save lexicons
    with open(OUTPUT_DIR / "lexicons.json", 'w') as f:
        json.dump(lexicons, f, indent=2)

    # Save synthetic conversations
    with open(OUTPUT_DIR / "conversations.txt", 'w') as f:
        for conv in conversations:
            f.write(conv + "\n\n")

    # Create metadata
    metadata = {
        "source": str(FALCON_PATH),
        "extracted_at": str(Path(__file__).parent),
        "files_extracted": list(docs.keys()),
        "code_patterns": len(patterns),
        "lexicons": len(lexicons),
        "synthetic_conversations": len(conversations),
        "total_tokens_approx": sum(len(c.split()) for c in conversations) + sum(len(d.split()) for d in docs.values())
    }

    with open(OUTPUT_DIR / "metadata.json", 'w') as f:
        json.dump(metadata, f, indent=2)

    print(f"\n[OK] Saved all training data to {OUTPUT_DIR}")
    print(f"  - documentation.jsonl")
    print(f"  - code_patterns.jsonl")
    print(f"  - lexicons.json")
    print(f"  - conversations.txt")
    print(f"  - metadata.json")


def main():
    """Extract all data from ProjectFalcon."""
    print("Monarch: Extracting training data from ProjectFalcon...\n")

    if not FALCON_PATH.exists():
        print(f"ProjectFalcon not found at {FALCON_PATH}")
        return

    docs = extract_markdown_files()
    patterns = extract_java_patterns()
    lexicons = extract_lexicons()
    conversations = create_synthetic_conversations(docs)

    save_training_data(docs, patterns, lexicons, conversations)
    print("\nData extraction complete!")


if __name__ == "__main__":
    main()
