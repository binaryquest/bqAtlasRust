//! Optional local identity, OIDC and persistent browser sessions.
pub mod oidc;
pub mod recovery;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use async_trait::async_trait;
use axum::{
    Json, Router,
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use bqatlas_contracts::SessionInfo;
use bqatlas_core::{Actor, AppError, invalid, new_version};
use bqatlas_db::{Database, Migration, Sql, Value};
use bqatlas_http::{ApiError, ApiJson, ApiResult, CurrentActor};
use chrono::Utc;
pub use oidc::{OidcConfig, OidcProvider};
use rand::{RngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
};
use subtle::ConstantTimeEq;
use tokio::sync::{Mutex, Semaphore};
use tower_sessions::{
    Session, SessionStore,
    session::{Id, Record},
    session_store,
};
use uuid::Uuid;

pub fn migration() -> Migration {
    Migration {
        module: "identity",
        version: 1,
        sql: Sql::new(
            include_str!("../migrations/postgresql/0001.sql"),
            include_str!("../migrations/sqlserver/0001.sql"),
        ),
    }
}
#[derive(Clone)]
pub struct DatabaseSessionStore(pub Database);
impl std::fmt::Debug for DatabaseSessionStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DatabaseSessionStore")
            .field("provider", &self.0.provider())
            .finish()
    }
}
fn store_error(_error: impl std::fmt::Display) -> session_store::Error {
    session_store::Error::Backend("Session storage unavailable".into())
}
#[async_trait]
impl SessionStore for DatabaseSessionStore {
    async fn create(&self, record: &mut Record) -> session_store::Result<()> {
        for _ in 0..3 {
            let data = serde_json::to_string(record)
                .map_err(|e| session_store::Error::Encode(e.to_string()))?;
            let result = self
                .0
                .execute(
                    Sql::new(
                        "INSERT INTO identity.sessions(id,data,expires) VALUES($1,$2,$3)",
                        "INSERT INTO [identity].sessions(id,data,expires) VALUES(@P1,@P2,@P3)",
                    ),
                    &[
                        Value::Text(record.id.to_string()),
                        Value::Text(data),
                        Value::Int(record.expiry_date.unix_timestamp()),
                    ],
                )
                .await;
            match result {
                Ok(_) => return Ok(()),
                Err(error) if error.unique => record.id = Id::default(),
                Err(error) => return Err(store_error(error)),
            }
        }
        Err(store_error("Session ID collision"))
    }
    async fn save(&self, record: &Record) -> session_store::Result<()> {
        let data = serde_json::to_string(record)
            .map_err(|e| session_store::Error::Encode(e.to_string()))?;
        self.0
            .execute(
                Sql::new(
                    "UPDATE identity.sessions SET data=$2,expires=$3 WHERE id=$1",
                    "UPDATE [identity].sessions SET data=@P2,expires=@P3 WHERE id=@P1",
                ),
                &[
                    Value::Text(record.id.to_string()),
                    Value::Text(data),
                    Value::Int(record.expiry_date.unix_timestamp()),
                ],
            )
            .await
            .map_err(store_error)?;
        // Deliberately do not recreate a deleted session: a parallel old request must not undo logout.
        Ok(())
    }
    async fn load(&self, id: &Id) -> session_store::Result<Option<Record>> {
        let rows = self
            .0
            .query(
                Sql::new(
                    "SELECT data FROM identity.sessions WHERE id=$1 AND expires>$2",
                    "SELECT data FROM [identity].sessions WHERE id=@P1 AND expires>@P2",
                ),
                &[
                    Value::Text(id.to_string()),
                    Value::Int(Utc::now().timestamp()),
                ],
            )
            .await
            .map_err(store_error)?;
        rows.first()
            .map(|row| {
                let text = row.text("data").map_err(store_error)?;
                serde_json::from_str(&text).map_err(|e| session_store::Error::Decode(e.to_string()))
            })
            .transpose()
    }
    async fn delete(&self, id: &Id) -> session_store::Result<()> {
        self.0
            .execute(
                Sql::new(
                    "DELETE FROM identity.sessions WHERE id=$1",
                    "DELETE FROM [identity].sessions WHERE id=@P1",
                ),
                &[Value::Text(id.to_string())],
            )
            .await
            .map_err(store_error)?;
        Ok(())
    }
}
#[derive(Clone)]
pub struct AuthState {
    pub db: Database,
    slots: Arc<Semaphore>,
    dummy_hash: Arc<String>,
    rates: Arc<Mutex<HashMap<String, (i64, u32)>>>,
    pub mode: String,
    pub oidc: Option<Arc<OidcProvider>>,
    pub recovery: Option<recovery::RecoveryConfig>,
}
#[derive(Debug, Serialize, Deserialize)]
struct Ticket {
    user_id: Uuid,
    security_version: String,
    issued_at: i64,
}
struct User {
    id: Uuid,
    name: String,
    hash: String,
    permissions: BTreeSet<String>,
    security_version: String,
    confirmed: bool,
    locked_until: i64,
}
impl AuthState {
    pub async fn new(db: Database) -> Result<Self, AppError> {
        let slots = Arc::new(Semaphore::new(4));
        let dummy_hash = hash_password("bqatlas-timing-placeholder".into(), slots.clone()).await?;
        Ok(Self {
            db,
            slots,
            dummy_hash: Arc::new(dummy_hash),
            rates: Arc::new(Mutex::new(HashMap::new())),
            mode: "local".into(),
            oidc: None,
            recovery: None,
        })
    }
    async fn user(&self, id: Option<Uuid>, email: Option<&str>) -> Result<Option<User>, AppError> {
        let (sql, parameter) = if let Some(id) = id {
            (
                Sql::new(
                    "SELECT * FROM identity.users WHERE id=$1",
                    "SELECT * FROM [identity].users WHERE id=@P1",
                ),
                Value::Uuid(id),
            )
        } else {
            (
                Sql::new(
                    "SELECT * FROM identity.users WHERE email=$1",
                    "SELECT * FROM [identity].users WHERE email=@P1",
                ),
                Value::Text(email.unwrap_or_default().into()),
            )
        };
        let rows = self
            .db
            .query(sql, &[parameter])
            .await
            .map_err(|e| e.application())?;
        rows.first()
            .map(|row| {
                Ok(User {
                    id: row.uuid("id").map_err(|e| e.application())?,
                    name: row.text("name").map_err(|e| e.application())?,
                    hash: row.text("password_hash").map_err(|e| e.application())?,
                    permissions: serde_json::from_str(
                        &row.text("permissions").map_err(|e| e.application())?,
                    )
                    .map_err(|_| AppError::Internal)?,
                    security_version: row.text("security_version").map_err(|e| e.application())?,
                    confirmed: row.boolean("confirmed").map_err(|e| e.application())?,
                    locked_until: row.integer("locked_until").map_err(|e| e.application())?,
                })
            })
            .transpose()
    }
    pub async fn bootstrap(
        &self,
        environment: &str,
        email: &str,
        password: &str,
        permissions: &[String],
    ) -> Result<bool, AppError> {
        if environment != "Development" {
            return Err(AppError::Forbidden);
        }
        self.create_local_account(email, password, email, true, permissions)
            .await
    }
    /// Explicit application/administrator provisioning. Existing accounts are never changed.
    pub async fn create_local_account(
        &self,
        email: &str,
        password: &str,
        name: &str,
        confirmed: bool,
        permissions: &[String],
    ) -> Result<bool, AppError> {
        let name = name.trim();
        if name.is_empty() || name.encode_utf16().count() > 200 {
            return Err(invalid("name", "Use a name of 1–200 characters."));
        }
        let email = normalize_email(email)?;
        password_policy(password)?;
        if self.user(None, Some(&email)).await?.is_some() {
            return Ok(false);
        }
        let hash = hash_password(password.into(), self.slots.clone()).await?;
        let permissions: BTreeSet<_> = permissions.iter().cloned().collect();
        let result=self.db.execute(Sql::new("INSERT INTO identity.users(id,email,name,password_hash,permissions,security_version,confirmed) VALUES($1,$2,$3,$4,$5,$6,$7)","INSERT INTO [identity].users(id,email,name,password_hash,permissions,security_version,confirmed) VALUES(@P1,@P2,@P3,@P4,@P5,@P6,@P7)"),&[Value::Uuid(Uuid::new_v4()),Value::Text(email.clone()),Value::Text(name.into()),Value::Text(hash),Value::Text(serde_json::to_string(&permissions).map_err(|_|AppError::Internal)?),Value::Text(new_version()),Value::Bool(confirmed)]).await;
        match result {
            Ok(_) => Ok(true),
            Err(e) if e.unique => Ok(false),
            Err(e) => Err(e.application()),
        }
    }
    async fn limit(&self, key: String) -> Result<(), AppError> {
        let now = Utc::now().timestamp();
        let mut rates = self.rates.lock().await;
        rates.retain(|_, (expires, _)| *expires > now);
        if rates.len() >= 1024 && !rates.contains_key(&key) {
            return Err(AppError::RateLimited);
        }
        let bucket = rates.entry(key).or_insert((now + 60, 0));
        bucket.1 += 1;
        if bucket.1 > 10 {
            Err(AppError::RateLimited)
        } else {
            Ok(())
        }
    }
    pub async fn current(&self, session: &Session) -> Result<Option<Actor>, AppError> {
        if self.mode == "oidc" {
            return oidc::current(session).await;
        }
        let ticket: Option<Ticket> = session
            .get("local_ticket")
            .await
            .map_err(|_| AppError::Unavailable)?;
        let Some(ticket) = ticket else {
            return Ok(None);
        };
        if Utc::now().timestamp() - ticket.issued_at > 8 * 60 * 60 {
            session.flush().await.map_err(|_| AppError::Unavailable)?;
            return Ok(None);
        }
        let user = self.user(Some(ticket.user_id), None).await?;
        match user {
            Some(user) if user.confirmed && user.security_version == ticket.security_version => {
                Ok(Some(Actor {
                    id: user.id.to_string(),
                    name: user.name,
                    permissions: user.permissions,
                }))
            }
            _ => {
                session.flush().await.map_err(|_| AppError::Unavailable)?;
                Ok(None)
            }
        }
    }
}
fn normalize_email(email: &str) -> Result<String, AppError> {
    let email = email.trim().to_lowercase();
    if email.len() > 254
        || email.chars().any(char::is_whitespace)
        || email.matches('@').count() != 1
        || email.starts_with('@')
        || email.ends_with('@')
    {
        return Err(invalid("email", "Enter a valid email address."));
    }
    Ok(email)
}
pub fn password_policy(password: &str) -> Result<(), AppError> {
    if password.len() < 12
        || password.len() > 1024
        || !password.chars().any(char::is_uppercase)
        || !password.chars().any(char::is_lowercase)
        || !password.chars().any(|c| c.is_ascii_digit())
        || !password.chars().any(|c| !c.is_alphanumeric())
    {
        return Err(invalid(
            "newPassword",
            "Use at least 12 characters including upper/lowercase letters, a digit and a symbol.",
        ));
    }
    Ok(())
}
async fn hash_password(password: String, slots: Arc<Semaphore>) -> Result<String, AppError> {
    let permit = tokio::time::timeout(std::time::Duration::from_secs(2), slots.acquire_owned())
        .await
        .map_err(|_| AppError::RateLimited)?
        .map_err(|_| AppError::Internal)?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|_| AppError::Internal)
    })
    .await
    .map_err(|_| AppError::Internal)?
}
async fn verify_password(
    password: String,
    hash: String,
    slots: Arc<Semaphore>,
) -> Result<bool, AppError> {
    let permit = tokio::time::timeout(std::time::Duration::from_secs(2), slots.acquire_owned())
        .await
        .map_err(|_| AppError::RateLimited)?
        .map_err(|_| AppError::Internal)?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let parsed = PasswordHash::new(&hash).map_err(|_| AppError::Internal)?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok())
    })
    .await
    .map_err(|_| AppError::Internal)?
}
fn random_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub async fn middleware(
    State(state): State<AuthState>,
    session: Session,
    mut request: Request,
    next: Next,
) -> Response {
    let result: Result<(), AppError> = async {
        // Bearer is not enabled in this slice. Never fall back to cookies for an invalid/unsupported header.
        if (request.uri().path().starts_with("/api/")
            || request.uri().path().starts_with("/odata/"))
            && request
                .headers()
                .contains_key(axum::http::header::AUTHORIZATION)
        {
            return Err(AppError::Unauthorized);
        }
        if let Some(actor) = state.current(&session).await? {
            request.extensions_mut().insert(actor);
        }
        // Atlas submits OIDC logout as a browser form so the provider redirect navigates.
        if state.mode == "oidc"
            && request.uri().path() == "/auth/logout"
            && request.method() == axum::http::Method::POST
            && request
                .headers()
                .get(axum::http::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| v.starts_with("application/x-www-form-urlencoded"))
        {
            let body = std::mem::replace(request.body_mut(), axum::body::Body::empty());
            let bytes = axum::body::to_bytes(body, 16 * 1024)
                .await
                .map_err(|_| AppError::Csrf)?;
            let values: HashMap<String, String> =
                serde_urlencoded::from_bytes(&bytes).map_err(|_| AppError::Csrf)?;
            let token = values
                .get("__RequestVerificationToken")
                .ok_or(AppError::Csrf)?;
            request.headers_mut().insert(
                "X-BQATLAS-CSRF",
                axum::http::HeaderValue::from_str(token).map_err(|_| AppError::Csrf)?,
            );
            *request.body_mut() = axum::body::Body::from(bytes);
        }
        if !matches!(
            *request.method(),
            axum::http::Method::GET | axum::http::Method::HEAD | axum::http::Method::OPTIONS
        ) {
            let expected: Option<String> = session
                .get("csrf")
                .await
                .map_err(|_| AppError::Unavailable)?;
            let actual = request
                .headers()
                .get("X-BQATLAS-CSRF")
                .and_then(|h| h.to_str().ok())
                .unwrap_or_default();
            if !expected.is_some_and(|e| {
                e.len() == actual.len() && bool::from(e.as_bytes().ct_eq(actual.as_bytes()))
            }) {
                return Err(AppError::Csrf);
            }
        }
        Ok(())
    }
    .await;
    match result {
        Ok(()) => {
            let mut response = next.run(request).await;
            response.headers_mut().insert(
                axum::http::header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-store"),
            );
            response
        }
        Err(error) => ApiError(error).into_response(),
    }
}
pub fn router() -> Router<AuthState> {
    Router::new()
        .route("/api/v1/session", get(session_info))
        .route("/api/v1/session/csrf", get(csrf))
        .route("/auth/login", get(oidc::login).post(login))
        .route("/signin-oidc", get(oidc::callback))
        .route("/signout-callback-oidc", get(oidc::logout_callback))
        .route("/auth/logout", post(logout))
        .route("/auth/change-password", post(change_password))
        .route(
            "/auth/request-password-reset",
            post(recovery::request_reset),
        )
        .route(
            "/auth/request-confirmation",
            post(recovery::request_confirmation),
        )
        .route("/auth/reset-password", post(recovery::reset))
        .route("/auth/confirm-email", post(recovery::confirm))
}
async fn session_info(
    State(state): State<AuthState>,
    session: Session,
) -> ApiResult<Json<SessionInfo>> {
    let actor = state.current(&session).await?;
    Ok(Json(SessionInfo {
        authenticated: actor.is_some(),
        id: actor.as_ref().map(|a| a.id.clone()),
        name: actor.as_ref().map(|a| a.name.clone()),
        permissions: actor.map_or(vec![], |a| a.permissions.into_iter().collect()),
        auth_mode: state.mode,
    }))
}
async fn csrf(session: Session) -> ApiResult<Json<serde_json::Value>> {
    let token = if let Some(token) = session
        .get::<String>("csrf")
        .await
        .map_err(|_| AppError::Unavailable)?
    {
        token
    } else {
        let token = random_token();
        session
            .insert("csrf", &token)
            .await
            .map_err(|_| AppError::Unavailable)?;
        token
    };
    Ok(Json(serde_json::json!({"token":token})))
}
#[derive(Deserialize)]
struct LoginInput {
    email: String,
    password: String,
}
async fn login(
    State(state): State<AuthState>,
    session: Session,
    bqatlas_http::ClientAddress(remote): bqatlas_http::ClientAddress,
    ApiJson(input): ApiJson<LoginInput>,
) -> ApiResult<axum::http::StatusCode> {
    if state.mode != "local" {
        return Err(AppError::NotFound.into());
    }
    if input.email.len() > 254 || input.password.len() > 1024 {
        return Err(AppError::BadRequest("Email and password are required.".into()).into());
    }
    let email = input.email.trim().to_lowercase();
    state.limit(format!("login:{remote}")).await?;
    let user = state.user(None, Some(&email)).await?;
    let valid = verify_password(
        input.password,
        user.as_ref()
            .map_or_else(|| state.dummy_hash.as_ref().clone(), |u| u.hash.clone()),
        state.slots.clone(),
    )
    .await?;
    let Some(user) = user else {
        return Err(AppError::Unauthorized.into());
    };
    if user.locked_until > Utc::now().timestamp() {
        return Err(AppError::Unauthorized.into());
    }
    if !valid || !user.confirmed {
        state.db.execute(Sql::new("UPDATE identity.users SET failures=CASE WHEN failures+1>=5 THEN 0 ELSE failures+1 END, locked_until=CASE WHEN failures+1>=5 THEN $2 ELSE locked_until END WHERE id=$1", "UPDATE [identity].users SET failures=CASE WHEN failures+1>=5 THEN 0 ELSE failures+1 END, locked_until=CASE WHEN failures+1>=5 THEN @P2 ELSE locked_until END WHERE id=@P1"),&[Value::Uuid(user.id),Value::Int(Utc::now().timestamp()+900)]).await.map_err(|e|e.application())?;
        return Err(AppError::Unauthorized.into());
    }
    state
        .db
        .execute(
            Sql::new(
                "UPDATE identity.users SET failures=0,locked_until=0 WHERE id=$1",
                "UPDATE [identity].users SET failures=0,locked_until=0 WHERE id=@P1",
            ),
            &[Value::Uuid(user.id)],
        )
        .await
        .map_err(|e| e.application())?;
    session.flush().await.map_err(|_| AppError::Unavailable)?;
    session
        .insert(
            "local_ticket",
            Ticket {
                user_id: user.id,
                security_version: user.security_version,
                issued_at: Utc::now().timestamp(),
            },
        )
        .await
        .map_err(|_| AppError::Unavailable)?;
    session
        .insert("csrf", random_token())
        .await
        .map_err(|_| AppError::Unavailable)?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
async fn logout(
    State(state): State<AuthState>,
    actor: CurrentActor,
    session: Session,
) -> ApiResult<Response> {
    if state.mode == "oidc" {
        return Ok(oidc::logout(&state, actor, &session).await?.into_response());
    }
    session.flush().await.map_err(|_| AppError::Unavailable)?;
    Ok(axum::http::StatusCode::NO_CONTENT.into_response())
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PasswordInput {
    current_password: String,
    new_password: String,
}
async fn change_password(
    State(state): State<AuthState>,
    CurrentActor(actor): CurrentActor,
    session: Session,
    ApiJson(input): ApiJson<PasswordInput>,
) -> ApiResult<axum::http::StatusCode> {
    if state.mode != "local" {
        return Err(AppError::NotFound.into());
    }
    state.limit(format!("password:{}", actor.id)).await?;
    password_policy(&input.new_password)?;
    if input.current_password.len() > 1024 {
        return Err(invalid("currentPassword", "Current password is incorrect.").into());
    }
    let id = Uuid::parse_str(&actor.id).map_err(|_| AppError::Unauthorized)?;
    let user = state
        .user(Some(id), None)
        .await?
        .ok_or(AppError::Unauthorized)?;
    if !verify_password(input.current_password, user.hash, state.slots.clone()).await? {
        return Err(invalid("currentPassword", "Current password is incorrect.").into());
    }
    let hash = hash_password(input.new_password, state.slots.clone()).await?;
    let rows=state.db.execute(Sql::new("UPDATE identity.users SET password_hash=$2,security_version=$3,failures=0,locked_until=0 WHERE id=$1 AND security_version=$4","UPDATE [identity].users SET password_hash=@P2,security_version=@P3,failures=0,locked_until=0 WHERE id=@P1 AND security_version=@P4"),&[Value::Uuid(id),Value::Text(hash),Value::Text(new_version()),Value::Text(user.security_version)]).await.map_err(|e|e.application())?;
    if rows != 1 {
        return Err(AppError::Unauthorized.into());
    }
    session.flush().await.map_err(|_| AppError::Unavailable)?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn password_hashes_are_salted_and_verified() {
        let slots = Arc::new(Semaphore::new(2));
        let first = hash_password("Strong-Rust!123".into(), slots.clone())
            .await
            .expect("hash");
        let second = hash_password("Strong-Rust!123".into(), slots.clone())
            .await
            .expect("hash");
        assert_ne!(first, second);
        assert!(
            verify_password("Strong-Rust!123".into(), first.clone(), slots.clone())
                .await
                .expect("verify")
        );
        assert!(
            !verify_password("wrong".into(), first, slots)
                .await
                .expect("verify")
        );
    }
    #[test]
    fn weak_passwords_and_bad_emails_are_rejected() {
        assert!(password_policy("password1234").is_err());
        assert!(password_policy("Strong-Rust!123").is_ok());
        assert!(normalize_email("a@@b").is_err());
        assert_eq!(
            normalize_email(" Admin@Example.COM ").expect("email"),
            "admin@example.com"
        );
    }
}
