# Testing and alpha qualification

The pinned build is Rust 1.90.0, with Cargo.lock; the frontend baseline is bqAtlas `f1d0d3ee1e7cbf044c5b2bf0e21ae89859b07757`, using checksummed archives under `samples/crm/web/vendor`. Node 24.15+ is required.

```sh
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo check --locked -p bqatlas-db --no-default-features --features postgresql
cargo check --locked -p bqatlas-db --no-default-features --features sqlserver
node dev/verify-frontend.mjs
npm --prefix samples/crm/web ci
npm --prefix samples/crm/web run check
npm --prefix samples/crm/web run build
```

## Real-engine qualification

Integration tests are ignored unless explicitly requested. Set a database connection that can create/drop **test databases**. Each run allocates a unique `bqatlas_rust_test_*` database and cleans up that database even when an assertion fails; it does not reset the configured Development database.

```sh
BQATLAS_TEST_DATABASE_URL=postgres://bqatlas:bqatlas-rust-dev-only@127.0.0.1:25433/bqatlas_rust_dev cargo test --locked -p bqatlas-server --test runtime -- --ignored
```

```sh
BQATLAS_TEST_PROVIDER=sqlserver BQATLAS_TEST_DATABASE_URL='Server=tcp:127.0.0.1,21434;Database=master;User Id=sa;Password=RustAtlas-Db!2026;TrustServerCertificate=true' cargo test --locked -p bqatlas-server --test runtime -- --ignored
```

To include the actual Keycloak Authorization Code/PKCE and logout flow, start the `oidc` profile and add `BQATLAS_TEST_OIDC_ISSUER=http://127.0.0.1:28182/realms/bqatlas-rust`. The provider registers the test callback at port 5202. Do not run two OIDC suites concurrently on that port.

Cases include migration reapply/checksums; session/login/manifest/permissions/CSRF; password change and session revocation; reset/confirmation hashing, purpose, expiry and one-use; customer validation/duplicates/literal wildcards/Unicode/query injection/stale and simultaneous writes; restricted lookup projections; connected CRM seed preservation; exact product/quote decimals; quote immutability and simultaneous/replayed submission; bounded OData for all sets; driver scalar round trips/rollback; cancelled SQL Server connection reuse. OIDC covers admin, reader, unassigned, consumed/tampered state and provider logout.

Generator tests refuse overwrite and verify portable source/view/menu wiring. `dev/test-starter.sh` additionally compiles a generated inventory directory, builds its frontend and runs the same real-engine suite plus generated resource CRUD/concurrency checks. It shares a target cache but never a sibling source dependency.

## Recorded local evidence (30 September 2026)

PostgreSQL 18 and the SQL Server 2022 development container both passed the complete CRM/recovery/OData/driver HTTP suite. Keycloak 26.7.4 passed the real browser-protocol flow on PostgreSQL. The SQL Server container ran via ARM64 host emulation; CI repeats on Linux x64. Clean generated inventory applications passed compilation, frontend build and real-engine CRUD/concurrency qualification on both providers. The shared frontend builds and browser checks covered local login, customer browsing, product lookup, quote save/submission and controls documentation.

The final ARM64 image ran as a non-root user and passed production account provisioning, migrations, seeded CRM reads, secure cookie/session login, frontend assets, reserved API path handling and clean SIGTERM shutdown (`bash dev/test-image.sh bqatlas-rust:alpha-local`). A temporary `cargo vendor --locked --offline` dependency feed was used for this local build after Docker's online registry/DNS fetch stalled; the Rust build step had network disabled. The shipped Dockerfile uses the ordinary locked registry build with a compiler cache.

CI configuration is included for formatting/Clippy/unit tests, frontend artifacts/build, both engines, Keycloak, generated starter and production-image build/runtime checks. No hosted CI result is recorded in this handoff. The initial source was committed locally on 5 October 2026; a local commit does not establish a push or a successful workflow run. See [HANDOFF.md](HANDOFF.md) for current scope. The source alpha is not a completed public package release; registry publication, broad generator/OData conformance, production email integration and API bearer access remain separately scoped work.

## Commit verification on 5 October 2026

Before the initial source commit, `cargo test --locked --workspace` passed 17 tests (including the generator test); the real-engine suite remained ignored. `cargo fmt --all --check`, `node dev/verify-frontend.mjs` and staged whitespace checks passed. These checks supplement the earlier real-engine evidence; they do not rerun database, Keycloak or image qualification.

## Documentation verification on 5 October 2026

The handoff cleanup passed formatting, strict Clippy, all 17 default workspace tests, shared archive checksums, whitespace checks and local Markdown link validation. The generator test confirms that new applications include AGENTS.md, the architecture/status guides and the extension walkthrough.

The Warehouse walkthrough was applied to a disposable generated starter using its actual Rust and SQL snippets. The complete generated Rust workspace compiled, all three Warehouse module tests passed (including the location example), and the modified Angular application passed its TypeScript check using the existing locked dependencies. No live migration, database HTTP suite, Keycloak, image or hosted CI run was performed for this documentation pass. The temporary extension was not added to the framework's CRM demo.
