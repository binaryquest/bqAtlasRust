# Working on bqAtlas Rust

This repository is an MIT-licensed Rust backend and source starter for ERP applications. Its CRM demo consumes the shared Angular Atlas packages. Keep the public HTTP behavior compatible with those packages. Follow the user's current instructions when they override this guide.

## Read first

1. [README.md](README.md) for setup and the release boundary.
2. [docs/HANDOFF.md](docs/HANDOFF.md) for implemented features, limitations and next priorities.
3. [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for actual files and request flow.
4. The task's guide: [authentication](docs/AUTHENTICATION.md), [operations](docs/OPERATIONS.md), [module authoring](docs/MODULES.md), [resource extension](docs/EXTENDING-RESOURCE.md), [OData](docs/ODATA.md) or [testing](docs/TESTING.md).

[docs/IMPLEMENTATION-PLAN.md](docs/IMPLEMENTATION-PLAN.md) preserves the original proposal. It is historical context, not a list of implemented APIs. Inspect current source before changing a behavior described there.

## Find the implementation

- Framework crates are under `crates/`. Business modules are under `samples/crm/modules/` and currently keep most implementation in `src/lib.rs`.
- `samples/crm/server/src/main.rs` loads configuration and runs explicit host commands. `src/lib.rs` composes migrations, permissions, services, middleware and routes.
- `samples/crm/web/src/main.ts` registers workspace screens, CRUD features and menus. The `crm/`, `sales/`, `showcase/` and `control-docs/` folders contain application views and examples.
- UI controls and Angular framework services are pinned archives under `samples/crm/web/vendor/`. Their source belongs to the separate `binaryquest/bqAtlas` repository. Do not edit installed `node_modules` or silently replace these archives.

## Preserve these boundaries

- Each business module owns its schema, SQL, validation and public projections. Cross-module customer access uses `CustomerDirectory` in `crm-contracts`; it does not query CRM tables from Sales or Engagement.
- Support PostgreSQL and SQL Server for persistence changes. Bind values with `Value` and pair provider SQL with `Sql::new`. Allowlist query fields; never interpolate user-supplied identifiers or sort syntax.
- Append provider migration files and register their `Migration` entries explicitly. Never modify an applied migration or assume that placing a file in the migrations directory registers it.
- Preserve camelCase REST JSON, decimal strings, date semantics, ETag/If-Match preconditions, field error paths and Sales submission receipts. Money must not pass through floating point.
- Enforce permissions in handlers. Hidden menus and disabled buttons do not enforce access. Browser writes and POST queries require session CSRF protection.
- Provision accounts and permissions deliberately. Bootstrap preserves existing accounts. This alpha uses fresh Rust databases, not ASP.NET cookies, Identity hashes or a shared .NET database.
- Account recovery and OIDC logs must exclude credentials, token values and recovery links. Private `.env` and `.local-mail` files stay ignored. Public development examples are not production credentials.
- Public registry publishing remains deferred. Do not infer deployment, package publication or production database changes from a development task.

## Run and verify

Use Rust 1.90.0 from `rust-toolchain.toml` and Node 24.15 or newer. Run commands from this repository's root unless a guide states otherwise. Start only the services needed for the task; another checkout's database or preview is not this application's service.

```sh
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
node dev/verify-frontend.mjs
```

These are the baseline Rust checks. For frontend changes, run `npm --prefix samples/crm/web run check` and `npm --prefix samples/crm/web run build` after installing the locked dependencies if needed. For database adapter changes, also check both independent features as listed in [TESTING.md](docs/TESTING.md). For persistence, authentication or HTTP changes, run the relevant real-engine suite on both providers; run Keycloak qualification for OIDC behavior. The default Cargo test command intentionally skips the real-database suite.

Starter source is embedded at CLI build time. Changes to templates, composition points, docs or shared archives must be followed by rebuilding the CLI and its generator test. `crates/bqatlas-cli/build.rs` selects the embedded source. `dev/test-starter.sh` checks a generated application; invoke it with `bash`, including inside generated projects.

The dev frontend is `127.0.0.1:4300`, the Rust API is `127.0.0.1:5201`, PostgreSQL is `127.0.0.1:25433`, optional SQL Server is `127.0.0.1:21434` and optional Keycloak is `127.0.0.1:28182`. Generated starters retain these default ports; change their configuration before running two copies at once. Troubleshooting is in [OPERATIONS.md](docs/OPERATIONS.md).

## Leave a useful handoff

Report the concrete change and the checks actually run. Distinguish configured CI from a successful hosted run, default tests from real-engine tests, and a local commit from a pushed commit. Update the relevant guide and `docs/HANDOFF.md` when scope or behavior changes. Record outstanding work without claiming planned features are implemented.
