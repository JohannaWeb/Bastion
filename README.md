[![MIT License](https://img.shields.io/badge/license-MIT-brightgreen)](LICENSE)
[![Live](https://img.shields.io/badge/juntos.chat-live-brightgreen)](https://juntos.chat)
[![Rust](https://img.shields.io/badge/rust-systems-orange)](https://github.com/JohannaWeb/Bastion)
[![Java](https://img.shields.io/badge/java-AT%20Protocol-blue)](https://github.com/JohannaWeb/ProjectFalcon)
# Bastion

**Juntos is live:** https://juntos.chat
<img width="1998" height="1046" alt="image" src="https://github.com/user-attachments/assets/8f352359-f8a1-46d2-8990-524230e2254f" />
<img width="1355" height="898" alt="image" src="https://github.com/user-attachments/assets/5eda6cad-a27e-45a8-983c-0461e4ac8a1a" />
<img width="1025" height="782" alt="image" src="https://github.com/user-attachments/assets/0b0a9623-b46a-46c0-90f1-f5274dd078a4" />


---

## What This Repo Is

Bastion is an umbrella repo for a group of related projects around identity, developer tooling, protocol work, and client software.

The important qualification is that these projects are at very different stages:

- `Juntos` is live
- `ProjectFalcon` is substantial backend/protocol code behind that live system
- `Aurora` and `Opus` are real Rust prototypes
- several other directories are experiments, integration work, or research tracks

If you are reading this from a Rust background, the honest framing is: this repo contains some shipped work, some serious prototypes, and some ideas that are not mature yet.

## Live Today

**Juntos** is the clearest proof that this repo is not just ideas. It is live, user-facing, and built on the protocol work in this ecosystem.

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

### Client
**Aurora** is a from-scratch Rust rendering and browser experiment. It has real code, but it is still a narrow prototype rather than a full browser.

**Gisberta** is a custom browser effort built on top of Servo.

### Compute
**MonarchOS** is the long-term OS direction. It is here as a direction of travel, not as a finished platform.

---

## How To Read This Repo

Bastion is best read as a working notebook plus implementation repo for a larger direction:

1. `Juntos` is the live thing.
2. `ProjectFalcon` is a big part of the backend and protocol foundation behind it.
3. `Opus` and `Aurora` are where a lot of the Rust systems experimentation lives.
4. `FalconPub`, `Falcon-Bridge`, `Monarch`, `Gisberta`, and `MonarchOS` are adjacent efforts that may or may not mature at the same pace.

## What To Judge

The easiest way to read this repo fairly is to judge each project at its actual boundary:

- judge `Juntos` as a live application
- judge `ProjectFalcon` as backend/protocol implementation work
- judge `Aurora` as an early rendering engine prototype
- judge `Opus` as a runtime/policy model prototype

The repo will look worse if everything is read as a finished product line. It will look more accurate if each subproject is read at its own level of maturity.

---

## Research

There are also papers and research notes linked from the relevant project repositories and releases:

1. **[Project Falcon: An Adversarial Algorithmic Trust Protocol for Decentralized Social Identity](https://github.com/JohannaWeb/ProjectFalcon/releases/tag/v1.0-paper)**
2. **[Identity-Driven Discourse Systems (IDDS) v2.1](https://github.com/JohannaWeb/Monarch/releases/tag/2.1.paper)**
3. **[Radical Alignment: Training AI through the Lens of bell hooks' Radical Love](https://github.com/JohannaWeb/Juntos/releases/tag/0.0.1)**

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

→ [GitHub Sponsors](https://github.com/sponsors/JohannaWeb)  
→ [Buy Me a Coffee](https://buymeacoffee.com/johannaweb)

MIT License · Porto, Portugal · 2026
