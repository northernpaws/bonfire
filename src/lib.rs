pub mod channel;
pub mod message;
pub mod role;
pub mod user;

pub mod proto;

/// Implements the server-side logic.
#[cfg(feature = "server")]
pub mod server;

/// HTTP server interface.
#[cfg(feature = "server")]
pub mod http;
