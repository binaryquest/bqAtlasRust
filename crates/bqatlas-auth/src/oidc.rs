//! Browser Authorization Code + PKCE. Provider tokens remain in server-side sessions.
use crate::{AuthState, random_token};
use axum::{
    extract::{Query, State},
    response::Redirect,
};
use bqatlas_core::{Actor, AppError};
use bqatlas_http::{ApiResult, CurrentActor};
use chrono::Utc;
use openidconnect::{core::*, *};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::Duration,
};
use subtle::ConstantTimeEq;
use tokio::sync::Mutex;
use tower_sessions::Session;

type RoleToken = StandardTokenResponse<
    IdTokenFields<
        Roles,
        EmptyExtraTokenFields,
        CoreGenderClaim,
        CoreJweContentEncryptionAlgorithm,
        CoreJwsSigningAlgorithm,
    >,
    CoreTokenType,
>;
type RoleClient = Client<
    Roles,
    CoreAuthDisplay,
    CoreGenderClaim,
    CoreJweContentEncryptionAlgorithm,
    CoreJsonWebKey,
    CoreAuthPrompt,
    StandardErrorResponse<CoreErrorResponseType>,
    RoleToken,
    CoreTokenIntrospectionResponse,
    CoreRevocableToken,
    CoreRevocationErrorResponse,
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct Roles {
    #[serde(default)]
    roles: Vec<String>,
    role: Option<String>,
    realm_access: Option<RoleList>,
    #[serde(default)]
    resource_access: BTreeMap<String, RoleList>,
}
impl AdditionalClaims for Roles {}
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct RoleList {
    #[serde(default)]
    roles: Vec<String>,
}
impl Roles {
    fn permissions(
        &self,
        client: &str,
        mapping: &BTreeMap<String, Vec<String>>,
    ) -> BTreeSet<String> {
        self.roles
            .iter()
            .chain(self.role.iter())
            .chain(self.realm_access.iter().flat_map(|r| &r.roles))
            .chain(
                self.resource_access
                    .get(client)
                    .into_iter()
                    .flat_map(|r| &r.roles),
            )
            .filter_map(|role| mapping.get(role))
            .flatten()
            .cloned()
            .collect()
    }
}
/// Application-supplied configuration. No implicit admin/reader role expansion.
pub struct OidcConfig {
    pub issuer: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub public_origin: String,
    pub role_permissions: BTreeMap<String, Vec<String>>,
    pub development: bool,
}
struct Discovery {
    client: RoleClient,
    logout: Option<EndSessionUrl>,
    loaded_at: i64,
}
pub struct OidcProvider {
    config: OidcConfig,
    http: reqwest::Client,
    discovery: Mutex<Option<Discovery>>,
}
fn valid_url(value: &str, development: bool) -> Result<openidconnect::url::Url, AppError> {
    let url = openidconnect::url::Url::parse(value)
        .map_err(|_| AppError::BadRequest("Invalid OIDC URL configuration".into()))?;
    if url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || (url.scheme() != "https" && !(development && url.scheme() == "http"))
    {
        return Err(AppError::BadRequest(
            "OIDC requires absolute HTTPS URLs (HTTP is allowed only in Development)".into(),
        ));
    }
    Ok(url)
}
impl OidcProvider {
    pub async fn connect(
        mut config: OidcConfig,
        allowed_permissions: &[String],
    ) -> Result<Arc<Self>, AppError> {
        valid_url(&config.issuer, config.development)?;
        let origin = valid_url(&config.public_origin, config.development)?;
        if origin.path() != "/"
            || config.client_id.trim().is_empty()
            || config
                .role_permissions
                .values()
                .flatten()
                .any(|p| !allowed_permissions.contains(p))
        {
            return Err(AppError::BadRequest(
                "Invalid OIDC origin, client or role permission mapping".into(),
            ));
        }
        config.public_origin = origin.origin().ascii_serialization();
        let provider = Arc::new(Self {
            config,
            http: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(10))
                .build()
                .map_err(|_| AppError::Internal)?,
            discovery: Mutex::new(None),
        });
        provider.discover().await?;
        Ok(provider)
    }
    async fn discover(&self) -> Result<(RoleClient, Option<EndSessionUrl>), AppError> {
        let mut cached = self.discovery.lock().await;
        if let Some(found) = cached
            .as_ref()
            .filter(|d| Utc::now().timestamp() - d.loaded_at < 60)
        {
            return Ok((found.client.clone(), found.logout.clone()));
        }
        let metadata = ProviderMetadataWithLogout::discover_async(
            IssuerUrl::new(self.config.issuer.clone()).map_err(|_| AppError::Internal)?,
            &self.http,
        )
        .await
        .map_err(|_| AppError::Unavailable)?;
        // Reject insecure discovered endpoints too, not only the configured issuer.
        valid_url(
            metadata.authorization_endpoint().as_str(),
            self.config.development,
        )?;
        let endpoint = metadata.token_endpoint().ok_or(AppError::Unavailable)?;
        valid_url(endpoint.as_str(), self.config.development)?;
        valid_url(metadata.jwks_uri().as_str(), self.config.development)?;
        let logout = metadata.additional_metadata().end_session_endpoint.clone();
        if let Some(url) = &logout {
            valid_url(url.as_str(), self.config.development)?;
        }
        let client = RoleClient::from_provider_metadata(
            metadata,
            ClientId::new(self.config.client_id.clone()),
            self.config.client_secret.clone().map(ClientSecret::new),
        )
        .set_redirect_uri(
            RedirectUrl::new(format!("{}/signin-oidc", self.config.public_origin))
                .map_err(|_| AppError::Internal)?,
        );
        *cached = Some(Discovery {
            client: client.clone(),
            logout: logout.clone(),
            loaded_at: Utc::now().timestamp(),
        });
        Ok((client, logout))
    }
}
#[derive(Serialize, Deserialize)]
struct PendingLogin {
    state: String,
    nonce: String,
    verifier: String,
    issued_at: i64,
}
#[derive(Serialize, Deserialize)]
struct ExternalTicket {
    actor: Actor,
    expires_at: i64,
    id_token: String,
}
pub(super) async fn current(session: &Session) -> Result<Option<Actor>, AppError> {
    let ticket: Option<ExternalTicket> = session
        .get("oidc_ticket")
        .await
        .map_err(|_| AppError::Unavailable)?;
    match ticket {
        Some(t) if t.expires_at > Utc::now().timestamp() => Ok(Some(t.actor)),
        Some(_) => {
            session.flush().await.map_err(|_| AppError::Unavailable)?;
            Ok(None)
        }
        None => Ok(None),
    }
}
pub(super) async fn login(State(state): State<AuthState>, session: Session) -> ApiResult<Redirect> {
    let provider = state.oidc.as_ref().ok_or(AppError::NotFound)?;
    let (client, _) = provider.discover().await?;
    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
    let (url, csrf, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("profile".into()))
        .add_scope(Scope::new("email".into()))
        .set_pkce_challenge(challenge)
        .url();
    session
        .insert(
            "oidc_pending",
            PendingLogin {
                state: csrf.secret().clone(),
                nonce: nonce.secret().clone(),
                verifier: verifier.secret().clone(),
                issued_at: Utc::now().timestamp(),
            },
        )
        .await
        .map_err(|_| AppError::Unavailable)?;
    Ok(Redirect::to(url.as_str()))
}
#[derive(Deserialize)]
pub(super) struct Callback {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}
pub(super) async fn callback(
    State(state): State<AuthState>,
    session: Session,
    Query(input): Query<Callback>,
) -> ApiResult<Redirect> {
    let provider = state.oidc.as_ref().ok_or(AppError::NotFound)?;
    let pending: PendingLogin = session
        .remove("oidc_pending")
        .await
        .map_err(|_| AppError::Unavailable)?
        .ok_or(AppError::Unauthorized)?;
    // Persist one-time consumption before contacting the provider, including on failed callbacks.
    session.save().await.map_err(|_| AppError::Unavailable)?;
    let supplied = input.state.unwrap_or_default();
    if supplied.len() != pending.state.len()
        || !bool::from(supplied.as_bytes().ct_eq(pending.state.as_bytes()))
        || Utc::now().timestamp() - pending.issued_at > 600
        || input.error.is_some()
    {
        return Err(AppError::Unauthorized.into());
    }
    let code = input
        .code
        .filter(|c| !c.is_empty() && c.len() <= 4096)
        .ok_or(AppError::Unauthorized)?;
    let (client, _) = provider.discover().await?;
    let response = client
        .exchange_code(AuthorizationCode::new(code))
        .map_err(|_| AppError::Unavailable)?
        .set_pkce_verifier(PkceCodeVerifier::new(pending.verifier))
        .request_async(&provider.http)
        .await
        .map_err(|_| AppError::Unauthorized)?;
    let id_token = response.id_token().ok_or(AppError::Unauthorized)?;
    let verifier = client.id_token_verifier();
    let claims = id_token
        .claims(&verifier, &Nonce::new(pending.nonce))
        .map_err(|_| AppError::Unauthorized)?;
    if let Some(expected) = claims.access_token_hash() {
        let actual = AccessTokenHash::from_token(
            response.access_token(),
            id_token.signing_alg().map_err(|_| AppError::Unauthorized)?,
            id_token
                .signing_key(&verifier)
                .map_err(|_| AppError::Unauthorized)?,
        )
        .map_err(|_| AppError::Unauthorized)?;
        if &actual != expected {
            return Err(AppError::Unauthorized.into());
        }
    }
    let actor = Actor {
        id: Actor::external_id(claims.issuer().as_str(), claims.subject().as_str()),
        name: claims
            .name()
            .and_then(|n| n.get(None))
            .map(|n| n.as_str())
            .or_else(|| claims.preferred_username().map(|n| n.as_str()))
            .unwrap_or(claims.subject().as_str())
            .into(),
        permissions: claims.additional_claims().permissions(
            &provider.config.client_id,
            &provider.config.role_permissions,
        ),
    };
    session.flush().await.map_err(|_| AppError::Unavailable)?;
    session
        .insert(
            "oidc_ticket",
            ExternalTicket {
                actor,
                expires_at: claims
                    .expiration()
                    .timestamp()
                    .min(Utc::now().timestamp() + 8 * 3600),
                id_token: id_token.to_string(),
            },
        )
        .await
        .map_err(|_| AppError::Unavailable)?;
    session
        .insert("csrf", random_token())
        .await
        .map_err(|_| AppError::Unavailable)?;
    Ok(Redirect::to(&format!(
        "{}/#/",
        provider.config.public_origin
    )))
}
pub(super) async fn logout(
    state: &AuthState,
    _actor: CurrentActor,
    session: &Session,
) -> ApiResult<Redirect> {
    let provider = state.oidc.as_ref().ok_or(AppError::NotFound)?;
    let ticket: ExternalTicket = session
        .get("oidc_ticket")
        .await
        .map_err(|_| AppError::Unavailable)?
        .ok_or(AppError::Unauthorized)?;
    session.flush().await.map_err(|_| AppError::Unavailable)?;
    let (_, endpoint) = provider.discover().await?;
    let Some(endpoint) = endpoint else {
        return Ok(Redirect::to(&format!(
            "{}/#/",
            provider.config.public_origin
        )));
    };
    let correlation = CsrfToken::new_random();
    session
        .insert(
            "oidc_logout",
            (correlation.secret(), Utc::now().timestamp()),
        )
        .await
        .map_err(|_| AppError::Unavailable)?;
    let token: IdToken<
        Roles,
        CoreGenderClaim,
        CoreJweContentEncryptionAlgorithm,
        CoreJwsSigningAlgorithm,
    > = serde_json::from_value(serde_json::Value::String(ticket.id_token))
        .map_err(|_| AppError::Internal)?;
    let url = LogoutRequest::from(endpoint)
        .set_id_token_hint(&token)
        .set_client_id(ClientId::new(provider.config.client_id.clone()))
        .set_post_logout_redirect_uri(
            PostLogoutRedirectUrl::new(format!(
                "{}/signout-callback-oidc",
                provider.config.public_origin
            ))
            .map_err(|_| AppError::Internal)?,
        )
        .set_state(correlation)
        .http_get_url();
    Ok(Redirect::to(url.as_str()))
}
#[derive(Deserialize)]
pub(super) struct LogoutCallback {
    state: Option<String>,
}
pub(super) async fn logout_callback(
    State(state): State<AuthState>,
    session: Session,
    Query(input): Query<LogoutCallback>,
) -> ApiResult<Redirect> {
    let provider = state.oidc.as_ref().ok_or(AppError::NotFound)?;
    let (expected, issued): (String, i64) = session
        .remove("oidc_logout")
        .await
        .map_err(|_| AppError::Unavailable)?
        .ok_or(AppError::Unauthorized)?;
    let actual = input.state.unwrap_or_default();
    session.flush().await.map_err(|_| AppError::Unavailable)?;
    if actual.len() != expected.len()
        || !bool::from(actual.as_bytes().ct_eq(expected.as_bytes()))
        || Utc::now().timestamp() - issued > 600
    {
        return Err(AppError::Unauthorized.into());
    }
    Ok(Redirect::to(&format!(
        "{}/#/",
        provider.config.public_origin
    )))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roles_only_grant_explicit_mappings() {
        let roles: Roles=serde_json::from_value(serde_json::json!({"roles":["reader","crm.customers.delete"],"realm_access":{"roles":["unknown"]},"resource_access":{"atlas":{"roles":["editor"]},"other":{"roles":["admin"]}},"permissions":["crm.customers.delete"]})).expect("roles");
        let mapping = BTreeMap::from([
            ("reader".into(), vec!["crm.customers.read".into()]),
            ("editor".into(), vec!["crm.customers.write".into()]),
            ("admin".into(), vec!["crm.customers.delete".into()]),
        ]);
        assert_eq!(
            roles.permissions("atlas", &mapping),
            BTreeSet::from(["crm.customers.read".into(), "crm.customers.write".into()])
        );
        assert!(valid_url("http://issuer.test", false).is_err());
        assert!(valid_url("https://name:secret@issuer.test", false).is_err());
    }
}
