# Architecture

## Positioning

Opus is not modeled as "an editor with a chatbot." It is modeled as a policy-enforced execution environment for humans, agents, plugins, and organizations.

In the current repository, that execution model is still a prototype.

## Core objects

### Identity

Every principal has a DID-like identifier:

- Human: developer or reviewer
- Agent: coding or review model
- Plugin: local capability provider
- Organization: issuer of trust and policy

Each identity can carry:

- issuer
- kind
- default capabilities

### Capability

Capabilities are explicit grants, not implied trust. The current prototype includes:

- `workspace.read`
- `workspace.write`
- `terminal.run`
- `network.access`
- `review.only`

### Action request

An action request is the contract boundary between an AI system and the workspace. It includes:

- actor DID
- summary
- justification
- target
- requested capabilities

This is the surface a Tauri UI would render before approval.

### Policy

Policies are owned by a human identity and classify capabilities into:

- auto-allow
- approval-required
- deny

That makes approval semantics legible instead of hidden inside agent prompts.

### Event ledger

Every state transition is recorded as a ledger event:

- request
- approval request
- approval granted
- approval rejected
- execution
- denial

The current implementation uses a demo authenticity mechanism for ledger entries. It is useful for modeling event flow, but it should not be described as production-grade signing.

## Why DID belongs here

The DID layer is useful only if it changes runtime behavior. In this architecture it does:

- identities anchor trust and issuance
- policies evaluate actions against identity grants
- events become attributable artifacts rather than opaque logs
- plugin and agent ecosystems can be filtered by issuer and reputation

Without that execution model, DID is just branding.

## Tauri integration path

The most direct next implementation step is to wrap the runtime in a Tauri shell:

1. Rust backend owns trust graph, policy engine, and ledger.
2. Frontend renders action contracts and approval prompts.
3. Terminal, file system, and model adapters become plugins with identities.
4. Patch generation, review, and command execution emit signed ledger events.

That preserves the product thesis: every AI action is identity-bound, policy-aware, and reviewable.
