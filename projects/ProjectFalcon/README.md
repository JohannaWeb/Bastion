# ProjectFalcon

**World-first native JVM implementation of the AT Protocol.**

ProjectFalcon is the coordination and trust substrate for the Bastion ecosystem. It provides a high-performance, adversarial-resistant identity and communication layer designed for human-agent collaboration.

## The Thesis

Most decentralized identity systems suffer from bootstrapping vulnerabilities and Sybil attacks. ProjectFalcon addresses this by implementing an **Adversarial Algorithmic Trust Protocol** (see [Research Paper](https://github.com/JohannaWeb/ProjectFalcon/releases/tag/v1.0-paper)).

## Stack

- **Runtime**: Java 21 LTS
- **Framework**: Spring Boot 4.0.3
- **Crypto**: Hand-rolled ES256K / secp256k1 verification (no JVM default)
- **Database**: JPA / Hibernate with high-concurrency support
- **Identity**: Native DID:PLC and DID:WEB resolution

## Key Features

- **STT (Sovereign Trust Tracking)**: A graph-based trust model with temporal decay and Sybil resistance.
- **AT Protocol Native**: Built from the ground up to follow the AT Protocol specification without proprietary wrappers.
- **AI Agent Mesh**: Designed to give AI agents first-class identities that can sign actions and be audited.
- **High Throughput**: Optimized for the real-time requirements of decentralized social and developer coordination.

## Getting Started

### Prerequisites

- Java 21+
- Maven 3.9+

### Build

```bash
mvn clean install
```

### Run (juntos-alpha)

```bash
cd juntos-alpha
mvn spring-boot:run
```

## Vision

To be the most robust, cryptographically sound implementation of the AT Protocol in the JVM ecosystem, powering a world where trust is computed, not assigned.

## License

MIT
