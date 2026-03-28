# Falcon Bridge Monorepo

Welcome to the **Falcon Bridge** project, a unified platform for decentralized communication and human-AI collaboration. This repository consolidates multiple protocol implementations and a dedicated bridging service.

## Project Structure

This monorepo contains the following primary services:

### 1. [ProjectFalcon](./ProjectFalcon)
A sophisticated decentralized systems platform built on the **AT Protocol** (Bluesky).
- **Stack**: Java 25, Spring Boot 4.
- **Key Features**: Graph-based trust model (STT), native ES256K JWT verification, and AI agent mesh.

### 2. [FalconPub](./FalconPub)
A federated community platform built with **Rust** and **ActivityPub**.
- **Stack**: Rust, Axum, SQLite.
- **Key Features**: Discord-style servers and channels, ActivityPub federation (Actor, Inbox, Outbox), and high-performance async processing.

### 3. [Falcon Bridge](./falcon-bridge)
The central coordination layer that enables cross-protocol communication.
- **Stack**: Rust (Axum, reqwest, k256).
- **Responsibilities**: 
  - Two-way translation between AT Protocol and ActivityPub.
  - Stateful identity mapping (DID ↔ Actor).
  - Live relaying with cryptographic HTTP Signatures.
  - DID Resolution (did:plc, did:web).

## Getting Started

Refer to the individual service directories for specific build and installation instructions.

### Prerequisites
- **Java 25+** (for ProjectFalcon)
- **Rust 1.75+** (for FalconPub and Falcon Bridge)
- **Node.js 20+** (for frontend components)

## License

This project is licensed under the [MIT License](./LICENSE).
