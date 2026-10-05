# bqAtlas Rust

A working MIT-licensed Rust backend and full-stack starter for the bqAtlas ERP/CRM framework. The Angular Atlas controls, workspace, menus, views and permission-aware CRUD services are shared with [bqAtlas](https://github.com/binaryquest/bqAtlas).

This is an **alpha implementation**, using a fresh Rust-owned database. It includes a modular monolith, PostgreSQL and SQL Server adapters, persistent local/OIDC login, customer/product/quote/pipeline/activity workflows, bounded read-only OData, a source starter and directory CRUD scaffolding. Publishing to registries is deferred.

## Run the demo

Install Rust 1.90.0 (rustup reads `rust-toolchain.toml`), Node 24.15 or newer, and Docker. Run from this repository's root:

```sh
cp .env.example .env
docker compose -f dev/compose.yaml up -d postgres
cargo run -p bqatlas-server -- migrate
cargo run -p bqatlas-server -- bootstrap
cargo run -p bqatlas-server -- seed
cargo run -p bqatlas-server -- serve
```

In a second terminal:

```sh
npm --prefix samples/crm/web ci
npm --prefix samples/crm/web start
```

Open **http://127.0.0.1:4300**. The API is at `127.0.0.1:5201`. The example Development account is **admin@bqatlas.local / RustAtlas-Dev!2026**. These are public demonstration credentials. `bootstrap` creates a new Development account only; it preserves existing accounts, passwords and permissions. `seed` preserves existing records and edits. Startup does not automatically migrate, seed or create accounts.

The home screen contains the connected CRM, forms/control showcases and individual control usage documentation. The frontend consumes checksummed local package archives; it has no dependency on a sibling checkout.

## Create an application

```sh
cargo run -p bqatlas-cli -- new ../my-erp --name my-erp
```

The new directory contains an independent source workspace, demo, database profiles, frontend archives and instructions. It does not copy `.env`, accounts, databases, build output or Git history. Existing destinations are rejected. Follow its README to configure and run it.

For a first directory resource (code, name, contact email, active), see [module authoring](docs/MODULES.md). Custom ERP aggregates follow the Sales transaction/service pattern.

## Read and verify

- [Authentication and accounts](docs/AUTHENTICATION.md)
- [Databases, migrations and deployment](docs/OPERATIONS.md)
- [Bounded OData queries](docs/ODATA.md)
- [Starter and module authoring](docs/MODULES.md)
- [Testing and qualification](docs/TESTING.md)
- [Original implementation plan and current status](docs/IMPLEMENTATION-PLAN.md)

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm --prefix samples/crm/web run build
```

Real database tests require explicit opt-in and create/delete only disposable databases named `bqatlas_rust_test_*`. CI runs PostgreSQL and Linux x64 SQL Server, Keycloak and generated-application checks. See testing instructions for local commands and recorded evidence.

The OData adapter is a documented query subset; full OData metadata/batch/expand/write conformance is outside this alpha. API bearer tokens and a general field-driven resource generator are also outside this alpha. Local production mail is supplied through the application's `AccountEmailSender` interface. No compatibility with ASP.NET cookies, Identity hashes or an existing .NET database is claimed.
