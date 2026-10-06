# Extend a directory resource

This walkthrough adds an optional `location` field to a generated Warehouse directory. It covers schema evolution, validation, REST/OData projection, the Atlas form and checks. Perform it in a new starter or your own application, not by editing the reusable CRM template. Commands assume the framework checkout's root initially, then the generated application's root after `cd`.

## Generate and wire the module

Choose a destination that does not exist:

```sh
cargo run --locked -p bqatlas-cli -- new ../warehouse-demo --name warehouse-demo
```

Create `warehouse.resource.json` in the framework checkout:

```json
{"module":"inventory","resource":"warehouses","entity":"Warehouse","title":"Warehouses"}
```

```sh
cargo run --locked -p bqatlas-cli -- directory --spec warehouse.resource.json --app ../warehouse-demo --wire
cd ../warehouse-demo
```

This creates crate `bqatlas-inventory-warehouses` under `samples/crm/modules/inventory`, with `WarehouseInput`, `WarehouseDto`, `WarehouseService` and its provider migrations. Wiring adds the host dependency, module/resource registration, migration, permissions, router, CRUD feature and menu. The REST base is `/api/v1/inventory/warehouses`; the OData set is `/odata/Warehouses`.

The spec accepts only module/resource/entity/title. Adding fields to it is not supported. The following changes are manual application code. Start with an optional field so pre-existing rows and clients that omit it remain valid.

## Append a migration for both providers

Leave `0001.sql` unchanged. Create `samples/crm/modules/inventory/migrations/postgresql/0002.sql`:

```sql
ALTER TABLE inventory.warehouses ADD COLUMN location varchar(120);
```

Create the corresponding `sqlserver/0002.sql`:

```sql
ALTER TABLE inventory.warehouses ADD location nvarchar(120) NULL;
```

In the module's `src/lib.rs`, keep its original `migration()` and add:

```rust
pub fn location_migration() -> Migration {
    Migration {
        module: "inventory",
        version: 2,
        sql: Sql::new(
            include_str!("../migrations/postgresql/0002.sql"),
            include_str!("../migrations/sqlserver/0002.sql"),
        ),
    }
}
```

In `samples/crm/server/src/lib.rs`, add `bqatlas_inventory_warehouses::location_migration()` immediately after `bqatlas_inventory_warehouses::migration()` in `migrations()`. File creation alone does not register a migration. Existing rows receive NULL and the checksum of version 1 remains valid.

## Extend input and public records

In the module, add the following field to `WarehouseInput`:

```rust
#[serde(default)]
pub location: Option<String>,
```

Add `pub location: Option<String>` to `WarehouseDto`. Preserve its existing camelCase serialization. Inside `validate`, normalize and validate before its final `errors.is_empty()` branch:

```rust
let location = input.location
    .filter(|value| !value.trim().is_empty())
    .map(|value| value.trim().to_owned());
if location.as_ref().is_some_and(|value| value.encode_utf16().count() > 120) {
    errors.insert("location".into(), vec!["Use at most 120 characters.".into()]);
}
```

Include `location` in the returned `WarehouseInput`. Use UTF-16 length like the other text rules so the SQL Server `nvarchar(120)` bound is honored. Omitting the input, explicit null, and whitespace-only input all normalize to null. Update every `WarehouseInput` literal in the generated module/tests to supply the new field.

## Bind and project the field

Update `dto(row)` to read `location` with `row.optional_text("location").map_err(|error| error.application())?`. The generated record/query SQL uses `SELECT *`, so the appended column is available after migrating.

In `WarehouseService::save`, append `Value::NullableText(value.location.clone())` to the base `values` vector after `modified_by` and before the update branch appends the expected version. This makes location parameter 10 and the conditional update version parameter 11. Replace the paired insert/update statements with these statements, keeping their existing execution and affected-row checks:

```sql
-- PostgreSQL insert
INSERT INTO inventory.warehouses
 (id,code,name,search_name,email,active,version,modified_at,modified_by,location)
VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10);

-- SQL Server insert
INSERT INTO inventory.warehouses
 (id,code,name,search_name,email,active,version,modified_at,modified_by,location)
VALUES (@P1,@P2,@P3,@P4,@P5,@P6,@P7,@P8,@P9,@P10);

-- PostgreSQL update
UPDATE inventory.warehouses
SET code=$2,name=$3,search_name=$4,email=$5,active=$6,
    version=$7,modified_at=$8,modified_by=$9,location=$10
WHERE id=$1 AND version=$11;

-- SQL Server update
UPDATE inventory.warehouses
SET code=@P2,name=@P3,search_name=@P4,email=@P5,active=@P6,
    version=@P7,modified_at=@P8,modified_by=@P9,location=@P10
WHERE id=@P1 AND version=@P11;
```

The immediate save result constructs `WarehouseDto` separately from `dto(row)`; include `location: value.location` there too. Keep the update/delete version predicates, permission checks and CSRF behavior unchanged. Do not add location to the small lookup summary unless a lookup actually needs it.

This example exposes location in OData reads. In the service's `odata` field vector, add `Field::new("Location", "location", T::String)`. The PascalCase projection will include it from the DTO. Registering a string field also allows the adapter's bounded equality, contains/startswith and ordering operations. The default comparison uses SQL `UPPER(location)` and normalized literals; a production Unicode search policy may instead need a stored normalized column selected with `.search(...)`.

Also add `"location"` to the non-lookup `validate_query` allowlist. In the query's sorting match, add `"location" => "COALESCE(location,'')"` before its catch-all arm. In `QuerySql::warehouses`, add `"location" => "UPPER(COALESCE(location,''))"` to the filter field match. Its existing non-email string normalization and bound values then handle location eq/contains/startsWith filters. Keep lookup validation unchanged. Free-text search still targets code/name; extending it is a separate explicit choice. These additions let generic list columns sort/filter the new field without submitting an unsupported field.

## Add the Atlas form field

In the module's `descriptor()` field vector, append:

```rust
FieldDescriptor::new("location", "Location", "string", false, Some(120)),
```

The generated feature uses Atlas's generic form, which reads these field descriptors from the manifest. In `samples/crm/web/src/main.ts`, extend only the generated `inventory.warehouses` feature's defaults with `location: ""`. An empty value will normalize to null on save. There is no need to duplicate a shared UI control. For a custom layout or renderer, follow the application's `crm/crm-fields.ts` and `crm/crm-views.ts` patterns.

Accounts need `inventory.warehouses.read`, `.write`, `.delete` and `.lookup` as appropriate. For a fresh application, generate the module before bootstrap so the new account receives the declared demo permissions. Existing accounts are preserved. `create-user` can provision a new account from an explicit permission file; it does not update permissions on an existing account. OIDC users need explicit entries in the configured role map. A menu item alone never grants access.

## Verify validation and the workflow

Add a module test such as:

```rust
#[test]
fn location_is_optional_trimmed_and_bounded() {
    let make_input = |location| WarehouseInput {
        code: "WH-01".into(),
        name: "Main warehouse".into(),
        email: None,
        active: true,
        location,
    };
    let valid = validate(make_input(Some(" Dock A ".into()))).expect("valid");
    assert_eq!(valid.location.as_deref(), Some("Dock A"));
    assert!(validate(make_input(Some("  ".into()))).expect("blank").location.is_none());
    assert!(validate(make_input(None)).expect("missing").location.is_none());
    let error = validate(make_input(Some("x".repeat(121)))).expect_err("too long");
    match error {
        AppError::Validation(fields) => assert!(fields.contains_key("location")),
        other => panic!("Unexpected error: {other:?}"),
    }
}
```

Update the existing generated tests' input literals. The first generated workspace build needs `cargo check --workspace` without `--locked` to add the new crate to Cargo.lock. Commit that updated lockfile; use `--locked` for subsequent checks. Then run:

```sh
cargo fmt --all
cargo check --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
node dev/verify-frontend.mjs
npm --prefix samples/crm/web ci
npm --prefix samples/crm/web run check
npm --prefix samples/crm/web run build
```

Configure `.env` and a fresh database using [README.md](../README.md). Generated Compose projects have unique names but retain the default ports, so stop the framework's matching services or change ports, proxy and environment before starting another copy. Apply migrations twice to check reapplication, then bootstrap only a new development account and serve:

```sh
cargo run --locked -p bqatlas-server -- migrate
cargo run --locked -p bqatlas-server -- migrate
cargo run --locked -p bqatlas-server -- bootstrap
cargo run --locked -p bqatlas-server -- serve
```

Start Angular separately with `npm --prefix samples/crm/web start`. Open Warehouses from the menu, create a record with `Dock A`, reload it, edit the location and save. Clear it and confirm the API returns null. Enter 121 characters and confirm a field error. Open the same record twice and confirm a stale save receives 412 rather than overwriting the first edit.

Extend the disposable real-engine suite in `samples/crm/server/tests/runtime.rs` with location create/read/update/null/length cases, including unauthorized writes, stale If-Match and OData Location projection. Its optional `BQATLAS_TEST_EXTRA_RESOURCE=inventory.warehouses` path already exercises generated-directory CRUD and concurrency. Run that suite on both providers as described in [TESTING.md](TESTING.md). For upgrade qualification, use a disposable database with version 1 data, then apply version 2 and confirm the old row survives with null location. Never reset the development database to test an upgrade.

Finish by recording actual test results in the application's handoff. Passing unit tests and a frontend build establish compilation and validation; they do not establish a live migration or HTTP round trip.
