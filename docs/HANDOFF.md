# Rust implementation status and next work

Reviewed 5 October 2026. This is the current source-alpha handoff for a contributor or AI agent. Start with [AGENTS.md](../AGENTS.md) and [ARCHITECTURE.md](ARCHITECTURE.md), then read the guide relevant to the task. The original implementation was committed locally as `e55e713`; that identifier refers to this repository's history, not a commit in every generated application. GitHub push and hosted CI qualification have not been recorded in this work session. Verify Git state rather than assuming this sentence remains current.

## Implemented alpha

| Area | Implemented behavior | Starting point |
| --- | --- | --- |
| Framework | Module/resource registry, dependency validation, actors, permissions, query rules, HTTP v1 DTOs and structured errors | `crates/bqatlas-core`, `bqatlas-contracts`, `bqatlas-http` |
| Databases | PostgreSQL and SQL Server transports, typed values, transactions, migration locks/checksums and readiness checks | `crates/bqatlas-db`, module migration folders, host `migrations()` |
| Identity | Persistent local sessions, Argon2id, CSRF, password change, reset/confirmation, explicit provisioning and OIDC | `crates/bqatlas-auth`, [AUTHENTICATION.md](AUTHENTICATION.md) |
| CRM | Customers, products, quotes with lines, pipeline opportunities, activities, lookups and deliberate demo seeding | `samples/crm/modules`, host `seed.rs` |
| Atlas UI | Shared workspace and controls, custom CRM/quote views, scaffolded CRUD, showcase and individual control usage | `samples/crm/web/src`, pinned `vendor/` archives |
| Queries | REST query allowlists and documented bounded read-only OData over five public resource sets | [ODATA.md](ODATA.md), `crates/bqatlas-odata` |
| Starter | Independent source application and fixed directory scaffold with optional host/view/menu wiring | `crates/bqatlas-cli`, [MODULES.md](MODULES.md) |
| Operations | Non-root application image, static hosting, health/readiness, bounded requests and graceful shutdown | `Dockerfile`, [OPERATIONS.md](OPERATIONS.md) |

This is a working alpha, not a published or generally production-qualified release. Public npm, NuGet and Cargo registry publication is deferred. NuGet remains part of the separate .NET product; Rust framework packages are Cargo crates.

## Evidence and limits

The 30 September local qualification recorded PostgreSQL, emulated SQL Server, real Keycloak, clean generated starters, frontend workflows and a production image. Exact scope and reproducible commands are in [TESTING.md](TESTING.md). On 5 October, the commit checks passed 17 default workspace tests, formatting and archive checksum verification. The default test run skips the opt-in real-database suite; it does not replace those engine results. The subsequent documentation cleanup also passed strict Clippy, the updated generator test and local link checks. The field walkthrough was compiled in a temporary starter, its three Warehouse validation tests passed, and its Angular TypeScript check passed; live field migration/HTTP qualification was not rerun. See the dated documentation evidence in TESTING.md.

CI workflows are present but no successful hosted Linux x64 run is recorded here. SQL Server's local container was exercised under ARM emulation; that is useful local evidence and does not establish supported x64 release qualification. Keep local evidence, configured jobs and hosted results separate when reporting progress.

## Work remaining

- API JWT bearer authentication. API/OData requests with Authorization headers currently return 401 instead of falling back to cookies.
- General field-driven generation, relationship scaffolding and regeneration/merge support. The existing CLI generates one fixed directory shape.
- Broader OData and OpenAPI tooling. The read adapter does not provide full protocol metadata, expansion, batch or writes; there is no complete generated OpenAPI specification.
- A production `AccountEmailSender` integration. The default host offers only explicit Development file delivery.
- Data import from .NET, shared identity compatibility, multi-company/tenant isolation, distributed deployment design, stock/accounting workflows and automated email/calendar behavior.
- Hosted CI/release qualification, registry ownership/versioning and authorized publication.

OIDC refresh-token renewal is not implemented. OIDC sessions end at the earlier of ID-token expiry and eight hours. No public account registration, MFA or administration UI is claimed. See [AUTHENTICATION.md](AUTHENTICATION.md) for the implemented security boundary.

## Recommended sequence

1. When the owner requests a push, run and inspect hosted CI, resolve actual failures and record the Linux x64 results. Run a clean generated application as part of that qualification.
2. Choose the next product capability with the owner: broader scaffolding, API clients or production email are independent workstreams. Keep the current alpha contracts stable while adding one.
3. Before a public release, qualify the intended deployment, document version compatibility and package ownership, and publish only under a separate release instruction.

These are recommendations, not authorization to publish, operate production data or implement every deferred feature. Use [EXTENDING-RESOURCE.md](EXTENDING-RESOURCE.md) for a bounded end-to-end change while learning the framework.

## Updating this handoff

After a meaningful change, update the implemented/deferred distinction and record the commands actually run with a date. Do not infer a commit, running dev server or CI result from old prose. Use `git status`, the current workflow runs and the documented health endpoints to establish current state. Generated starters inherit these docs as framework background; record their application-specific changes and qualification separately.
