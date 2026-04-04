# Falcon Bridge

Falcon Bridge is a bridge-oriented repository for experiments connecting Bastion's AT Protocol work with ActivityPub-oriented services.

This directory is a mixed repository: it contains a Rust bridge service in `falcon-bridge/` plus copied or vendored sibling projects used during development. It is more of an integration workspace than a tidy standalone product repo.

## What Is Here

- `falcon-bridge/`: Rust bridge service with translation, relay, and resolver modules
- `ProjectFalcon/`: a colocated copy of related AT Protocol work
- `FalconPub/`: a colocated copy of related ActivityPub work
- `manifesto.md`: project framing

## Bridge Service

The Rust bridge service currently uses:

- Axum
- Tokio
- SQLx with SQLite
- Reqwest
- `k256`

The source files suggest three main responsibilities:

- translating payloads between systems
- relaying events or messages
- resolving identities and related remote references

## Running

For the Rust bridge service:

```bash
cd falcon-bridge
cargo run
```

For the nested `ProjectFalcon/` and `FalconPub/` directories, use their own READMEs and entrypoints instead of treating this repository root as the place to start everything at once.

## Notes

- This repository is better treated as an integration workspace than as a polished monorepo.
- If you want strict source-of-truth docs for the nested projects, prefer the top-level copies under `projects/ProjectFalcon` and `projects/FalconPub`.

## License

MIT
