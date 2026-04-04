# FalconPub

FalconPub is an ActivityPub-oriented application split into a Rust backend and a web frontend.

This repository is an implementation experiment around federation, community spaces, and messaging. The safe claim is that it contains a real Rust service and frontend scaffold, not that every protocol surface is fully complete.

## Layout

- `falcon-rust/`: Rust backend built with Axum and SQLx
- `falcon-web/`: React/Vite frontend

## Backend Stack

- Rust
- Axum
- Tokio
- SQLx with SQLite
- `k256` for secp256k1-related crypto work

## Frontend Stack

- React
- TypeScript
- Vite

## Running

Backend:

```bash
cd falcon-rust
cargo run
```

Frontend:

```bash
cd falcon-web
npm install
npm run dev
```

If you need a custom database path for the backend, set `DATABASE_URL`.

## Current Scope

Based on the repository layout and source files, FalconPub currently covers:

- a Rust HTTP service with API and crypto modules
- a SQLite schema
- a separate web client

The old README listed a large set of specific routes and features. Unless you have recently verified each one against the running service, it is better to treat those as implementation goals rather than guaranteed surface area.

## License

MIT
