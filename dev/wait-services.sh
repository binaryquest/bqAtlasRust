#!/usr/bin/env bash
set -euo pipefail
provider="${1:-postgresql}"
if [[ "$provider" == "postgresql" ]]; then
  for attempt in {1..60}; do
    if docker compose -f dev/compose.yaml exec -T postgres pg_isready -U bqatlas -d bqatlas_rust_dev >/dev/null 2>&1; then break; fi
    if [[ "$attempt" == 60 ]]; then exit 1; fi
    sleep 2
  done
else
  for attempt in {1..90}; do
    if docker compose -f dev/compose.yaml exec -T -e SQLCMDPASSWORD='RustAtlas-Db!2026' sqlserver /opt/mssql-tools18/bin/sqlcmd -S localhost -U sa -C -Q 'SELECT 1' >/dev/null 2>&1; then break; fi
    if [[ "$attempt" == 90 ]]; then exit 1; fi
    sleep 2
  done
fi
if [[ "${BQATLAS_TEST_OIDC_ISSUER:-}" != "" ]]; then
  for attempt in {1..90}; do
    if curl --fail --silent --max-time 3 "$BQATLAS_TEST_OIDC_ISSUER/.well-known/openid-configuration" >/dev/null; then break; fi
    if [[ "$attempt" == 90 ]]; then exit 1; fi
    sleep 2
  done
fi
