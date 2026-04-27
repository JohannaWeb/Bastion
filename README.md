MIT License Live Rust Java

# Bastion

Juntos is live: https://juntos.chat

---

## What This Repo Is

Bastion is an umbrella repo for a group of related projects around identity, developer tooling, protocol work, and client software.

The important qualification is that these projects are at very different stages:

- Juntos is live
- ProjectFalcon is substantial backend/protocol code behind that live system
- Aurora and Opus are real Rust prototypes
- several other directories are experiments, integration work, or research tracks

If you are reading this from a Rust background, the honest framing is: this repo contains some shipped work, some serious prototypes, and some ideas that are not mature yet.

---

## Live Today

Juntos is the clearest proof that this repo is not just ideas. It is live, user-facing, and built on the protocol work in this ecosystem.

→ https://juntos.chat

---

## The Stack

### Execution & Trust
**Opus** is a Rust prototype for an identity-aware developer runtime. Right now it is mostly a policy, approval, and ledger model with a small UI shell around it.

### Protocol & Collaboration
**ProjectFalcon** is the backend and protocol work behind Juntos and the surrounding AT Protocol experiments.

**FalconPub + Falcon-Bridge** are the federation and bridge experiments for ActivityPub and related interop work.

### Intelligence
**Monarch** is the local-model training and inference track.

The first Monarch result is a 25.6M parameter byte-level Rust language model trained from scratch on a 173.5M byte corpus assembled from rustc, cargo, tokio, serde, ripgrep, and the top 500 crates. Training ran for 30k steps on a single RTX 4060 Ti 8GB.

The model uses a novel **HybridAttention** architecture that replaces standard full attention with a combination of local windowed causal attention and a GRU-like recurrent state path, mixed via a learned per-head gate. Inference uses custom Triton kernels and a two-tier KV cache with 8-bit magnitude/angle compression and selective promotion.

**Benchmark results:**

| Mode | Time | Speed |
|------|------|-------|
| Full attention O(n²) | 17.96s | 5.6 tok/s |
| HybridAttention O(n·W + n·D) | 0.35s | 286.6 tok/s |

**51x speedup** with no visible quality loss. Complexity changes from quadratic to near-linear for typical inference workloads.

→ [HN discussion: Hybrid Attention (40 points)](https://news.ycombinator.com/item?id=47674749)  
→ [Corpus and training: Sisyphus](https://codeberg.org/JohannaJuntos/Sisyphus)

Next steps: ablations comparing hybrid vs local-only vs recurrent-only, syntax-level validation (parse and compile generated code), and scaling context length from 512 to 2048.

### Client
**Aurora** is a from-scratch Rust rendering and browser experiment. It has real code, but it is still a narrow prototype rather than a full browser.

**Gisberta** is a custom browser effort built on top of Servo.

### Compute
**MonarchOS** is the long-term OS direction. It is here as a direction of travel, not as a finished platform.

---

## How To Read This Repo

Bastion is best read as a working notebook plus implementation repo for a larger direction:

- Juntos is the live thing.
- ProjectFalcon is a big part of the backend and protocol foundation behind it.
- Opus and Aurora are where a lot of the Rust systems experimentation lives.
- Monarch is where the AI inference research lives.
- FalconPub, Falcon-Bridge, Gisberta, and MonarchOS are adjacent efforts that may or may not mature at the same pace.

---

## What To Judge

The easiest way to read this repo fairly is to judge each project at its actual boundary:

- judge Juntos as a live application
- judge ProjectFalcon as backend/protocol implementation work
- judge Aurora as an early rendering engine prototype
- judge Opus as a runtime/policy model prototype
- judge Monarch as novel AI inference research with published benchmarks

The repo will look worse if everything is read as a finished product line. It will look more accurate if each subproject is read at its own level of maturity.

---

## Research

Papers and research notes linked from the relevant project repositories and releases:

- [Project Falcon: An Adversarial Algorithmic Trust Protocol for Decentralized Social Identity](#)
- [Identity-Driven Discourse Systems (IDDS) v2.1](#)
- [Radical Alignment: Training AI through the Lens of bell hooks' Radical Love](#)
- [Hybrid Attention: 51x Inference Speedup on Consumer GPUs](https://news.ycombinator.com/item?id=47674749)

---

## Repository Layout

```
projects/
  Aurora/
  Falcon-Bridge/
  FalconPub/
  Gisberta/
  Juntos/
  Monarch/
  MonarchOS/
  Opus/
  ProjectFalcon/
```

---

## Support

This work is self-funded. If you want to support it:

→ [GitHub Sponsors](#)  
→ [Buy Me a Coffee](#)

---

MIT License · Porto, Portugal · 2026
