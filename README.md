[![MIT License](https://img.shields.io/badge/license-MIT-brightgreen)](LICENSE)
[![Live](https://img.shields.io/badge/juntos.chat-live-brightgreen)](https://juntos.chat)
[![Rust](https://img.shields.io/badge/rust-systems-orange)](https://github.com/JohannaWeb/Bastion)
[![Java](https://img.shields.io/badge/java-AT%20Protocol-blue)](https://github.com/JohannaWeb/ProjectFalcon)
# Bastion — Sovereign Developer Infrastructure

**Juntos is live:** https://juntos.chat
<img width="1998" height="1046" alt="image" src="https://github.com/user-attachments/assets/8f352359-f8a1-46d2-8990-524230e2254f" />
<img width="1355" height="898" alt="image" src="https://github.com/user-attachments/assets/5eda6cad-a27e-45a8-983c-0461e4ac8a1a" />
<img width="1025" height="782" alt="image" src="https://github.com/user-attachments/assets/0b0a9623-b46a-46c0-90f1-f5274dd078a4" />


---

## The Problem

The current developer stack is owned by three companies. Agents are opaque. Permissions are bolted on. Provenance is missing. Identity is rented.

When AI starts acting on your behalf, this stops being a UX problem and becomes an infrastructure problem.

## The Thesis

Bastion is a vertically integrated sovereign computing stack — from the OS layer to the browser, IDE, AI model, and social protocol.

The core bet: execution should be capability-bound, actions should be signed and auditable, collaboration should be protocol-native, and the stack should be owned — not rented.

## Live Today

**Juntos** — the world's first pure AT Protocol implementation of real-time decentralized chat. No wrappers. No non-spec extensions. Built for trans and queer communities. Running in production.

→ https://juntos.chat

---

## The Stack

### Execution & Trust
**Opus** — Sovereign developer runtime and IDE. Every principal (developer, agent, plugin, organization) has a DID-like identifier with declared capabilities, approvals, and signed provenance. Not an editor with a chatbot — a policy-enforced execution environment.

### Protocol & Collaboration
**ProjectFalcon** — World-first native JVM implementation of AT Protocol. Hand-rolled ES256K/secp256k1 cryptographic verification. Adversarial trust protocol with Sybil resistance, temporal decay, and on-chain attestation via EAS. Directly addresses the bootstrapping vulnerability in EigenTrust (Kamvar et al.).

**FalconPub + Falcon-Bridge** — ActivityPub federation server and interoperability layer between AT Protocol and the Fediverse.

### Intelligence
**Monarch** — Mistral-based fine-tune trained on the Bastion codebase. 4-bit quantization, aggressive KV cache compression, RAG. Stack-native intelligence, not generic assistant behavior. Runs on a 4060 Ti.

### Client
**Aurora / Gisberta** — Hand-rolled Rust browser engine and sovereign shell. No Chromium. No WebView. Full HTML parser, CSS cascade engine, GPU rendering pipeline, box model layout. DID-native identity and AT Protocol integration built in.

### Compute
**MonarchOS** — 64-bit Rust kernel. Custom process scheduler, memory manager, decentralized identity runtime. The long-term machine layer.

---

## Why It Compounds

Each layer makes the others stronger.

- `Opus` makes AI execution governable
- `ProjectFalcon` makes identity and collaboration portable
- `FalconPub` and `Falcon-Bridge` expand that across protocols
- `Monarch` reasons inside the same trust and domain model
- `Aurora` and `MonarchOS` extend sovereignty down to the metal

This is not a collection of side projects. It is one vertically integrated stack, built solo, in 36 days, on a 4060 Ti, in Porto.

---

## Research

Three papers published March 2026:

1. **[Project Falcon: An Adversarial Algorithmic Trust Protocol for Decentralized Social Identity](https://github.com/JohannaWeb/ProjectFalcon/releases/tag/v1.0-paper)** — formal security analysis, SIV bootstrapping, EigenTrust vulnerability fix
2. **[Identity-Driven Discourse Systems (IDDS) v2.1](https://github.com/JohannaWeb/Monarch/releases/tag/2.1.paper)** — conflict modeling, MPF detection, Sovereign Moderation framework
3. **[Radical Alignment: Training AI through the Lens of bell hooks' Radical Love](https://github.com/JohannaWeb/Juntos/releases/tag/0.0.1)** — alignment framework grounded in care ethics

---

## Repository Layout

```
projects/
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

Self-funded. If this work matters to you:

→ [GitHub Sponsors](https://github.com/sponsors/JohannaWeb)  
→ [Buy Me a Coffee](https://buymeacoffee.com/johannaweb)

MIT License · Porto, Portugal · 2026
