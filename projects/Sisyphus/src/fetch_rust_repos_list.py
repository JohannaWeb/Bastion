#!/usr/bin/env python3
"""Fetch popular Rust repositories from awesome-rust and rust-repos index."""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

try:
    import requests
except ImportError:
    print("requests library required. Install with: pip install requests")
    sys.exit(1)


def get_awesome_rust_repos() -> list[tuple[str, str]]:
    """Scrape awesome-rust GitHub README for repository links.

    Returns list of (repo_name, repo_url) tuples.
    """
    print("Fetching awesome-rust repository list...")

    try:
        url = "https://raw.githubusercontent.com/rust-unofficial/awesome-rust/main/README.md"
        response = requests.get(url, timeout=10)
        response.raise_for_status()
        content = response.text
    except Exception as e:
        print(f"  Error fetching awesome-rust: {e}")
        return []

    # Extract GitHub links (basic pattern)
    repos = []
    pattern = r"https://github\.com/([a-zA-Z0-9\-._]+/[a-zA-Z0-9\-._]+)"
    for match in re.finditer(pattern, content):
        repo_url = match.group(0)
        repo_name = match.group(1).split("/")[-1]
        if repo_url not in [r[1] for r in repos]:  # Deduplicate
            repos.append((repo_name, repo_url))

    print(f"  Found {len(repos)} repositories in awesome-rust")
    return repos


def get_rust_repos_index() -> list[tuple[str, str]]:
    """Fetch curated Rust repositories from rust-repos project.

    Returns list of (repo_name, repo_url) tuples.
    """
    print("Fetching rust-repos index...")

    try:
        # rust-repos is a collection of Rust project metadata
        url = "https://raw.githubusercontent.com/rust-lang/rust-repos/main/repos.json"
        response = requests.get(url, timeout=10)
        response.raise_for_status()
        repos_data = response.json()
    except Exception as e:
        print(f"  Error fetching rust-repos: {e}")
        return []

    repos = []
    for repo_info in repos_data.get("repositories", []):
        repo_url = repo_info.get("github_url")
        if repo_url and "github.com" in repo_url:
            repo_name = repo_url.rstrip("/").split("/")[-1]
            repos.append((repo_name, repo_url))

    print(f"  Found {len(repos)} repositories in rust-repos index")
    return repos


def checkout_repo(repo_name: str, repo_url: str, checkout_dir: Path) -> bool:
    """Clone a repository."""
    repo_path = checkout_dir / repo_name

    if repo_path.exists():
        return True

    print(f"  {repo_name:40s}...", end=" ", flush=True)

    try:
        subprocess.run(
            [
                "git",
                "clone",
                "--depth",
                "1",
                repo_url,
                str(repo_path),
            ],
            check=True,
            capture_output=True,
            timeout=30,
        )
        print("✓")
        return True
    except (subprocess.CalledProcessError, subprocess.TimeoutExpired):
        print("✗")
        return False


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Fetch curated Rust repositories from awesome-rust and rust-repos"
    )
    parser.add_argument(
        "--checkout-dir",
        type=Path,
        default=Path("data/external/rust-repos"),
        help="Directory to clone repositories into",
    )
    parser.add_argument(
        "--max-repos",
        type=int,
        default=200,
        help="Maximum number of repos to fetch",
    )
    args = parser.parse_args()

    checkout_dir = args.checkout_dir
    checkout_dir.mkdir(parents=True, exist_ok=True)

    # Fetch from multiple sources
    awesome_repos = get_awesome_rust_repos()
    rust_repos = get_rust_repos_index()

    # Combine and deduplicate
    all_repos = {}
    for name, url in awesome_repos + rust_repos:
        if url not in all_repos.values():
            all_repos[name] = url

    repos_to_clone = list(all_repos.items())[: args.max_repos]

    print(f"\nCloning {len(repos_to_clone)} repositories...")
    success_count = 0
    failed_repos = []

    for i, (repo_name, repo_url) in enumerate(repos_to_clone, 1):
        if checkout_repo(repo_name, repo_url, checkout_dir):
            success_count += 1
        else:
            failed_repos.append(repo_name)
        if i % 25 == 0:
            print(
                f"Progress: {i}/{len(repos_to_clone)} "
                f"({success_count} cloned, {len(failed_repos)} failed)\n"
            )
        time.sleep(0.5)  # Rate limit

    print(f"\n✓ Completed: {success_count}/{len(repos_to_clone)} repositories cloned")
    print(f"✓ Cloned to: {checkout_dir.resolve()}")
    if failed_repos:
        print(f"\n⚠ {len(failed_repos)} repositories failed (will be skipped)")
    print(f"\nNext step: Run build_corpus.py to include cloned repositories")


if __name__ == "__main__":
    main()
