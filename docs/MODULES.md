# Starter and module authoring

`cargo run -p bqatlas-cli -- new ../my-erp --name my-erp` generates an independent source workspace. The CLI embeds an explicit source allowlist and the pinned frontend package archives at build time. No existing path is overwritten. Rebuild the CLI after changing starter source. Registry package consumption can replace bundled framework source when publishing is authorized.

## First CRUD directory

Create `warehouse.resource.json`:

```json
{"module":"inventory","resource":"warehouses","entity":"Warehouse","title":"Warehouses"}
```

```sh
cargo run -p bqatlas-cli -- directory --spec warehouse.resource.json --app ../my-erp --wire
cd ../my-erp
cargo fmt --all
cargo test --workspace
cargo run -p bqatlas-server -- migrate
```

The initial scaffold has `code`, `name`, optional contact `email`, `active`, UUID ID, version and audit fields. It produces a typed module, both provider migrations, REST query/CRUD/lookup, OData reads and validation tests. With `--wire`, it adds the module dependency, registry, migrations, permissions, router, generic CRUD view and menu to the standard starter. Without it, compose the module yourself. Existing modules/destinations are refused; wiring validates known composition points before writing.

Permissions are resource-specific (`inventory.warehouses.read/write/delete/lookup`). Assign them deliberately to users/provider roles. Re-running bootstrap preserves an existing account; it does not silently grant newly declared permissions. Generate the module before a new Development account bootstrap, or use deliberate application provisioning. New resources appear only to readers with their read permission.

This generator is a **directory template**, not yet a general field-spec compiler. Extend generated DTOs, rules, projections and provider migrations for custom fields. Do not edit a migration already applied to a database: append a new migration and register its version. General field-driven schemas and generator upgrade/merge support are future work.

## Business modules

`bqatlas-core` owns module dependency validation, actor/permission policy, version/query rules and errors. `bqatlas-contracts` owns the HTTP v1 DTOs. HTTP, authentication, database and OData crates adapt them to infrastructure. Business modules own their schemas, SQL, public projections and services.

CRM exports a small `CustomerDirectory` contract through `crm-contracts`; Sales and Engagement use it without querying CRM tables. Keep module internals private, and define similarly narrow contracts for new module relationships. The host explicitly composes modules and rejects duplicate IDs, missing dependencies and cycles.

For an aggregate, follow Sales: checked decimal validation, child IDs, customer snapshots, a transaction covering header/lines/version/audit, conditional If-Match writes, immutable submission and a receipt scoped to aggregate/actor/idempotency key. A generic CRUD directory is not a substitute for business transitions.

The starter reuses the shared Atlas workspace, resource metadata, views, menus, field renderers, lookups and controls documentation. Add custom Angular views when a generic record editor does not express the business workflow.
