# Opus

Opus is a Rust prototype for an identity-aware developer runtime.

The current codebase models identities, capabilities, policy evaluation, approvals, and a local event ledger. It is a demo of the runtime idea, not yet a production IDE or agent platform.

## Why I Built It

Most AI developer tools feel like chat bolted onto an editor. Opus is me pulling in the other direction: start from the execution model, then build the UI around that.

- **Identity-First**: Every developer, agent, plugin, and organization has a DID-like identity in the model.
- **Capability-Bound**: Every action request declares explicit capabilities (e.g., READ /home, EXEC shell).
- **Audit-Native**: Every request, approval, denial, and execution is recorded in a local ledger with demo authenticity markers.
- **Sovereign**: The runtime is owned by the developer, not rented from a cloud provider.

The interesting part is not autocomplete. It is whether actions are explicit, attributable, and reviewable.

## Current Implementation

This repo now has both the protocol core and a desktop shell:

- `src/domain.rs`: identity, capability, policy, and ledger primitives
- `src/app.rs`: runtime state, structured snapshots, and demo action orchestration
- `src/crypto.rs`: demo MAC helper used by the local ledger
- `src/main.rs`: CLI entrypoint that prints the trust graph and demo session ledger
- `src-tauri/src/main.rs`: Tauri backend exposing runtime snapshot and action commands
- `ui/index.html`: desktop UI for trust graph, policy, action contracts, and ledger

## What It Is Not

Opus is not yet:

- a real IDE
- a hardened security product
- a real DID/key-management implementation
- proof that the whole runtime model works outside a prototype

What it is good for right now is making the policy and approval model concrete enough to inspect and argue about.

## Why It Matters

The point is not "chat in an editor." The point is making the runtime behavior legible:

- Agents can declare who they are in the runtime model.
- Organizations can define what they may do.
- Developers can approve specific risky actions.
- Teams can see how provenance could later attach to code review, CI, and deployment.

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

## Verification

The Rust core has a small test suite:

```bash
cargo test
```

That does not prove the product thesis. It does prove the current prototype is executable and not just a README.

## Next steps

Natural next layers on top of this:

1. Replace the demo MAC with real DID methods and key management.
2. Replace the demo action buttons with actual file, patch, terminal, and model adapters.
3. Attach signed ledger entries to patches, reviews, and terminal executions.
4. Add portable trust graph sync for users, teams, and agent packages.
