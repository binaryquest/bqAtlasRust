#!/usr/bin/env bash
set -euo pipefail
starter_source_root="$(pwd)"
starter_test_root="$(mktemp -d -t bqatlas-starter-XXXXXX)"
trap 'rm -rf "$starter_test_root"' EXIT
export CARGO_TARGET_DIR="$starter_source_root/target"
cargo run --locked -p bqatlas-cli -- new "$starter_test_root/app" --name qualification-app
cat > "$starter_test_root/directory.json" <<'JSON'
{"module":"inventory","resource":"warehouses","entity":"Warehouse","title":"Warehouses"}
JSON
cargo run --locked -p bqatlas-cli -- directory --spec "$starter_test_root/directory.json" --app "$starter_test_root/app" --wire
cargo fmt --manifest-path "$starter_test_root/app/Cargo.toml" --all
cargo check --manifest-path "$starter_test_root/app/Cargo.toml" --workspace
npm --prefix "$starter_test_root/app/samples/crm/web" ci
npm --prefix "$starter_test_root/app/samples/crm/web" run build
if [[ "${BQATLAS_TEST_DATABASE_URL:-}" != "" ]]; then
  BQATLAS_TEST_EXTRA_RESOURCE=inventory.warehouses cargo test --manifest-path "$starter_test_root/app/Cargo.toml" -p bqatlas-server --test runtime -- --ignored
fi
