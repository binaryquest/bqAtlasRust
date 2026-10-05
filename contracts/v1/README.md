# HTTP contract v1

The shared Angular baseline is recorded in `samples/crm/web/vendor/manifest.json`. Framework response/request types are explicit Serde DTOs in `crates/bqatlas-contracts`; each business module owns its record and input DTOs. Decimal values remain strings, dates are ISO calendar dates, IDs are UUIDs and nullable fields preserve the frontend contract. The shared quote schema and decimal fixtures are retained here from that MIT baseline.

`GET /api/v1/session`, `GET /api/v1/session/csrf` and the permission-filtered `GET /api/v1/manifest` connect the existing workspace. Resource metadata identifies REST query/record endpoints and qualified OData read endpoints. Query responses contain `items`, `total`, `page` and `pageSize`; records contain `data`, `version` and optional `capabilities`. ETag and If-Match carry the same opaque version.

`bqatlas-http` emits application/problem+json with stable `code` and field validation paths. HTTP compatibility is exercised against both databases in `samples/crm/server/tests/runtime.rs`, including malformed input/IDs, session rotation, authorization, preconditions, conflict responses, exact money and aggregate transitions. This is executable behavior qualification rather than a claim of a complete OpenAPI schema or full OData protocol conformance. See `docs/TESTING.md` and `docs/ODATA.md`.
