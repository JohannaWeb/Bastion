# ProjectFalcon

ProjectFalcon is the AT Protocol and identity backend work that powers the Bastion ecosystem's live Juntos deployment.

This repository is primarily a Java 21 / Spring Boot codebase with supporting frontend, deployment, and research material. The sensible way to read it is as the implementation repo behind Juntos and related protocol experiments, not as some finished universal trust layer.

## What Is Here

- `juntos-alpha/`: the main Spring Boot backend module
- `juntos-web/`: the web frontend for Juntos
- `lexicons/`: AT Protocol lexicon files for the app namespace
- `k8s/`, `Dockerfile`, `docker-compose.yml`, `railway.toml`: deployment material
- `docs/` and `agent/`: research notes, paper artifacts, and product thinking

## Current Scope

ProjectFalcon currently covers:

- a Java/Spring backend for Juntos
- AT Protocol-oriented application models and lexicons
- JWT and secp256k1-related cryptographic work in the JVM stack
- local and cloud deployment paths

The repository also contains research and framing documents. They are useful context, but they are not the same thing as shipped behavior.

## Stack

- Java 21
- Spring Boot 4
- Maven
- H2 for local runtime in the current backend module
- Vite frontend in `juntos-web/`

## Build

From the project root:

```bash
mvn clean install
```

Run the backend:

```bash
cd juntos-alpha
mvn spring-boot:run
```

Frontend assets for the web client live under `juntos-web/`; use that directory's own package scripts as needed.

## Notes

- The repo includes deployment and research material alongside application code.
- Some of the broader trust and protocol claims belong to the research direction, not necessarily to the current implementation boundary.

## License

MIT
