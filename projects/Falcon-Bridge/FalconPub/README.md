# FalconPub

A federated community platform built with **Rust** and **ActivityPub**.

## Stack

| Layer | Technology |
|---|---|
| Backend | Rust + Axum |
| Protocol | ActivityPub (Fediverse) |
| Crypto | Native ES256K (`k256` crate) |
| Database | SQLite via `sqlx` |
| Frontend | React + TypeScript (Vite) |

## Features

-  **ActivityPub federation** — Actor, Inbox, Outbox, WebFinger
-  **Native ES256K signing** — secp256k1 ECDSA, no JVM required
-  **Servers & Channels** — Discord-style community spaces
-  **Direct Messages** — Private conversations between users
-  **Async Rust** — Tokio + Axum for high-performance HTTP

## Getting Started

### Backend

```bash
cd falcon-rust
cargo run --release
# Listening on http://0.0.0.0:8080
```

Set `DATABASE_URL` to override the default `sqlite:falcon.db`.

### Frontend

```bash
cd falcon-web
npm install
npm run dev
```

## API Endpoints

| Method | Route | Description |
|---|---|---|
| GET | `/xrpc/app.falcon.server.list` | List servers |
| POST | `/xrpc/app.falcon.server.create` | Create server |
| GET | `/xrpc/app.falcon.server.get` | Get server |
| POST | `/xrpc/app.falcon.server.invite` | Invite member |
| GET | `/xrpc/app.falcon.channel.list` | List channels |
| POST | `/xrpc/app.falcon.channel.create` | Create channel |
| POST | `/xrpc/app.falcon.channel.postMessage` | Post message |
| GET | `/xrpc/app.falcon.channel.getMessages` | Get messages |
| GET/POST | `/xrpc/app.falcon.convo.*` | Direct messages |
| GET | `/actor/:name` | ActivityPub Actor |
| POST | `/inbox` | ActivityPub Inbox |
| GET | `/actor/:name/outbox` | ActivityPub Outbox |
| GET | `/.well-known/webfinger` | WebFinger discovery |

## License

MIT — see [LICENSE](LICENSE)
