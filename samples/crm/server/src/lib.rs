pub mod seed;
use axum::{Json, Router, extract::State, middleware, routing::get};
use bqatlas_auth::{AuthState, DatabaseSessionStore};
use bqatlas_contracts::{ApplicationManifest, CONTRACT_VERSION};
use bqatlas_core::{AppError, ModuleDefinition, Registry};
use bqatlas_db::{Database, Migration, Sql};
use bqatlas_http::{ApiResult, CurrentActor};
use bqatlas_sample_crm::CustomerService;
use std::sync::Arc;
use tower_sessions::{Expiry, SessionManagerLayer, cookie::SameSite};

pub fn migrations() -> Vec<Migration> {
    vec![
        bqatlas_auth::migration(),
        bqatlas_auth::recovery::migration(),
        bqatlas_sample_crm::migration(),
        bqatlas_sample_sales::migration(),
        bqatlas_sample_engagement::migration(),
    ]
}
pub fn registry() -> Result<Registry, String> {
    let mut registry = Registry::default();
    registry.add_module(ModuleDefinition {
        id: "identity".into(),
        dependencies: vec![],
    })?;
    registry.add_module(bqatlas_sample_crm::definition())?;
    registry.add_resource(bqatlas_sample_crm::descriptor())?;
    registry.add_module(bqatlas_sample_sales::definition())?;
    registry.add_resource(bqatlas_sample_sales::descriptor())?;
    registry.add_module(bqatlas_sample_engagement::definition())?;
    for resource in bqatlas_sample_engagement::descriptors() {
        registry.add_resource(resource)?;
    }
    registry.module_order()?;
    Ok(registry)
}
pub fn permissions() -> Vec<String> {
    let mut permissions: Vec<String> = bqatlas_sample_sales::PERMISSIONS
        .iter()
        .map(|p| (*p).into())
        .collect();
    permissions.extend(bqatlas_sample_engagement::permissions());
    permissions.extend(
        bqatlas_sample_crm::PERMISSIONS
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>(),
    );
    permissions
}
pub async fn application(db: Database, secure_cookies: bool) -> Result<Router, AppError> {
    let auth = AuthState::new(db.clone()).await?;
    application_with_auth(db, secure_cookies, auth).await
}
pub async fn application_with_auth(
    db: Database,
    secure_cookies: bool,
    auth: AuthState,
) -> Result<Router, AppError> {
    let store = DatabaseSessionStore(db.clone());
    let sessions = SessionManagerLayer::new(store)
        .with_name("bqatlas-rust.session")
        .with_secure(secure_cookies)
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(time::Duration::hours(8)));
    let registry = Arc::new(registry().map_err(|_| AppError::Internal)?);
    let customers = Arc::new(CustomerService::new(db.clone()));
    let app =
        Router::new()
            .merge(bqatlas_auth::router().with_state(auth.clone()))
            .merge(bqatlas_sample_crm::router().with_state(CustomerService::new(db.clone())))
            .merge(bqatlas_sample_sales::router().with_state(
                bqatlas_sample_sales::QuoteService::new(db.clone(), customers.clone()),
            ))
            .merge(bqatlas_sample_engagement::router().with_state(
                bqatlas_sample_engagement::EngagementService::new(db.clone(), customers),
            ))
            .merge(
                Router::new()
                    .route("/api/v1/manifest", get(manifest))
                    .with_state(registry),
            )
            .merge(
                Router::new()
                    .route("/health/ready", get(ready))
                    .with_state(db),
            )
            .route(
                "/health/live",
                get(|| async { Json(serde_json::json!({"status":"ok"})) }),
            )
            .fallback(|| async { bqatlas_http::ApiError(AppError::NotFound) })
            .layer(middleware::from_fn_with_state(
                auth,
                bqatlas_auth::middleware,
            ))
            .layer(sessions)
            .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024))
            .layer(tower_http::timeout::TimeoutLayer::with_status_code(
                axum::http::StatusCode::REQUEST_TIMEOUT,
                std::time::Duration::from_secs(30),
            ))
            .layer(tower_http::trace::TraceLayer::new_for_http().make_span_with(|request:&axum::http::Request<axum::body::Body>| tracing::info_span!("http", method=%request.method(), path=request.uri().path(), request_id=?request.headers().get("x-request-id"))))
            .layer(tower_http::request_id::PropagateRequestIdLayer::x_request_id())
            .layer(tower_http::request_id::SetRequestIdLayer::x_request_id(tower_http::request_id::MakeRequestUuid));
    Ok(app)
}
async fn manifest(
    State(registry): State<Arc<Registry>>,
    CurrentActor(actor): CurrentActor,
) -> ApiResult<Json<ApplicationManifest>> {
    Ok(Json(ApplicationManifest {
        contract_version: CONTRACT_VERSION.into(),
        modules: registry.module_order().map_err(|_| AppError::Internal)?,
        resources: registry.resources_for(&actor),
    }))
}
async fn ready(State(db): State<Database>) -> ApiResult<Json<serde_json::Value>> {
    db.check_migrations(&migrations())
        .await
        .map_err(|e| e.application())?;
    db.query(
        Sql::new(
            "SELECT 1::bigint AS healthy",
            "SELECT CAST(1 AS bigint) AS healthy",
        ),
        &[],
    )
    .await
    .map_err(|e| e.application())?;
    Ok(Json(serde_json::json!({"status":"ready"})))
}
