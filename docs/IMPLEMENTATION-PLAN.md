# Original Rust implementation plan

Prepared 30 September 2026 before implementation. This document preserves the original design and delivery sequence. Proposed structures, dependencies and release gates below are historical context, not claims of implemented behavior. For current files and decisions, read [ARCHITECTURE.md](ARCHITECTURE.md); for delivered/deferred scope, read [HANDOFF.md](HANDOFF.md); for actual evidence, read [TESTING.md](TESTING.md).

Build a Rust backend that can run the existing Atlas CRM demo and support new ERP modules. Preserve the public HTTP behavior that the Angular application depends on, while replacing ASP.NET Core infrastructure with Rust implementations. Keep the existing bqAtlas repository as the source of shared frontend packages and the reference for compatibility.

## Scope and release boundary

The target repository is `binaryquest/bqAtlasRust`. It contains the independent Rust source alpha. The product remains MIT licensed, with a reusable framework, full-stack starter and CRUD scaffolding. Rust backend libraries will be Cargo crates; Angular libraries retain their existing npm package identities. Public registry publication remains deferred.

The first Rust release targets one organization per deployment, one backend process and one database provider per deployment. Both PostgreSQL and SQL Server must pass release tests. The first working slice can use PostgreSQL, but SQL Server feasibility is checked before framework APIs are committed.

The owner confirmed a fresh database. Use Rust-owned schemas and migrations. Importing existing .NET data, interpreting ASP.NET Identity password hashes and sharing ASP.NET cookies are separate migration work. Existing logins will establish new Rust sessions.

Retain the demo's current business scope: customers, products, quotes with child lines, opportunities and follow-up activities. Products are copied into quote lines as description and price snapshots; activities relate to customers. Accounting, stock movements, automated email/calendar actions, multi-company isolation and multi-tenant SaaS are later modules or separate designs.

## Findings from the existing project

The baseline reviewed is bqAtlas commit `f1d0d3ee1e7cbf044c5b2bf0e21ae89859b07757`. The important boundaries already exist:

| Existing boundary | Rust plan |
| --- | --- |
| Angular `AtlasApi`, REST resource and lookup providers | Keep paths, request bodies, envelopes and errors compatible. Change the development proxy target. |
| Session and permission-filtered manifest | Implement the same responses so registered views, menus and security continue to work. |
| CRM, Sales and Engagement modules | Keep module ownership and public service contracts. Sales and Engagement obtain customer information through a CRM service. |
| EF Core provider implementations | Replace with explicit module stores using PostgreSQL and SQL Server drivers. |
| Local ASP.NET Identity and external OIDC | Implement optional Rust local identity and OIDC adapters behind a common application actor. |
| OData provider delegating writes to REST | Build a separate read adapter after the REST workflow; preserve REST concurrency and commands. |
| Quote decimal fixtures and integration tests | Reuse fixtures and port behavioral cases into Rust and HTTP compatibility tests. |

The architecture document describes a broad contract target, but the checked-in portable schema currently concentrates on quote input and decimal fixtures. The initial batch must formalize session, manifest, errors, queries, lookups and record envelopes rather than assume a complete OpenAPI specification already exists.

Reference sources are [the architecture](https://github.com/binaryquest/bqAtlas/blob/f1d0d3ee1e7cbf044c5b2bf0e21ae89859b07757/docs/ARCHITECTURE.md), [HTTP contracts](https://github.com/binaryquest/bqAtlas/blob/f1d0d3ee1e7cbf044c5b2bf0e21ae89859b07757/backend/src/BqAtlas.Core/Contracts.cs), [Angular API client](https://github.com/binaryquest/bqAtlas/blob/f1d0d3ee1e7cbf044c5b2bf0e21ae89859b07757/frontend/projects/angular/src/api.ts) and [quote workflow](https://github.com/binaryquest/bqAtlas/blob/f1d0d3ee1e7cbf044c5b2bf0e21ae89859b07757/contracts/v1/README.md).

## Proposed Rust stack

| Responsibility | Proposed choice | Reason and qualification |
| --- | --- | --- |
| HTTP and asynchronous runtime | Axum, Tokio and Tower | Axum integrates with Tower middleware; suitable for composing module routers. |
| Public JSON | Serde, explicit DTOs, UUID and a date/time crate | Preserve camelCase names, null behavior, ISO dates and UTC timestamps. |
| PostgreSQL | SQLx PostgreSQL driver and pool | Use parameterized SQL and typed row mapping; compile-time checks where practical. |
| SQL Server | Tiberius behind a pool and transaction adapter | Direct SQL Server support. Pooling, cancellation and transaction cleanup require an early proof. |
| Decimal arithmetic | `rust_decimal` plus bqAtlas decimal text validation | Preserve exact arithmetic and explicit string formatting. |
| Browser sessions | `tower-sessions` with database-backed stores for both engines | Its storage interface supports custom adapters; prove persistence and revocation. |
| Local passwords | RustCrypto Argon2 using Argon2id | Store versioned PHC hashes, tune costs and bound hashing concurrency. |
| OIDC login | `openidconnect` | Discovery and Authorization Code with PKCE; integrate validation and logout explicitly. |
| Optional API bearer tokens | `jsonwebtoken` plus issuer metadata and JWKS management | Signature verification is one part of the adapter; key refresh and claim policies remain application infrastructure. |
| API documentation | Utoipa with contract fixtures | Generate OpenAPI and verify it against actual responses. |
| CLI and operations | A small Rust CLI and tracing | Explicit migration, development bootstrap, demo seed and scaffolding commands. |

These are researched recommendations, not certified dependencies. Batch 0 pins the Rust toolchain and compatible crate versions in `Cargo.lock`, verifies licenses and tests the selected features.

SQLx documents PostgreSQL, MySQL and SQLite support and states that its earlier MSSQL driver was removed. Consequently, SQLx `AnyPool` is not a solution for both required engines. Use distinct database adapters. [SQLx project documentation](https://github.com/transact-rs/sqlx)

Tiberius provides an asynchronous SQL Server client. Its documentation warns that cancellation or an incompletely consumed response can leave a connection unusable, so the pool must discard affected connections. Test this before quote transactions depend on it. [Tiberius](https://docs.rs/tiberius/latest/tiberius/), [connection cancellation](https://docs.rs/tiberius/latest/tiberius/struct.Client.html#cancellation-safety)

The other building blocks are documented in [Axum](https://docs.rs/axum/latest/axum/), [session stores](https://docs.rs/tower-sessions/latest/tower_sessions/), [Argon2](https://docs.rs/argon2/latest/argon2/), [OIDC](https://docs.rs/openidconnect/latest/openidconnect/), [decimal string serialization](https://docs.rs/rust_decimal/latest/rust_decimal/), [JWT verification](https://docs.rs/jsonwebtoken/latest/jsonwebtoken/) and [OpenAPI generation](https://docs.rs/utoipa/latest/utoipa/). A crate's features do not establish bqAtlas runtime compatibility; the batches below provide that evidence.

## Workspace and module design

Layout considered during planning. The implemented source map is in [ARCHITECTURE.md](ARCHITECTURE.md); top-level `templates/` and `tests/` directories were not created, and API bearer support remains deferred:

```text
bqAtlasRust/
  Cargo.toml                    workspace and shared dependencies
  Cargo.lock
  rust-toolchain.toml
  crates/
    bqatlas-contracts/          HTTP DTOs and serialization
    bqatlas-core/               module graph, permissions, errors, query rules
    bqatlas-http/               Axum integration and endpoint conventions
    bqatlas-auth/               local identity, sessions, OIDC and bearer adapters
    bqatlas-db/                 provider connections and migration interfaces
    bqatlas-cli/                starter and resource generation
  samples/crm/
    server/                    composition root and configuration
    modules/crm-contracts/     customer directory interface and projection
    modules/crm/               customer domain, services, endpoints and stores
    modules/sales/             quotes, lines, audit and submission receipts
    modules/engagement/        products, opportunities and activities
    web/                       Angular demo consuming shared Atlas packages
  contracts/v1/                schemas and shared HTTP fixtures
  templates/                   starter and module/resource templates
  tests/                       HTTP compatibility and starter tests
  dev/                         isolated database and Keycloak profiles
  docs/
```

The plan proposed separate domain, application, endpoint and persistence folders. The source alpha instead groups these responsibilities in each business module's `src/lib.rs`; future splitting should preserve module ownership. PostgreSQL and SQL Server stores live at the persistence boundary. Keep SQL execution and database row types out of domain services. A typed customer directory contract is shared; CRM tables and repositories are private to CRM.

The host registers modules explicitly, validates dependencies and rejects cycles or duplicate resource IDs. It selects configuration, database provider and authentication mode, then composes routers and services. Compile-time feature flags can omit an adapter; startup must reject a configured provider that the binary does not contain.

Use typed aggregate store operations such as save quote and submit quote, with a single owned transaction for header, lines, version, audit and receipt. Avoid a generic repository interface that hides transaction boundaries. New business modules normally remain application source rather than mandatory public framework crates.

## HTTP compatibility requirements

Keep contract version `1.0` while preserving its existing behavior. Capture representative responses and edge cases before porting. Compatibility compares structure and behavior, allowing independent record IDs, tokens and timestamps.

| Area | Required behavior |
| --- | --- |
| Session and metadata | `/api/v1/session`, `/api/v1/session/csrf` and `/api/v1/manifest`; same authentication mode names, permission IDs and resource metadata. |
| REST | Existing customer, quote and Engagement paths; POST query, GET record, POST create, PUT update and DELETE. |
| Lookups | Customer search and resolve, product catalog lookup; unavailable customer resolves to an explicit JSON `null`. Preserve lookup permissions and active-record rules. |
| Envelopes | `items/total/page/pageSize`, `data/version/capabilities`; preserve optional and nullable fields as observed. |
| Browser mutations | Session cookie and `X-BQATLAS-CSRF`, including POST queries and anonymous login/recovery requests. |
| Concurrency | Opaque versions with ETag and If-Match; missing precondition 428, stale version 412, business/unique conflicts 409. Match endpoint-specific not-found behavior. |
| Errors | Problem Details with existing codes and field paths such as `lines[0].quantity`; use the same representation for extractor and domain validation errors. |
| Scalars | Decimal strings, UUID keys, ISO calendar dates and UTC timestamps; never convert money through binary floating point. |
| Query rules | Resource-specific allowlists, stable key tie-breakers and current page/search/sort/filter limits. Provider defaults must not change search semantics. |
| Sales commands | UUID idempotency keys scoped to quote, actor and submission; matching retries return the immutable result even with an old version. |

Quote quantities retain scale 3 and prices scale 4. Round each line to two decimal places, half away from zero, then sum. Use checked decimal arithmetic, preserve customer snapshots, and prohibit edits/deletes after submission. Port the existing large-value fixtures and simultaneous-submit cases.

Keep frontend libraries maintained in the existing bqAtlas repository. Before npm publication, produce versioned local archives from an explicitly pinned source revision and install them into this repository's demo. Include checksums and source notices. Avoid dependencies on sibling folders or absolute paths in the starter. The demo application source can be brought over with attribution and a recorded baseline; Atlas controls and Angular services remain shared packages.

## Database ownership and migrations

Use schemas `identity`, `crm`, `sales` and `engagement`, with per-module migration ownership and provider-specific SQL. A common migration command must handle dependency order, a checksum ledger and a migration lock. Prefer an existing runner if Batch 0 proves both adapters work; otherwise implement only the small driver-neutral orchestration needed for these requirements.

Migrations are explicit deployment commands. Ordinary startup checks the required schema version. Development account creation and demo seeding are separate, idempotent commands; existing users do not receive new passwords or permissions implicitly.

Store decimals as exact numeric types with the current quote precisions, dates as calendar dates and timestamps with an explicit UTC policy. UUIDs and opaque application versions work on both engines. Updates atomically match the expected version in SQL, so two writers cannot both save from the same version.

Define text normalization, wildcard escaping, case comparison, null ordering and pagination in tests. Do not inherit incompatible database collations or concatenate user-provided field names into SQL. Unique indexes arbitrate competing customer codes and product SKUs. Map duplicate-key, deadlock and connectivity errors deliberately; do not retry an uncertain write unless its command has safe retry semantics.

Start with fresh databases named for the Rust application. Do not operate both backends as writers against the same database. A future .NET import must preserve IDs, snapshots, audit actors and receipts through an explicit export/import design.

This development machine is ARM64. PostgreSQL can be tested locally; SQL Server qualification should run in Linux x64 CI or against a supplied x64 server. Microsoft's SQL Server container support is limited to Linux on Intel/AMD x86-64. [SQL Server container requirements](https://learn.microsoft.com/en-us/sql/linux/quickstart-install-connect-docker)

## Authentication and permissions

Local mode supplies application accounts, login/logout, password change, reset and email confirmation. It is an optional identity module, not an OAuth authorization server. Use the existing frontend account forms and endpoint payloads. Hash passwords in bounded blocking tasks, with rate limiting and lockout behavior covered by tests.

Use opaque HttpOnly browser cookies and a persistent session store. Configure secure production cookies, idle/absolute expiry and session rotation at login. Bind CSRF tokens to the current browser session and invalidate old authentication state after logout, password change or reset. Preserve the frontend's session-expiry behavior. Use distinct Rust cookie names during parallel development so hosts on the same machine do not reuse incompatible sessions.

Recovery tokens are random, stored hashed, expire, are bound to user and purpose, and become unusable after success. Known and unknown email requests receive the same acknowledgement. Public registration stays disabled. The starter supplies an opt-in development mail sink and an application email-sender interface; production delivery is deployment configuration.

OIDC mode uses discovery, Authorization Code with PKCE, state and nonce validation, issuer/audience/signature/lifetime checks, and backend-managed sessions. Validate callback and post-logout URLs, use controlled HTTP timeouts and metadata/JWKS refresh, and exercise Keycloak end to end. Login configuration never silently falls back from OIDC to local passwords. OAuth2-only providers require an identity adapter or broker.

Map provider roles and API scopes explicitly to the existing permission IDs. Enforce them in every endpoint and filter the manifest. Preserve the OIDC audit actor derived from issuer plus subject; email is not an account-linking key. New role assignments are deliberate application configuration, not an automatic expansion of existing provider roles.

Planned extension, outside the current alpha: optional JWT bearer access would support API clients. Validate the API audience and issuer and map configured scopes; an invalid Authorization header cannot fall back to a valid browser cookie. Only successfully authenticated API bearer requests bypass cookie CSRF. Account endpoints retain browser authentication. Do not substitute ID tokens for API access tokens.

## OData approach

The proposed sequence was REST CRM first, followed by a documented read-only OData subset. Implementation followed that sequence. The supported adapter is now documented in [ODATA.md](ODATA.md); full protocol conformance remains separate scope.

The Rust libraries reviewed include an OData query parser and a DataFusion adapter; their published scope does not establish equivalent ASP.NET OData functionality for this ERP backend. The parser returns an expression tree and SQL renderings, while the DataFusion project's README describes experimental, limited protocol support. Treat these as feasibility inputs. Do not adopt their SQL output without field allowlists and parameter binding. [OData parser](https://docs.rs/odatav4-parser/latest/odatav4_parser/), [DataFusion adapter](https://github.com/kamu-data/datafusion-odata)

The bounded adapter should cover the Atlas provider's actual requests: `$top`, `$skip`, `$count`, `$orderby`, typed equality, `contains`, `startswith`, and the AND/OR grouping used by combined filters and search. Return the approved public projection, exact count and compatible decimal strings. Document and test any supported service document, metadata and continuation behavior. Reject unsupported expressions and excessive query complexity explicitly. `$expand`, `$apply`, writes and batch processing are later scope.

Advertise `oDataEndpoint` only after the endpoint's tests pass. If full OData compatibility is required for the initial release, make its protocol implementation and qualification a separate prerequisite before committing the delivery scope.

## Implementation batches

| Batch | Deliverable | Completion evidence |
| --- | --- | --- |
| 0. Contracts and feasibility | Inventory endpoints and fixtures; pin shared frontend baseline; prove both database drivers, migration strategy and session stores; record OData scope. | Driver tests cover exact decimals, dates, UUIDs, versions, commit/rollback and cancelled SQL Server connections. Contract fixtures cover auth, metadata, errors and records. |
| 1. Framework and customer slice | Cargo workspace, module graph, HTTP errors, PostgreSQL migrations, local login/session/CSRF/manifest and customer CRUD/lookup. Add matching SQL Server stores. | Existing Atlas customer screen performs create/search/edit/delete on both databases, with direct API permissions and stale-write tests. |
| 2. Sales aggregate | Quote header/lines, customer snapshots, totals, audit, submission receipts and concurrency. | Shared decimal fixtures, rollback, immutable submitted records and concurrent/replayed submissions pass on both engines; current quote editor works. |
| 3. Connected CRM | Products, opportunities, activities and explicit demo seed. | Customer detail, product-to-quote selection, pipeline and activity screens work using shared controls; permissions and validation match. |
| 4. Authentication completion | Account recovery/confirmation, password change, persistent session revocation, OIDC, logout and optional API bearer access. | Both database stores pass account tests; Keycloak admin, reader and unassigned users pass configured API/UI checks; negative protocol tests pass. |
| 5. OData and resource generation | Qualified bounded query adapter; Rust resource generator using existing resource-spec concepts and shared frontend generation. | Actual Atlas REST/OData providers agree on approved fixtures; a generated resource compiles and passes CRUD, query and concurrency tests on both databases. |
| 6. Starter and release qualification | Full-stack template, docs, deployment image and local package consumption. | Clean generated applications run for PostgreSQL/SQL Server with local/OIDC authentication; migration reapply/restart, demo workflows and module-authoring walkthrough pass. |

These batches were proposed delivery targets. The alpha delivered local recovery and OIDC while deferring API bearer access and general field-driven generation. Use the current handoff to assess completion rather than treating every item in this historical table as shipped. Public package publishing remains a separate owner-authorized release step.

## Testing and operations

Use unit tests for module graph, permissions, decimal grammar and totals, query validation, version rules and business transitions. HTTP tests exercise the actual Axum routes and middleware, including malformed JSON, field validation, permission denial, CSRF, ETags and idempotency. Port meaningful cases from the .NET suite rather than reproduce implementation details.

Run the same persistence and business scenarios against disposable real PostgreSQL and SQL Server databases. Include unique-value races, quote rollback, timestamp precision, session persistence and connection cancellation. Clean up only databases created by the test run. OIDC qualification includes a real disposable Keycloak profile and separate negative validation tests.

CI runs formatting, Clippy, workspace tests and explicit provider feature builds. A database matrix runs integration tests and the generated starter; frontend tests consume the pinned Atlas artifacts. Record tested toolchain, engine and package versions. The old .NET CI results are reference evidence, not Rust qualification.

Add structured tracing, request correlation, health/readiness endpoints, configuration validation, query/body/time limits and graceful shutdown. Readiness checks the database and required migrations. Log business operation identifiers without passwords, recovery tokens or provider tokens. Supply deployment guidance for a same-origin reverse proxy and TLS.

The original implementation started with driver/contracts feasibility and the customer slice, then extended the connected CRM. Current next priorities are recorded in [HANDOFF.md](HANDOFF.md). Existing data migration, broader OData conformance and public registry releases remain separately scoped work.
