use std::sync::{Arc, RwLock};

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::{any, get, post},
};

use crate::server::{Server, channel::Channel};

pub mod client;
pub mod gateway;
pub mod oauth2;

/// Provides the shared state for the app router.
pub struct AppState {
    /// The application server
    server: Arc<Server>,
}

pub type SharedState = Arc<RwLock<AppState>>;

/// Create the HTTP app router for the server.
pub fn make_app_router(server: Arc<Server>) -> Router {
    let state: SharedState = Arc::new(RwLock::new(AppState { server }));

    Router::new()
        .route("/", get(handle_web_interface))
        .route("/channels", get(handle_list_channels))
        .route("/channels", post(handle_create_channel))
        // Inject the web client router at the `/client` path.
        .nest_service("/client", client::make_client_router())
        // Redirect URL to a provider's authorization endpoint.
        .route("/oauth/{provider}", any(oauth2::handle_redirect))
        // Callback from a user successfully authenticating with a provider.
        .route("/oauth/{provider}/callback", any(oauth2::handle_callback))
        // Gateway websocket used for server to client communications.
        .route("/gateway", post(gateway::ws_handler))
        .with_state(state)
}

/// Redirect users that hit the root in a browser to the client endpoint.
pub(crate) async fn handle_web_interface() -> impl IntoResponse {
    Redirect::temporary("/client")
}

/// Retrieves a list of all channels available on the server.
async fn handle_list_channels(State(state): State<SharedState>) -> impl IntoResponse {
    let state = state.read().unwrap();

    let text_channels = state.server.text_channels();

    let names: Vec<&str> = text_channels.iter().map(|c| c.get_label()).collect();

    Json(names).into_response()
}

/// Creates a new channel on the server.
async fn handle_create_channel(State(state): State<SharedState>) -> impl IntoResponse {
    let state = state.read().unwrap();

    let channel = match state
        .server
        .create_text_channel("todo-change-to-variable".to_string())
    {
        Ok(channel) => channel,
        Err(_) => {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    StatusCode::OK.into_response()
}
