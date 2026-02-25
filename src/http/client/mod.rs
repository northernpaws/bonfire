use askama::Template;
use axum::{Router, response::IntoResponse, routing::get};

pub mod templates;

pub fn make_client_router() -> Router {
    Router::new().route("/login", get(handle_login))
}

/// Renders the login page
async fn handle_login() -> impl IntoResponse {
    let oauth2_providers = Vec::new();

    let page = templates::LoginTemplate { oauth2_providers };

    page.render().unwrap()
}
