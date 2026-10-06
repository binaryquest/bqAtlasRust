# Implemented Rust architecture

This guide describes the source alpha reviewed on 5 October 2026. It maps the current files and behavior so contributors can extend the application without relying on the earlier proposal. The backend is a modular monolith with one configured database provider. The Angular frontend retains the Atlas workspace, permissions, menus and CRUD concepts shared with bqAtlas.

## Source map

| Location | Responsibility |
| --- | --- |
| [`crates/bqatlas-contracts/src/lib.rs`](../crates/bqatlas-contracts/src/lib.rs) | HTTP v1 session, manifest, resource metadata, query, record and Problem Details DTOs |
| [`crates/bqatlas-core/src/lib.rs`](../crates/bqatlas-core/src/lib.rs) | Actor permissions, module graph, query limits, validation, decimal grammar and opaque versions |
| [`crates/bqatlas-http/src/lib.rs`](../crates/bqatlas-http/src/lib.rs) | Axum JSON/path/actor extractors, error responses and record ETags |
| [`crates/bqatlas-db/src/lib.rs`](../crates/bqatlas-db/src/lib.rs) | Provider transports, typed values/rows, transactions, locks and migration checksums |
| [`crates/bqatlas-auth/src/lib.rs`](../crates/bqatlas-auth/src/lib.rs) | Persistent sessions, local accounts, actor loading, CSRF, login/logout and password change |
| [`crates/bqatlas-auth/src/oidc.rs`](../crates/bqatlas-auth/src/oidc.rs) and [`recovery.rs`](../crates/bqatlas-auth/src/recovery.rs) | OIDC protocol and account recovery/delivery adapters |
| [`crates/bqatlas-odata/src/lib.rs`](../crates/bqatlas-odata/src/lib.rs) | Bounded read-query parsing and parameterized SQL over approved projections |
| [`crates/bqatlas-cli/src/main.rs`](../crates/bqatlas-cli/src/main.rs) and [`build.rs`](../crates/bqatlas-cli/build.rs) | Source starter, fixed directory scaffold and embedded source selection |
| [`samples/crm/server/src/main.rs`](../samples/crm/server/src/main.rs) | Configuration, commands, static hosting and shutdown |
| [`samples/crm/server/src/lib.rs`](../samples/crm/server/src/lib.rs) | Application composition, module/resource registry, migrations, permissions and middleware |
| [`samples/crm/server/src/seed.rs`](../samples/crm/server/src/seed.rs) | Explicit Development demo seeding that preserves edits |
| [`samples/crm/web/src/main.ts`](../samples/crm/web/src/main.ts) | Angular shell, authentication UI, workspace, CRUD and menu registration |
| [`samples/crm/server/tests/runtime.rs`](../samples/crm/server/tests/runtime.rs) | Real HTTP/database qualification, including generated resources |

Business modules currently group endpoints, validation, service operations and provider SQL in each module's `src/lib.rs`. There are no separate top-level `templates/` or `tests/` directories. The CLI builds templates from checked-in source, and tests live with their crates or the sample server. Folder splitting can be introduced when it improves maintainability; it is not already implemented by the original plan.

## Module ownership

| Module | Owned data | Public boundary |
| --- | --- | --- |
| Identity | Local accounts, sessions and recovery tokens | Actor, authentication routes and recovery sender interfaces |
| CRM | Customer master records | Customer REST/lookup/OData and `CustomerDirectory` |
| Sales | Quote header/lines, snapshots, audit and submission receipts | Quote REST, submission command and summary OData |
| Engagement | Products, opportunities and activities | Resource REST, product lookup and public OData projections |

The CRM directory interface is in [`samples/crm/modules/crm-contracts/src/lib.rs`](../samples/crm/modules/crm-contracts/src/lib.rs). Sales and Engagement receive an `Arc<dyn CustomerDirectory>` through their service constructors. They can resolve the public customer projection without depending on CRM's stores or tables. Engagement currently groups three resources in one module; a product catalog is not an inventory or accounting ledger.

`Registry` validates dependencies, rejects duplicate IDs and filters resources by the actor's read permission. The host composes routers and its migration vector explicitly. Module dependency validation does not automatically discover routers or order migrations; additions must update the composition root.

## Browser request flow

1. Angular establishes the session through `/api/v1/session` and obtains a token from `/api/v1/session/csrf`. Local login rotates session and CSRF state. OIDC login completes in the backend and returns a browser session.
2. The dev frontend proxies `/api`, `/auth`, `/odata` and OIDC callback paths to the Rust API. Production can serve the compiled frontend from the same backend origin.
3. Request ID, tracing, timeout/body limits and database-backed session middleware wrap the routers. Authentication middleware inserts the current `Actor`, checks browser CSRF on non-read methods and rejects unsupported Authorization headers on API/OData paths.
4. Handlers use `CurrentActor`, `ApiJson` and `ApiPath` where appropriate, require operation-specific permissions, and pass explicit input to the module service. Invalid resource UUIDs and JSON use structured errors.
5. The service normalizes and validates input, checks related records, then uses module-owned SQL and transaction boundaries. The database adapter binds values and maps typed rows.
6. `bqatlas-http` returns a record envelope plus ETag, a page envelope, or Problem Details. The Atlas provider maps field errors, stale versions and capabilities into the open editor.

`POST .../query` is a read operation at the business layer, but it is still a browser POST and requires CSRF. After successful local login, clients must obtain the rotated CSRF token before another mutation. Menu filtering does not replace permissions in handlers.

## Record and aggregate behavior

REST records contain `data`, `version` and optional `capabilities`; pages contain `items`, `total`, `page` and `pageSize`. DTOs define public fields rather than exposing database rows. Dates are ISO calendar dates, timestamps use UTC and decimal values are strings. See [HTTP contract v1](../contracts/v1/README.md).

Updates/deletes require `If-Match` against the opaque version. Missing preconditions return 428 and stale versions return 412. Conditional SQL also matches the expected version, protecting against a race after the initial service read. Handlers separately enforce read and write/delete permissions.

Sales saves header, child lines, version and audit in one owned transaction. Quantities accept scale 3 and prices scale 4; each line rounds to two decimal places, half away from zero, before totals are summed. Submission becomes immutable and stores a receipt scoped to quote, actor and idempotency key. A replay returns its recorded result; adding a generic directory resource does not provide this business workflow.

OData is a separate read adapter over allowlisted public projections. It has no CSDL metadata, relational navigation, writes or batch conformance. REST remains responsible for record reads, lookups, concurrency and commands. Supported options and limits are in [ODATA.md](ODATA.md).

## Persistence and startup

`Database` selects SQLx PostgreSQL or a Tiberius SQL Server lease pool. `Sql::new` carries the provider-specific statements and `Value` carries bound parameters. Normalized search keys and escaped literal wildcards provide consistent search behavior across engines. Failed/cancelled/uncommitted SQL Server operations discard their connection rather than return uncertain state to the pool.

Schemas are `identity`, `crm`, `sales`, `engagement` and the migration ledger schema `bqatlas`. The host's `migrations()` vector selects provider SQL and registration order. `migrate` takes a provider lock, verifies recorded checksums and applies pending entries in one transaction. Files are not discovered automatically. Applied files are immutable; schema extensions append and register new versions.

`serve` checks required migrations and never migrates, creates users or seeds records implicitly. `bootstrap`, `seed` and `create-user` are explicit commands. `/health/live` reports that the HTTP process responds; `/health/ready` checks the required migration entries and executes a database query. Operational configuration and deployment limits are in [OPERATIONS.md](OPERATIONS.md).

## Shared frontend ownership

The Rust repository owns its application code under `samples/crm/web/src/`: the CRM workbench, custom quote editor, field renderers, showcase and individual control documentation. `main.ts` registers resource features and menus; `crm/crm-views.ts` and `sales/quote-views.ts` demonstrate custom business screens. Generic CRUD forms consume field descriptors from the manifest; applications can override field rendering, validation or whole views.

The controls and Angular framework services are the three local npm archives in `samples/crm/web/vendor/`, pinned to the source revision and SHA256 values in [`manifest.json`](../samples/crm/web/vendor/manifest.json). Updating a shared control requires a coordinated change in `binaryquest/bqAtlas`, rebuilt archives, an updated manifest and lockfile, and validation of the Rust demo and generated starter. This repository has no runtime dependency on a sibling checkout and does not maintain a second fork of the controls. Read [source provenance](../samples/crm/web/SOURCE-NOTICE.md) before updating those artifacts.

The generator embeds framework, demo, documentation and archive files when the CLI is built. `new` copies that source into an independent application; `directory --wire` transforms the customer directory template and edits known host/frontend registration points. It is a fixed code/name/email/active scaffold, not a general field-spec compiler. Follow [MODULES.md](MODULES.md) and [EXTENDING-RESOURCE.md](EXTENDING-RESOURCE.md).

## Design boundaries

The alpha deliberately uses a fresh Rust database, one organization, one backend process and one provider per deployment. It shares frontend behavior with .NET, not database files, Identity hashes or cookies. Local auth is an application account module, not an OAuth authorization server; OAuth2-only providers need an identity adapter or OIDC broker. API bearer tokens, full OData, field-driven generation, production mail delivery, data import and registry releases remain separate work. The current completion record and recommended priorities are in [HANDOFF.md](HANDOFF.md).
