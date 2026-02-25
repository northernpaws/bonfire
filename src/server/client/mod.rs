use std::{
    collections::HashMap,
    hash::{self, Hasher},
    sync::{Arc, RwLock},
};

use chrono::Utc;
use snowflaked::Snowflake;

use crate::proto::v0;

/// Concrete type for client session ID's .
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct SessionId(u64);

impl hash::Hash for SessionId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.0);
    }
}

impl Snowflake for SessionId {
    fn from_parts(timestamp: u64, instance: u64, sequence: u64) -> Self {
        Self(u64::from_parts(timestamp, instance, sequence))
    }

    fn timestamp(&self) -> u64 {
        self.0.timestamp()
    }

    fn instance(&self) -> u64 {
        self.0.instance()
    }

    fn sequence(&self) -> u64 {
        self.0.sequence()
    }
}

/// Indicates the connection state of the client.
pub enum ConnectionState {
    Connected,
    Disconnected,
}

/// State of a connected client session.
pub struct Session {
    id: SessionId,

    /// Indiciates the connection state of the client.
    state: ConnectionState,

    /// The identity transmitted by the client
    /// when it connected to the gateway.
    identity: v0::GatewayIdentify,

    /// Indicates when the client was last
    /// connected to the session in seconds.
    last_contact_s: i64,
}

impl Session {
    /// Constructs a new client session.
    pub fn new(id: SessionId, state: ConnectionState, identity: v0::GatewayIdentify) -> Self {
        Self {
            id,
            state,
            identity,
            last_contact_s: 0,
        }
    }

    /// Returns the ID of the session.
    pub fn session_id(&self) -> SessionId {
        self.id
    }

    /// Updates the last-contacted time for the session.
    pub fn contacted(&mut self) {
        self.last_contact_s = Utc::now().timestamp();

        tracing::debug!(session = ?self.id, "updating client session with heartbeat");
    }
}

/// Service for managing clients.
///
/// This maintains and manages client connection sessions.
pub struct ClientService {
    id_generator: snowflaked::Generator,

    /// Active client sessions.
    sessions: RwLock<HashMap<SessionId, Arc<RwLock<Session>>>>,
}

impl ClientService {
    /// Construct a new instance of the client service.
    pub fn new() -> Self {
        Self {
            id_generator: snowflaked::Generator::new(0),
            sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Creates a new client connection session.
    pub fn create_session(&mut self, identity: v0::GatewayIdentify) -> Arc<RwLock<Session>> {
        // Generate the ID for the new session.
        let id = self.id_generator.generate();

        // Construct the new session's state.
        let session = Arc::new(RwLock::new(Session::new(
            id,
            ConnectionState::Connected,
            identity,
        )));

        // Insert the session into the active session table.
        self.sessions
            .write()
            .unwrap()
            .insert(id, Arc::clone(&session));

        tracing::info!(id = ?id, "created new client session");

        session
    }

    /// Closes an open client session.
    pub fn close_session(&mut self, id: SessionId) {
        // Remove the session from the active session table.
        self.sessions.write().unwrap().remove(&id);

        tracing::info!(id = ?id, "closing client session");
    }
}
