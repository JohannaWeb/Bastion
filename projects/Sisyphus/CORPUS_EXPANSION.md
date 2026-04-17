# Corpus Expansion to 200M+ Characters

## Strategy

Expand training corpus from ~31M to 200M+ characters by fetching from multiple complementary sources:

### Data Sources

| Source | Script | Allocation | Notes |
|--------|--------|-----------|-------|
| **Top 500 Crates** | `fetch_top_crates.py` | 150M chars | Most-downloaded from crates.io; robust with automatic failure skipping |
| **The Stack (Rust)** | `fetch_the_stack_rust.py` | 50M chars | HuggingFace dataset; large corpus of permissively licensed Rust code |
| **Awesome-Rust Repos** | `fetch_rust_repos_list.py` | 50M chars | Community-curated popular repositories; high-quality, well-maintained code |
| **Existing Web Docs** | `build_corpus.py` | ~30M chars | Rust book, reference, examples (already in config) |

**Total Target: 250M+ characters**

## Execution Steps

### 1. Fetch crates from crates.io (autonomous, handles failures)
```bash
python3 src/fetch_top_crates.py --count 500
```
- Fetches top 500 by download count
- Uses shallow clones (--depth 1) to minimize disk usage
- Skips repos that fail to clone (reported at end)
- ~10-20 minutes with API rate limiting

### 2. Fetch from The Stack (HuggingFace)
```bash
python3 src/fetch_the_stack_rust.py \
  --max-files 50000 \
  --max-chars 50000000
```
- Downloads Rust code from bigcode/the-stack dataset
- Streaming mode (doesn't require full dataset download)
- Alternative: Use pre-cleaned subset `ammarnasr/the-stack-rust-clean` if main fails
- ~5-20 minutes (depending on network)

### 3. Fetch curated awesome-rust repositories
```bash
python3 src/fetch_rust_repos_list.py \
  --max-repos 200 \
  --checkout-dir data/external/rust-repos
```
- Scrapes awesome-rust GitHub README
- Fetches rust-repos index
- Clones most popular/well-maintained repos
- Handles clone failures gracefully
- ~5-10 minutes

### 4. Build combined corpus
```bash
python3 src/build_corpus.py --config config.yaml
```
- Combines all sources into single corpus.txt
- Deduplicates exact matches
- Generates corpus_metadata.json with statistics
- File filters: .rs, .toml, .md, .txt, .rst
- Character limits respected per-source
- ~2-5 minutes depending on disk speed

## Corpus Statistics (Expected)

- **Total Files**: ~10,000-15,000 (from all sources)
- **Total Characters**: 250M+ (target)
- **Unique Content**: High (deduplication enabled)
- **File Breakdown**: 
  - Rust code (.rs): ~70%
  - Configuration/Manifests (.toml): ~15%
  - Documentation (.md, .txt, .rst): ~15%

## Key Implementation Details

### Failure Handling
- `fetch_top_crates.py`: Skips individual crate failures, reports summary
- `fetch_the_stack_rust.py`: Streams dataset, skips corrupt/binary files
- `fetch_rust_repos_list.py`: Skips repos that don't clone, reports summary
- `build_corpus.py`: Skips unreadable files, reports detailed statistics

### Deduplication
- Exact-match deduplication enabled (SHA256 of normalized content)
- Normalizes whitespace for comparison
- Prevents duplicate training examples

### File Filtering
- Excluded directories: .git, node_modules, target, vendor, build (build artifacts)
- Allowed extensions: .rs, .toml, .md, .txt, .rst
- Excluded formats: binaries, images, archives
- Size constraints: 256KB max per file, 20KB per-file truncation, 40 char minimum

## Total Wall-Clock Time

- **Parallel execution** (run fetches in separate terminals):
  - Top 500 crates: ~15 min
  - The Stack: ~15 min  
  - Awesome-rust: ~10 min
  - **Parallel total: ~15-20 min**

- **Sequential execution** (one after another):
  - ~40-50 minutes total + corpus building

- **Corpus building**: ~2-5 min (sequential, single-threaded)

## Troubleshooting

### The Stack dataset not available
```python
# Use pre-cleaned subset instead
from datasets import load_dataset
ds = load_dataset("ammarnasr/the-stack-rust-clean")
```

### crates.io API rate limit
- Natural delays built in (0.1s between requests)
- Typically not an issue for 500 crates
- If needed, pass `--count 100` for smaller subset

### Disk space
- Top 500 crates: ~5-10GB (shallow clones)
- The Stack Rust: ~2-5GB (files only, compressed downloads)
- Awesome-rust repos: ~2-3GB
- **Total: ~10-20GB** (temporary; corpus compresses to ~600MB-1GB)

## Next Steps

After corpus building:
```bash
# Resume or start fresh training
python3 src/train.py --config config.yaml
# or with resumption from last checkpoint
python3 src/train.py --config config.yaml --resume checkpoints/sisyphus.last.pt
```
