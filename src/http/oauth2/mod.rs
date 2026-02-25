use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};

use axum_extra::extract::{CookieJar, cookie::Cookie};
use cookie::time::Duration;
use oauth2::{AuthorizationCode, CsrfToken};
use serde::Deserialize;

use crate::http::SharedState;

/// Handles redirecting a user to the specified OAuth2 provider's authorization endpoint.
///
/// Successful logins will have the user be redirected back to the `/callback` endpoint to
/// check exchange the authorization code for a token, and to issue the user a local token.
pub async fn handle_redirect(
    Path(provider): Path<String>,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    let auth = state.read().unwrap().server.auth();

    // Generate an authorization URL for the request.
    let Some(authorize_url) = auth.oauth2_authorize_web(
        provider.clone(),
        &format!("http://localhost:3000/oauth2/{provider}/callback"),
    ) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    Redirect::temporary(authorize_url.as_str()).into_response()
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: String,
    state: String,
}

/// Handles the callback from an OAuth2 provider to the application with an auth code and state.
pub async fn handle_callback(
    Path(provider): Path<String>,
    query: Query<CallbackQuery>,
    jar: CookieJar,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    let auth = state.read().unwrap().server.auth();

    // Extract the OAuth2 callback code and state supplied by the OAuth2 provider.
    let code = AuthorizationCode::new(query.0.code);
    let state = CsrfToken::new(query.0.state);

    // Attempt to exchange the code and state for a local auth token.
    let Some(token) = auth.oauth2_code_exchange_web(provider, code, state) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    // Build the cookie for the token.
    // ref: https://mattrighetti.com/2025/05/03/authentication-with-axum
    let cookie = Cookie::build(("token", token))
        .path("/")
        .http_only(true)
        .max_age(Duration::hours(6))
        .secure(if cfg!(debug_assertions) {
            // Safari won't allow secure cookies
            // coming from localhost in debug mode
            false
        } else {
            // Secure cookies in release mode
            true
        })
        .build();

    // Add the cookie to the response.
    jar.add(cookie);

    // Redirect use back to the web client.
    Redirect::temporary("/client").into_response()
}
