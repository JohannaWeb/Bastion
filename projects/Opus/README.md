# Opus — Sovereign Developer Runtime

Opus is the flagship developer runtime for the **Bastion sovereign developer stack**. It provides a capability-bound execution environment built around identity, approvals, and signed provenance.

## The Thesis

Most AI developer tools are wrappers around editors they do not control. Opus takes the opposite approach: the editor is a policy-enforced window into a trusted runtime.

- **Identity-First**: Every developer, agent, plugin, and organization has a verifiable DID-like identity.
- **Capability-Bound**: Every action request declares explicit capabilities (e.g., READ /home, EXEC shell).
- **Audit-Native**: Every request, approval, denial, and execution is recorded as a signed event in a local ledger.
- **Sovereign**: The runtime is owned by the developer, not rented from a cloud provider.

That is the wedge: not just AI autocomplete, but a trustworthy execution model for a world where humans and agents work together.

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
