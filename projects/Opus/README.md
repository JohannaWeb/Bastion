# Opus

Prototype foundation for an AI-native IDE built around sovereign identity, capability-bound execution, and signed provenance.

## Product thesis

Most AI IDEs compete on editor ergonomics and model integration. This prototype starts from a different premise:

- Every human, agent, plugin, and organization has a DID-like identity.
- Every action request declares explicit capabilities.
- Every risky action can be routed through policy and human approval.
- Every request, approval, denial, and execution is recorded as a signed event.

That is the wedge against editors that treat AI as an assistant bolted onto a text buffer.

## Current implementation

This repo now has both the protocol core and a desktop shell:

- `src/domain.rs`: identity, capability, policy, and signed ledger primitives
- `src/app.rs`: runtime state, structured snapshots, and demo action orchestration
- `src/crypto.rs`: deterministic local signing helper for event provenance
- `src/main.rs`: CLI entrypoint that prints the trust graph and demo session ledger
- `src-tauri/src/main.rs`: Tauri backend exposing runtime snapshot and action commands
- `ui/index.html`: desktop UI for trust graph, policy, action contracts, and ledger

## Why this matters

The useful differentiation is not "chat in an editor." It is a trustworthy execution model:

- Agents can prove who they are.
- Organizations can define what they may do.
- Developers can approve specific risky actions.
- Teams can carry signed provenance into code review, CI, and deployment.

## Running

CLI:

```bash
cargo run
```

Desktop shell:

```bash
npm run desktop
```

Linux desktop prerequisites for Tauri/WebKitGTK:

```bash
sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev
```

The current environment compiled the shared Rust core successfully, but the Tauri build stopped at the missing `gdk-3.0` system package boundary.

## Next steps

Natural next layers on top of this:

1. Replace the demo signer with real DID methods and key management.
2. Replace the demo action buttons with actual file, patch, terminal, and model adapters.
3. Attach signed ledger entries to patches, reviews, and terminal executions.
4. Add portable trust graph sync for users, teams, and agent packages.
