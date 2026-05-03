use crate::application::auth as auth_app;
use crate::{AppError, AppState};
use axum::body::Body;
use axum::extract::State;
use axum::http::header::{COOKIE, SET_COOKIE};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Request};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

pub const SESSION_COOKIE_NAME: &str = "__Host-musiclib_session";

const HSTS_HEADER: &str = "max-age=63072000; includeSubDomains";
const SESSION_COOKIE_PATH: &str = "/";

pub async fn require_auth(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();
    let account_count = match auth_app::account_count(&state.repositories.auth).await {
        Ok(count) => count,
        Err(err) => return with_security_headers(AppError::from(err).into_response(), &state),
    };
    if account_count == 0 || !path.starts_with("/api/") {
        return with_security_headers(next.run(req).await, &state);
    }
    if !state.transport.encrypted {
        return with_security_headers(
            AppError::upgrade_required("HTTPS is required when accounts are enabled")
                .into_response(),
            &state,
        );
    }
    if is_public_auth_route(&path) {
        return with_security_headers(next.run(req).await, &state);
    }

    let token = session_cookie(req.headers());
    let username =
        match auth_app::authenticated_username(&state.repositories.auth, token.as_deref()).await {
            Ok(username) => username,
            Err(err) => return with_security_headers(AppError::from(err).into_response(), &state),
        };
    let Some(username) = username else {
        return with_security_headers(
            AppError::unauthorized("login required").into_response(),
            &state,
        );
    };
    req.extensions_mut().insert(AuthenticatedUser { username });
    with_security_headers(next.run(req).await, &state)
}

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub username: String,
}

pub fn session_cookie(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|part| part.trim().split_once('='))
        .find_map(|(name, value)| {
            if name == SESSION_COOKIE_NAME {
                Some(value.to_string())
            } else {
                None
            }
        })
}

pub fn session_set_cookie(token: &str, max_age_seconds: i64) -> HeaderValue {
    HeaderValue::from_str(&format!(
        "{SESSION_COOKIE_NAME}={token}; Max-Age={max_age_seconds}; Path={SESSION_COOKIE_PATH}; HttpOnly; Secure; SameSite=Strict"
    ))
    .expect("session cookie value should be a valid header")
}

pub fn session_clear_cookie() -> HeaderValue {
    HeaderValue::from_static(
        "__Host-musiclib_session=; Max-Age=0; Path=/; HttpOnly; Secure; SameSite=Strict",
    )
}

pub fn add_set_cookie(response: &mut Response, cookie: HeaderValue) {
    response.headers_mut().append(SET_COOKIE, cookie);
}

fn is_public_auth_route(path: &str) -> bool {
    matches!(
        path,
        "/api/auth/status" | "/api/auth/login" | "/api/auth/logout"
    )
}

fn with_security_headers(mut response: Response, state: &AppState) -> Response {
    response.headers_mut().insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    response.headers_mut().insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("same-origin"),
    );
    if state.transport.hsts {
        response.headers_mut().insert(
            HeaderName::from_static("strict-transport-security"),
            HeaderValue::from_static(HSTS_HEADER),
        );
    }
    response
}
