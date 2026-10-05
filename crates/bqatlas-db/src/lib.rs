//! Database transport and migration orchestration. SQL and aggregate transactions stay module-owned.
use bqatlas_core::AppError;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sha2::{Digest, Sha256};
#[cfg(feature = "postgresql")]
use sqlx::{
    Row,
    postgres::{PgPoolOptions, PgRow},
};
#[cfg(feature = "sqlserver")]
use std::sync::Arc;
use std::time::Duration;
#[cfg(feature = "sqlserver")]
use tiberius::{Client, Config, Query};
#[cfg(feature = "sqlserver")]
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};
#[cfg(feature = "sqlserver")]
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
#[error("Database operation failed")]
pub struct DbError {
    pub unique: bool,
    source: Box<dyn std::error::Error + Send + Sync>,
}
impl DbError {
    pub fn application(self) -> AppError {
        if self.unique {
            AppError::Conflict {
                code: "duplicate_value".into(),
                message: "This value already exists.".into(),
            }
        } else {
            tracing_error(&self);
            AppError::Unavailable
        }
    }
    pub fn storage_message(&self) -> String {
        self.source.to_string()
    }
}
fn tracing_error(_error: &DbError) { /* Callers log operation names, never connection strings or parameter values. */
}
fn other(error: impl std::error::Error + Send + Sync + 'static) -> DbError {
    DbError {
        unique: false,
        source: Box::new(error),
    }
}
#[cfg(feature = "postgresql")]
impl From<sqlx::Error> for DbError {
    fn from(error: sqlx::Error) -> Self {
        Self {
            unique: error
                .as_database_error()
                .is_some_and(|e| e.is_unique_violation()),
            source: Box::new(error),
        }
    }
}
#[cfg(feature = "sqlserver")]
impl From<tiberius::error::Error> for DbError {
    fn from(error: tiberius::error::Error) -> Self {
        Self {
            unique: matches!(error.code(), Some(2601 | 2627)),
            source: Box::new(error),
        }
    }
}
#[derive(Clone, Debug)]
pub enum Value {
    Text(String),
    NullableText(Option<String>),
    Uuid(Uuid),
    Bool(bool),
    Int(i64),
    Timestamp(DateTime<Utc>),
    Date(NaiveDate),
    Decimal(Decimal),
}
#[derive(Clone, Copy)]
pub struct Sql<'a> {
    pub postgres: &'a str,
    pub sqlserver: &'a str,
}
impl<'a> Sql<'a> {
    pub fn new(postgres: &'a str, sqlserver: &'a str) -> Self {
        Self {
            postgres,
            sqlserver,
        }
    }
}
#[derive(Clone)]
pub enum Database {
    #[cfg(feature = "postgresql")]
    Postgres(sqlx::PgPool),
    #[cfg(feature = "sqlserver")]
    SqlServer(Arc<MsPool>),
}
pub enum DbRow {
    #[cfg(feature = "postgresql")]
    Postgres(PgRow),
    #[cfg(feature = "sqlserver")]
    SqlServer(tiberius::Row),
}
impl DbRow {
    pub fn text(&self, name: &str) -> Result<String, DbError> {
        match self {
            #[cfg(feature = "postgresql")]
            Self::Postgres(row) => Ok(row.try_get(name)?),
            #[cfg(feature = "sqlserver")]
            Self::SqlServer(row) => row
                .try_get::<&str, _>(name)?
                .map(str::to_owned)
                .ok_or_else(|| other(std::io::Error::other("Missing text column"))),
        }
    }
    pub fn optional_text(&self, name: &str) -> Result<Option<String>, DbError> {
        match self {
            #[cfg(feature = "postgresql")]
            Self::Postgres(row) => Ok(row.try_get(name)?),
            #[cfg(feature = "sqlserver")]
            Self::SqlServer(row) => Ok(row.try_get::<&str, _>(name)?.map(str::to_owned)),
        }
    }
    pub fn uuid(&self, name: &str) -> Result<Uuid, DbError> {
        match self {
            #[cfg(feature = "postgresql")]
            Self::Postgres(row) => Ok(row.try_get(name)?),
            #[cfg(feature = "sqlserver")]
            Self::SqlServer(row) => row
                .try_get(name)?
                .ok_or_else(|| other(std::io::Error::other("Missing UUID column"))),
        }
    }
    pub fn boolean(&self, name: &str) -> Result<bool, DbError> {
        match self {
            #[cfg(feature = "postgresql")]
            Self::Postgres(row) => Ok(row.try_get(name)?),
            #[cfg(feature = "sqlserver")]
            Self::SqlServer(row) => row
                .try_get(name)?
                .ok_or_else(|| other(std::io::Error::other("Missing boolean column"))),
        }
    }
    pub fn integer(&self, name: &str) -> Result<i64, DbError> {
        match self {
            #[cfg(feature = "postgresql")]
            Self::Postgres(row) => Ok(row.try_get(name)?),
            #[cfg(feature = "sqlserver")]
            Self::SqlServer(row) => row
                .try_get(name)?
                .ok_or_else(|| other(std::io::Error::other("Missing integer column"))),
        }
    }
    pub fn timestamp(&self, name: &str) -> Result<DateTime<Utc>, DbError> {
        match self {
            #[cfg(feature = "postgresql")]
            Self::Postgres(row) => Ok(row.try_get(name)?),
            #[cfg(feature = "sqlserver")]
            Self::SqlServer(row) => row
                .try_get::<chrono::NaiveDateTime, _>(name)?
                .map(|d| d.and_utc())
                .ok_or_else(|| other(std::io::Error::other("Missing timestamp column"))),
        }
    }
    pub fn date(&self, name: &str) -> Result<NaiveDate, DbError> {
        match self {
            #[cfg(feature = "postgresql")]
            Self::Postgres(row) => Ok(row.try_get(name)?),
            #[cfg(feature = "sqlserver")]
            Self::SqlServer(row) => row
                .try_get(name)?
                .ok_or_else(|| other(std::io::Error::other("Missing date column"))),
        }
    }
    pub fn decimal(&self, name: &str) -> Result<Decimal, DbError> {
        match self {
            #[cfg(feature = "postgresql")]
            Self::Postgres(row) => Ok(row.try_get(name)?),
            #[cfg(feature = "sqlserver")]
            Self::SqlServer(row) => row
                .try_get(name)?
                .ok_or_else(|| other(std::io::Error::other("Missing decimal column"))),
        }
    }
}
#[cfg(feature = "postgresql")]
fn pg_query<'a>(
    sql: &'a str,
    values: &[Value],
) -> sqlx::query::Query<'a, sqlx::Postgres, sqlx::postgres::PgArguments> {
    let mut query = sqlx::query(sql);
    for value in values {
        query = match value {
            Value::Text(v) => query.bind(v.clone()),
            Value::NullableText(v) => query.bind(v.clone()),
            Value::Uuid(v) => query.bind(*v),
            Value::Bool(v) => query.bind(*v),
            Value::Int(v) => query.bind(*v),
            Value::Timestamp(v) => query.bind(*v),
            Value::Date(v) => query.bind(*v),
            Value::Decimal(v) => query.bind(*v),
        };
    }
    query
}
#[cfg(feature = "sqlserver")]
fn ms_query<'a>(sql: &'a str, values: &[Value]) -> Query<'a> {
    let mut query = Query::new(sql);
    for value in values {
        match value {
            Value::Text(v) => query.bind(v.clone()),
            Value::NullableText(v) => query.bind(v.clone()),
            Value::Uuid(v) => query.bind(*v),
            Value::Bool(v) => query.bind(*v),
            Value::Int(v) => query.bind(*v),
            Value::Timestamp(v) => query.bind(v.naive_utc()),
            Value::Date(v) => query.bind(*v),
            Value::Decimal(v) => query.bind(tiberius::numeric::Numeric::new_with_scale(
                v.mantissa(),
                v.scale() as u8,
            )),
        };
    }
    query
}
impl Database {
    pub async fn connect(provider: &str, url: &str, max_connections: u32) -> Result<Self, DbError> {
        match provider {
            #[cfg(feature = "postgresql")]
            "postgresql" => Ok(Self::Postgres(
                PgPoolOptions::new()
                    .max_connections(max_connections)
                    .acquire_timeout(Duration::from_secs(10))
                    .connect(url)
                    .await?,
            )),
            #[cfg(feature = "sqlserver")]
            "sqlserver" => {
                let pool = Arc::new(MsPool {
                    config: Config::from_ado_string(url)?,
                    idle: Mutex::new(Vec::new()),
                    slots: Arc::new(Semaphore::new(max_connections as usize)),
                });
                let lease = pool.checkout().await?;
                lease.release().await;
                Ok(Self::SqlServer(pool))
            }
            _ => Err(other(std::io::Error::other(
                "Database provider is unsupported by this binary",
            ))),
        }
    }
    pub fn provider(&self) -> &'static str {
        match self {
            #[cfg(feature = "postgresql")]
            Self::Postgres(_) => "postgresql",
            #[cfg(feature = "sqlserver")]
            Self::SqlServer(_) => "sqlserver",
        }
    }
    pub async fn query(&self, sql: Sql<'_>, values: &[Value]) -> Result<Vec<DbRow>, DbError> {
        match self {
            #[cfg(feature = "postgresql")]
            Self::Postgres(pool) => Ok(pg_query(sql.postgres, values)
                .fetch_all(pool)
                .await?
                .into_iter()
                .map(DbRow::Postgres)
                .collect()),
            #[cfg(feature = "sqlserver")]
            Self::SqlServer(pool) => {
                let mut lease = pool.checkout().await?;
                let rows = ms_query(sql.sqlserver, values)
                    .query(lease.client()?)
                    .await?
                    .into_first_result()
                    .await?;
                lease.release().await;
                Ok(rows.into_iter().map(DbRow::SqlServer).collect())
            }
        }
    }
    pub async fn execute(&self, sql: Sql<'_>, values: &[Value]) -> Result<u64, DbError> {
        match self {
            #[cfg(feature = "postgresql")]
            Self::Postgres(pool) => Ok(if values.is_empty() {
                sqlx::Executor::execute(pool, sql.postgres)
                    .await?
                    .rows_affected()
            } else {
                pg_query(sql.postgres, values)
                    .execute(pool)
                    .await?
                    .rows_affected()
            }),
            #[cfg(feature = "sqlserver")]
            Self::SqlServer(pool) => {
                let mut lease = pool.checkout().await?;
                let n = ms_query(sql.sqlserver, values)
                    .execute(lease.client()?)
                    .await?
                    .total();
                lease.release().await;
                Ok(n)
            }
        }
    }
    pub async fn begin(&self) -> Result<Transaction, DbError> {
        match self {
            #[cfg(feature = "postgresql")]
            Self::Postgres(pool) => Ok(Transaction::Postgres(pool.begin().await?)),
            #[cfg(feature = "sqlserver")]
            Self::SqlServer(pool) => {
                let mut lease = pool.checkout().await?;
                lease
                    .client()?
                    .simple_query("SET XACT_ABORT ON; BEGIN TRANSACTION")
                    .await?
                    .into_results()
                    .await?;
                Ok(Transaction::SqlServer(Box::new(lease)))
            }
        }
    }
}
pub enum Transaction {
    #[cfg(feature = "postgresql")]
    Postgres(sqlx::Transaction<'static, sqlx::Postgres>),
    #[cfg(feature = "sqlserver")]
    SqlServer(Box<MsLease>),
}
impl Transaction {
    pub async fn query(&mut self, sql: Sql<'_>, values: &[Value]) -> Result<Vec<DbRow>, DbError> {
        match self {
            #[cfg(feature = "postgresql")]
            Self::Postgres(tx) => Ok(pg_query(sql.postgres, values)
                .fetch_all(&mut **tx)
                .await?
                .into_iter()
                .map(DbRow::Postgres)
                .collect()),
            #[cfg(feature = "sqlserver")]
            Self::SqlServer(lease) => Ok(ms_query(sql.sqlserver, values)
                .query(lease.client()?)
                .await?
                .into_first_result()
                .await?
                .into_iter()
                .map(DbRow::SqlServer)
                .collect()),
        }
    }
    pub async fn execute(&mut self, sql: Sql<'_>, values: &[Value]) -> Result<u64, DbError> {
        match self {
            #[cfg(feature = "postgresql")]
            Self::Postgres(tx) => Ok(if values.is_empty() {
                sqlx::Executor::execute(&mut **tx, sql.postgres)
                    .await?
                    .rows_affected()
            } else {
                pg_query(sql.postgres, values)
                    .execute(&mut **tx)
                    .await?
                    .rows_affected()
            }),
            #[cfg(feature = "sqlserver")]
            Self::SqlServer(lease) => Ok(ms_query(sql.sqlserver, values)
                .execute(lease.client()?)
                .await?
                .total()),
        }
    }
    pub async fn commit(self) -> Result<(), DbError> {
        match self {
            #[cfg(feature = "postgresql")]
            Self::Postgres(tx) => {
                tx.commit().await?;
                Ok(())
            }
            #[cfg(feature = "sqlserver")]
            Self::SqlServer(lease) => {
                let mut lease = *lease;
                lease
                    .client()?
                    .simple_query("COMMIT TRANSACTION")
                    .await?
                    .into_results()
                    .await?;
                lease.release().await;
                Ok(())
            }
        }
    }
    pub async fn rollback(self) -> Result<(), DbError> {
        match self {
            #[cfg(feature = "postgresql")]
            Self::Postgres(tx) => {
                tx.rollback().await?;
                Ok(())
            }
            #[cfg(feature = "sqlserver")]
            Self::SqlServer(lease) => {
                let mut lease = *lease;
                lease
                    .client()?
                    .simple_query("ROLLBACK TRANSACTION")
                    .await?
                    .into_results()
                    .await?;
                lease.release().await;
                Ok(())
            }
        }
    }
}
#[cfg(feature = "sqlserver")]
type MsClient = Client<Compat<tokio::net::TcpStream>>;
#[cfg(feature = "sqlserver")]
pub struct MsPool {
    config: Config,
    idle: Mutex<Vec<MsClient>>,
    slots: Arc<Semaphore>,
}
#[cfg(feature = "sqlserver")]
pub struct MsLease {
    client: Option<MsClient>,
    pool: Arc<MsPool>,
    _permit: OwnedSemaphorePermit,
}
#[cfg(feature = "sqlserver")]
impl MsPool {
    async fn checkout(self: &Arc<Self>) -> Result<MsLease, DbError> {
        let permit =
            tokio::time::timeout(Duration::from_secs(10), self.slots.clone().acquire_owned())
                .await
                .map_err(other)?
                .map_err(other)?;
        let idle = self.idle.lock().await.pop();
        let client = if let Some(client) = idle {
            client
        } else {
            let tcp = tokio::time::timeout(
                Duration::from_secs(10),
                tokio::net::TcpStream::connect(self.config.get_addr()),
            )
            .await
            .map_err(other)?
            .map_err(other)?;
            tcp.set_nodelay(true).map_err(other)?;
            tokio::time::timeout(
                Duration::from_secs(10),
                Client::connect(self.config.clone(), tcp.compat_write()),
            )
            .await
            .map_err(other)??
        };
        Ok(MsLease {
            client: Some(client),
            pool: self.clone(),
            _permit: permit,
        })
    }
}
#[cfg(feature = "sqlserver")]
impl MsLease {
    fn client(&mut self) -> Result<&mut MsClient, DbError> {
        self.client
            .as_mut()
            .ok_or_else(|| other(std::io::Error::other("Connection has been released")))
    }
    // Drop discards the connection. Only complete, successful operations return it to the pool.
    // Cancellation or a failed/uncommitted transaction therefore cannot poison the next request.
    async fn release(mut self) {
        if let Some(client) = self.client.take() {
            self.pool.idle.lock().await.push(client);
        }
    }
}
pub struct Migration {
    pub module: &'static str,
    pub version: i64,
    pub sql: Sql<'static>,
}
impl Database {
    pub async fn migrate(&self, migrations: &[Migration]) -> Result<(), DbError> {
        let mut tx = self.begin().await?;
        tx.execute(Sql::new("SELECT pg_advisory_xact_lock(728113001)","DECLARE @result int; EXEC @result = sp_getapplock @Resource=N'bqatlas.migrations', @LockMode=N'Exclusive', @LockOwner=N'Transaction', @LockTimeout=10000; IF @result < 0 THROW 51000, 'Unable to acquire migration lock', 1;"),&[]).await?;
        tx.execute(Sql::new("CREATE SCHEMA IF NOT EXISTS bqatlas; CREATE TABLE IF NOT EXISTS bqatlas.schema_migrations (module text NOT NULL, version bigint NOT NULL, checksum text NOT NULL, PRIMARY KEY(module,version))", "IF SCHEMA_ID('bqatlas') IS NULL EXEC('CREATE SCHEMA bqatlas'); IF OBJECT_ID('bqatlas.schema_migrations') IS NULL CREATE TABLE bqatlas.schema_migrations(module nvarchar(64) NOT NULL, version bigint NOT NULL, checksum nvarchar(64) NOT NULL, CONSTRAINT pk_schema_migrations PRIMARY KEY(module,version))"),&[]).await?;
        for migration in migrations {
            let checksum = format!(
                "{:x}",
                Sha256::digest(if self.provider() == "postgresql" {
                    migration.sql.postgres
                } else {
                    migration.sql.sqlserver
                })
            );
            let key = [
                Value::Text(migration.module.into()),
                Value::Int(migration.version),
            ];
            let rows=tx.query(Sql::new("SELECT checksum FROM bqatlas.schema_migrations WHERE module=$1 AND version=$2","SELECT checksum FROM bqatlas.schema_migrations WHERE module=@P1 AND version=@P2"),&key).await?;
            if let Some(row) = rows.first() {
                if row.text("checksum")? != checksum {
                    return Err(other(std::io::Error::other(
                        "An applied migration checksum changed",
                    )));
                }
                continue;
            }
            tx.execute(migration.sql, &[]).await?;
            tx.execute(Sql::new("INSERT INTO bqatlas.schema_migrations(module,version,checksum) VALUES($1,$2,$3)","INSERT INTO bqatlas.schema_migrations(module,version,checksum) VALUES(@P1,@P2,@P3)"),&[key[0].clone(),key[1].clone(),Value::Text(checksum)]).await?;
        }
        tx.commit().await
    }
    pub async fn check_migrations(&self, migrations: &[Migration]) -> Result<(), DbError> {
        let rows = self
            .query(
                Sql::new(
                    "SELECT module,version,checksum FROM bqatlas.schema_migrations",
                    "SELECT module,version,checksum FROM bqatlas.schema_migrations",
                ),
                &[],
            )
            .await?;
        for migration in migrations {
            let checksum = format!(
                "{:x}",
                Sha256::digest(if self.provider() == "postgresql" {
                    migration.sql.postgres
                } else {
                    migration.sql.sqlserver
                })
            );
            let mut found = false;
            for row in &rows {
                if row.text("module")? == migration.module
                    && row.integer("version")? == migration.version
                    && row.text("checksum")? == checksum
                {
                    found = true;
                }
            }
            if !found {
                return Err(other(std::io::Error::other(
                    "Database migrations are missing or changed; run migrate",
                )));
            }
        }
        Ok(())
    }
}
