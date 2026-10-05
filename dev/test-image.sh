#!/usr/bin/env bash
set -euo pipefail
image_test_tag="${1:-bqatlas-rust:qualification}"
image_test_pg="$(docker compose -f dev/compose.yaml ps -q postgres)"
[[ -n "$image_test_pg" ]]
image_test_id="$(node -e "process.stdout.write(require('node:crypto').randomUUID().replaceAll('-', ''))")"
image_test_db="bqatlas_rust_test_image_$image_test_id"
image_test_container="bqatlas-rust-image-$image_test_id"
image_test_root="$(mktemp -d -t bqatlas-image-XXXXXX)"
image_test_created=0
cleanup() {
  docker rm -f "$image_test_container" >/dev/null 2>&1 || true
  if [[ "$image_test_created" == 1 ]]; then
    docker exec "$image_test_pg" psql -U bqatlas -d bqatlas_rust_dev -v ON_ERROR_STOP=1 -c "DROP DATABASE $image_test_db WITH (FORCE)" >/dev/null
  fi
  rm -rf "$image_test_root"
}
trap cleanup EXIT
image_test_ip="$(docker inspect --format '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' "$image_test_pg")"
[[ "$image_test_ip" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]
image_test_url="postgres://bqatlas:bqatlas-rust-dev-only@$image_test_ip:5432/$image_test_db"
docker exec "$image_test_pg" psql -U bqatlas -d bqatlas_rust_dev -v ON_ERROR_STOP=1 -c "CREATE DATABASE $image_test_db" >/dev/null
image_test_created=1
image_test_env=(--network bridge -e "BQATLAS_DATABASE_URL=$image_test_url" -e BQATLAS_DATABASE_PROVIDER=postgresql)
docker run --rm "${image_test_env[@]}" "$image_test_tag" migrate
printf '%s\n' '["crm.customers.read","crm.customers.lookup"]' > "$image_test_root/permissions.json"
chmod 644 "$image_test_root/permissions.json"
# Public throwaway credentials scoped to this disposable database, never production account data.
export BQATLAS_IMAGE_TEST_EMAIL=image-test@bqatlas.local
export BQATLAS_IMAGE_TEST_PASSWORD='RustImage-Test!2026'
docker run --rm "${image_test_env[@]}" -e "BQATLAS_ACCOUNT_EMAIL=$BQATLAS_IMAGE_TEST_EMAIL" -e "BQATLAS_ACCOUNT_PASSWORD=$BQATLAS_IMAGE_TEST_PASSWORD" -v "$image_test_root/permissions.json:/permissions.json:ro" "$image_test_tag" create-user --permissions /permissions.json --confirmed
docker run --rm "${image_test_env[@]}" -e BQATLAS_ENVIRONMENT=Development "$image_test_tag" seed
docker run -d --name "$image_test_container" "${image_test_env[@]}" -p 127.0.0.1::5201 "$image_test_tag" serve >/dev/null
image_test_port="$(docker port "$image_test_container" 5201/tcp)"
export BQATLAS_IMAGE_TEST_ORIGIN="http://$image_test_port"
for attempt in {1..60}; do
  if curl --silent --fail --max-time 3 "$BQATLAS_IMAGE_TEST_ORIGIN/health/ready" >/dev/null; then break; fi
  if [[ "$attempt" == 60 ]]; then docker logs "$image_test_container"; exit 1; fi
  sleep 1
done
node dev/verify-image.mjs
docker stop --time 10 "$image_test_container" >/dev/null
[[ "$(docker inspect --format '{{.State.ExitCode}}' "$image_test_container")" == 0 ]]
echo 'Production container exited cleanly on SIGTERM.'
