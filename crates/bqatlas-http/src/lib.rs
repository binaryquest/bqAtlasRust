//! Axum conventions for errors, explicit JSON inputs and authenticated actors.
use axum::{
    Json,
    extract::{FromRequest, Request},
    response::{IntoResponse, Response},
};
use bqatlas_contracts::ProblemDetails;
use bqatlas_core::{Actor, AppError};
use http::{StatusCode, header};
use serde::de::DeserializeOwned;

pub struct ApiError(pub AppError);
impl From<AppError> for ApiError {
    fn from(value: AppError) -> Self {
        Self(value)
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, title, detail, errors) = match self.0 {
            AppError::Validation(errors) => (
                400,
                "validation_failed",
                "Validation failed".to_owned(),
                None,
                Some(errors),
            ),
            AppError::BadRequest(message) => (400, "invalid_request", message, None, None),
            AppError::Unauthorized => (
                401,
                "unauthorized",
                "Authentication is required".into(),
                None,
                None,
            ),
            AppError::Forbidden => (
                403,
                "forbidden",
                "This operation is not permitted".into(),
                None,
                None,
            ),
            AppError::NotFound => (404, "not_found", "Record was not found".into(), None, None),
            AppError::PreconditionRequired => (
                428,
                "precondition_required",
                "If-Match is required.".into(),
                None,
                None,
            ),
            AppError::VersionConflict => (
                412,
                "version_conflict",
                "Record changed".into(),
                Some("This record changed after you opened it. Reload before saving.".into()),
                None,
            ),
            AppError::Conflict { code, message } => {
                return problem(409, &code, &message, None, None);
            }
            AppError::Csrf => (
                400,
                "csrf_failed",
                "Invalid request verification token".into(),
                None,
                None,
            ),
            AppError::RateLimited => (
                429,
                "rate_limited",
                "Too many requests. Try again later.".into(),
                None,
                None,
            ),
            AppError::Unavailable => (503, "unavailable", "Service unavailable".into(), None, None),
            AppError::Internal => (
                500,
                "internal_error",
                "An unexpected error occurred".into(),
                None,
                None,
            ),
        };
        problem(status, code, &title, detail, errors)
    }
}
fn problem(
    status: u16,
    code: &str,
    title: &str,
    detail: Option<String>,
    errors: Option<bqatlas_core::ValidationErrors>,
) -> Response {
    let value = ProblemDetails {
        kind: format!("https://httpstatuses.com/{status}"),
        title: title.into(),
        status,
        code: code.into(),
        detail,
        errors,
    };
    let mut response = (
        StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        Json(value),
    )
        .into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        http::HeaderValue::from_static("application/problem+json"),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        http::HeaderValue::from_static("no-store"),
    );
    response
}
pub type ApiResult<T> = Result<T, ApiError>;
pub struct ApiJson<T>(pub T);
impl<S, T> FromRequest<S> for ApiJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = ApiError;
    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        Json::<T>::from_request(request, state)
            .await
            .map(|Json(value)| Self(value))
            .map_err(|_| {
                ApiError(AppError::BadRequest(
                    "Invalid JSON request or field type.".into(),
                ))
            })
    }
}
#[derive(Clone)]
pub struct CurrentActor(pub Actor);
impl<S: Send + Sync> axum::extract::FromRequestParts<S> for CurrentActor {
    type Rejection = ApiError;
    async fn from_request_parts(
        parts: &mut http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Actor>()
            .cloned()
            .map(Self)
            .ok_or(ApiError(AppError::Unauthorized))
    }
}
pub fn record<T: serde::Serialize>(
    result: bqatlas_contracts::RecordResult<T>,
    created_path: Option<String>,
) -> ApiResult<Response> {
    let version = result.version.clone();
    let mut response = (
        if created_path.is_some() {
            StatusCode::CREATED
        } else {
            StatusCode::OK
        },
        Json(result),
    )
        .into_response();
    response.headers_mut().insert(
        header::ETAG,
        http::HeaderValue::from_str(&format!("\"{version}\""))
            .map_err(|_| ApiError(AppError::Internal))?,
    );
    if let Some(path) = created_path {
        response.headers_mut().insert(
            header::LOCATION,
            http::HeaderValue::from_str(&path).map_err(|_| ApiError(AppError::Internal))?,
        );
    }
    Ok(response)
}

/// Peer address supplied by the server, never a caller-controlled forwarding header.
pub struct ClientAddress(pub std::net::IpAddr);
impl<S: Send + Sync> axum::extract::FromRequestParts<S> for ClientAddress {
    type Rejection = std::convert::Infallible;
    async fn from_request_parts(
        parts: &mut http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self(
            parts
                .extensions
                .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
                .map_or(
                    std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
                    |remote| remote.0.ip(),
                ),
        ))
    }
}

/// Match the reference application's constrained resource paths and return structured errors.
pub struct ApiPath<T>(pub T);
impl<S, T> axum::extract::FromRequestParts<S> for ApiPath<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Send,
{
    type Rejection = ApiError;
    async fn from_request_parts(
        parts: &mut http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        axum::extract::Path::<T>::from_request_parts(parts, state)
            .await
            .map(|axum::extract::Path(value)| Self(value))
            .map_err(|_| ApiError(AppError::NotFound))
    }
}
