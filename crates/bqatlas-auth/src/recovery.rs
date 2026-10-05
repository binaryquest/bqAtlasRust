//! Application-configurable mail delivery; public self-registration is disabled.
use crate::{
    AuthState, hash_password, normalize_email, password_policy, random_token, verify_password,
};
use async_trait::async_trait;
use axum::{Json, extract::State, http::StatusCode};
use bqatlas_core::{AppError, new_version};
use bqatlas_db::{Migration, Sql, Value};
use bqatlas_http::{ApiJson, ApiResult, ClientAddress};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{path::PathBuf, sync::Arc};
use tower_sessions::Session;
use uuid::Uuid;

pub fn migration() -> Migration {
    Migration {
        module: "identity",
        version: 2,
        sql: Sql::new(
            include_str!("../migrations/postgresql/0002.sql"),
            include_str!("../migrations/sqlserver/0002.sql"),
        ),
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountEmail {
    pub recipient: String,
    pub subject: String,
    pub action_url: String,
}
#[async_trait]
pub trait AccountEmailSender: Send + Sync {
    async fn send(&self, message: AccountEmail) -> Result<(), AppError>;
}
/// Opt-in sink for development only. Message files contain recovery links: never commit them.
pub struct DevelopmentEmailSender {
    directory: PathBuf,
}
impl DevelopmentEmailSender {
    pub fn new(environment: &str, directory: PathBuf) -> Result<Arc<Self>, AppError> {
        if environment != "Development" {
            return Err(AppError::Forbidden);
        }
        std::fs::create_dir_all(&directory).map_err(|_| AppError::Unavailable)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))
                .map_err(|_| AppError::Unavailable)?;
        }
        Ok(Arc::new(Self { directory }))
    }
}
#[async_trait]
impl AccountEmailSender for DevelopmentEmailSender {
    async fn send(&self, message: AccountEmail) -> Result<(), AppError> {
        let path = self
            .directory
            .join(format!("{}.json", Uuid::new_v4().simple()));
        let data = serde_json::to_vec_pretty(&message).map_err(|_| AppError::Internal)?;
        tokio::task::spawn_blocking(move || {
            use std::io::Write;
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(path).map_err(|_| AppError::Unavailable)?;
            file.write_all(&data).map_err(|_| AppError::Unavailable)
        })
        .await
        .map_err(|_| AppError::Internal)?
    }
}
#[derive(Clone)]
pub struct RecoveryConfig {
    pub sender: Arc<dyn AccountEmailSender>,
    pub public_origin: String,
}
impl RecoveryConfig {
    pub fn new(
        sender: Arc<dyn AccountEmailSender>,
        origin: &str,
        development: bool,
    ) -> Result<Self, AppError> {
        let url = openidconnect::url::Url::parse(origin)
            .map_err(|_| AppError::BadRequest("Invalid account public origin".into()))?;
        if url.path() != "/"
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.host_str().is_none()
            || (url.scheme() != "https"
                && !(development
                    && url.scheme() == "http"
                    && matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))))
        {
            return Err(AppError::BadRequest(
                "Account origin requires HTTPS, or a Development loopback URL".into(),
            ));
        }
        Ok(Self {
            sender,
            public_origin: url.origin().ascii_serialization(),
        })
    }
}
#[derive(Deserialize)]
pub(super) struct EmailInput {
    email: String,
}
fn bad_token() -> AppError {
    AppError::BadRequest(
        "Unable to complete account recovery. Check the link and password requirements.".into(),
    )
}
async fn request(
    state: AuthState,
    remote: std::net::IpAddr,
    input: EmailInput,
    purpose: &str,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if state.mode != "local" {
        return Err(AppError::NotFound.into());
    }
    let config = state.recovery.as_ref().ok_or(AppError::Unavailable)?;
    state.limit(format!("recovery:{remote}")).await?;
    let email = normalize_email(&input.email).ok();
    // Equal password-work for all requests reduces account enumeration by response time.
    verify_password(
        "recovery-timing".into(),
        state.dummy_hash.as_ref().clone(),
        state.slots.clone(),
    )
    .await?;
    if let Some(email) = email {
        state
            .limit(format!(
                "recovery-email:{:x}",
                Sha256::digest(email.as_bytes())
            ))
            .await?;
        if let Some(user) = state.user(None, Some(&email)).await?.filter(|u| {
            if purpose == "reset" {
                u.confirmed
            } else {
                !u.confirmed
            }
        }) {
            let token = random_token();
            let digest = format!("{:x}", Sha256::digest(token.as_bytes()));
            let mut tx = state.db.begin().await.map_err(|e| e.application())?;
            tx.execute(
                Sql::new(
                    "DELETE FROM identity.recovery_tokens WHERE user_id=$1 AND purpose=$2",
                    "DELETE FROM [identity].recovery_tokens WHERE user_id=@P1 AND purpose=@P2",
                ),
                &[Value::Uuid(user.id), Value::Text(purpose.into())],
            )
            .await
            .map_err(|e| e.application())?;
            tx.execute(Sql::new("INSERT INTO identity.recovery_tokens(digest,user_id,purpose,security_version,expires) VALUES($1,$2,$3,$4,$5)","INSERT INTO [identity].recovery_tokens(digest,user_id,purpose,security_version,expires) VALUES(@P1,@P2,@P3,@P4,@P5)"),&[Value::Text(digest),Value::Uuid(user.id),Value::Text(purpose.into()),Value::Text(user.security_version),Value::Int(Utc::now().timestamp()+3600)]).await.map_err(|e|e.application())?;
            tx.commit().await.map_err(|e| e.application())?;
            let mut url = openidconnect::url::Url::parse(&config.public_origin)
                .map_err(|_| AppError::Internal)?;
            url.query_pairs_mut()
                .append_pair("accountAction", purpose)
                .append_pair("userId", &user.id.to_string())
                .append_pair("token", &token);
            // Same acknowledgement on delivery failure. Operator logs never include the message/link.
            if config
                .sender
                .send(AccountEmail {
                    recipient: email,
                    subject: if purpose == "reset" {
                        "Reset your password"
                    } else {
                        "Confirm your email"
                    }
                    .into(),
                    action_url: url.into(),
                })
                .await
                .is_err()
            {
                tracing::error!("Account mail delivery failed");
            }
        }
    }
    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({"message":"If the account is eligible, an email has been sent."})),
    ))
}
pub(super) async fn request_reset(
    State(state): State<AuthState>,
    ClientAddress(remote): ClientAddress,
    ApiJson(input): ApiJson<EmailInput>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    request(state, remote, input, "reset").await
}
pub(super) async fn request_confirmation(
    State(state): State<AuthState>,
    ClientAddress(remote): ClientAddress,
    ApiJson(input): ApiJson<EmailInput>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    request(state, remote, input, "confirm").await
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TokenInput {
    user_id: String,
    token: String,
    #[serde(default)]
    new_password: String,
}
async fn redeem(
    state: AuthState,
    session: Session,
    remote: std::net::IpAddr,
    input: TokenInput,
    purpose: &str,
) -> ApiResult<StatusCode> {
    if state.mode != "local" {
        return Err(AppError::NotFound.into());
    }
    state.limit(format!("redeem:{remote}")).await?;
    let id = Uuid::parse_str(&input.user_id).map_err(|_| bad_token())?;
    if input.token.len() != 64 || !input.token.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(bad_token().into());
    }
    let digest = format!("{:x}", Sha256::digest(input.token.as_bytes()));
    let hash = if purpose == "reset" {
        password_policy(&input.new_password).map_err(|_| bad_token())?;
        Some(hash_password(input.new_password, state.slots.clone()).await?)
    } else {
        None
    };
    let mut tx = state.db.begin().await.map_err(|e| e.application())?;
    let rows=tx.query(Sql::new("SELECT security_version FROM identity.recovery_tokens WHERE digest=$1 AND user_id=$2 AND purpose=$3 AND expires>$4 FOR UPDATE","SELECT security_version FROM [identity].recovery_tokens WITH (UPDLOCK,HOLDLOCK) WHERE digest=@P1 AND user_id=@P2 AND purpose=@P3 AND expires>@P4"),&[Value::Text(digest.clone()),Value::Uuid(id),Value::Text(purpose.into()),Value::Int(Utc::now().timestamp())]).await.map_err(|e|e.application())?;
    let version = rows
        .first()
        .ok_or_else(bad_token)?
        .text("security_version")
        .map_err(|e| e.application())?;
    let changed=if let Some(hash)=hash {
        tx.execute(Sql::new("UPDATE identity.users SET password_hash=$2,security_version=$3,failures=0,locked_until=0 WHERE id=$1 AND security_version=$4 AND confirmed=true","UPDATE [identity].users SET password_hash=@P2,security_version=@P3,failures=0,locked_until=0 WHERE id=@P1 AND security_version=@P4 AND confirmed=1"),&[Value::Uuid(id),Value::Text(hash),Value::Text(new_version()),Value::Text(version)]).await
    } else {
        tx.execute(Sql::new("UPDATE identity.users SET confirmed=true,security_version=$2 WHERE id=$1 AND security_version=$3 AND confirmed=false","UPDATE [identity].users SET confirmed=1,security_version=@P2 WHERE id=@P1 AND security_version=@P3 AND confirmed=0"),&[Value::Uuid(id),Value::Text(new_version()),Value::Text(version)]).await
    }.map_err(|e|e.application())?;
    if changed != 1 {
        return Err(bad_token().into());
    }
    tx.execute(
        Sql::new(
            "DELETE FROM identity.recovery_tokens WHERE user_id=$1",
            "DELETE FROM [identity].recovery_tokens WHERE user_id=@P1",
        ),
        &[Value::Uuid(id)],
    )
    .await
    .map_err(|e| e.application())?;
    tx.commit().await.map_err(|e| e.application())?;
    session.flush().await.map_err(|_| AppError::Unavailable)?;
    Ok(StatusCode::NO_CONTENT)
}
pub(super) async fn reset(
    State(state): State<AuthState>,
    session: Session,
    ClientAddress(remote): ClientAddress,
    ApiJson(input): ApiJson<TokenInput>,
) -> ApiResult<StatusCode> {
    redeem(state, session, remote, input, "reset").await
}
pub(super) async fn confirm(
    State(state): State<AuthState>,
    session: Session,
    ClientAddress(remote): ClientAddress,
    ApiJson(input): ApiJson<TokenInput>,
) -> ApiResult<StatusCode> {
    redeem(state, session, remote, input, "confirm").await
}
