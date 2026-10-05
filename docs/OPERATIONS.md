# Databases, migrations and deployment

Select `BQATLAS_DATABASE_PROVIDER=postgresql|sqlserver`; a deployment uses one provider. Driver features can be built independently in `bqatlas-db`; an unavailable provider fails startup. The Rust database is fresh and independent of .NET data.

PostgreSQL uses SQLx; SQL Server uses Tiberius with a bounded lease pool. All values are bound parameters. Cancellation/failed/uncommitted SQL Server operations discard their connection. Transaction control uses TDS batches (BEGIN/COMMIT/ROLLBACK), while normal DML retains affected-row counts. Exact decimal, UUID, date, UTC timestamp, rollback and cancellation behavior is checked against real engines.

`migrate` uses a provider lock and one owned transaction, recording per-module/version/provider-source checksums in `bqatlas.schema_migrations`. Missing or modified migrations fail readiness/startup. Reapplying is safe; changing an applied migration is refused. The host does not migrate automatically.

## SQL Server development

```sh
docker compose -f dev/compose.yaml --profile sqlserver up -d sqlserver
```

Example connection (development only):

```dotenv
BQATLAS_DATABASE_PROVIDER=sqlserver
BQATLAS_DATABASE_URL=Server=tcp:127.0.0.1,21434;Database=bqatlas_rust_dev;User Id=sa;Password=RustAtlas-Db!2026;TrustServerCertificate=true
```

Create that **fresh database** deliberately before migrate, using your normal SQL administration tool. Never point the demo at a .NET production database. Production uses a restricted application login and validated TLS certificates; the development profile's public SA password and trust override are not production configuration.

The compose project has its own service names, loopback ports and volumes, and uses the bridge network to avoid exhausting Docker's custom network pool. SQL Server's supported container architecture is Linux x64; local ARM64 emulation was exercised but Linux x64 CI is the release qualification environment. [Microsoft container requirements](https://learn.microsoft.com/en-us/sql/linux/quickstart-install-connect-docker)

## Production image

`Dockerfile` builds the Angular application and Rust host, then serves both from one non-root image. Framework attribution, frontend third-party notices and the dependency license inventory are included under `/app/licenses`. It does not include `.env` or development accounts. Set `BQATLAS_ENVIRONMENT=Production`, database URL/provider, listen address `0.0.0.0:5201`, identity configuration and public origin through deployment configuration. Run an explicit migration job before starting replicas; do not bootstrap or seed in Production.

Place a TLS reverse proxy in front of the same-origin application. Use the external HTTPS origin for OIDC callbacks and mail links. Secure cookies are enabled outside Development. The first alpha supports one organization/process per deployment, without multi-tenant/company isolation or rolling migration coordination beyond the database migration lock.

Health endpoints are `/health/live` and `/health/ready`. Readiness verifies required migrations and a real database query. Body, request, database-acquire and OIDC request limits are bounded. Shutdown handles Ctrl-C and Unix SIGTERM, including Docker stop. Tracing uses request IDs, method and path; query strings and credential/provider/recovery tokens are not logged.

Production email is an application-provided `AccountEmailSender`. The default CLI host enables only opt-in Development file delivery; integrate a production sender in the composition root. Back up databases and treat migrations/account provisioning as deployment operations.
