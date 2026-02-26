use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use fjall::Database;

use crate::{
    channel::ChannelId,
    server::{
        auth::AuthService,
        channel::text::{TextChannel, TextChannelError},
        client::ClientService,
    },
};

pub mod auth;
pub mod channel;
pub mod client;
pub mod user;

/// An event that occures on a server.
pub enum ServerEvent {
    /// Emitted when a new channel is created.
    ChannelCreated,
}

/// Config for the application server.
pub struct Config {
    /// Root directory for storing server data.
    pub data_dir: PathBuf,

    pub auth: auth::AuthConfig,
}

/// Application server.
pub struct Server {
    config: Config,

    id_generator: snowflaked::Generator,

    /// FSM-tree database for storing the time-series channel messages.
    db: fjall::Database,

    /// Service for managing user authentication.
    auth: Arc<RwLock<AuthService>>,
    /// Service for managing connections to clients.
    clients: Arc<RwLock<ClientService>>,

    /// A hashmap of the available channels on the server.
    text_channels: RwLock<HashMap<ChannelId, Arc<TextChannel>>>,
}

#[derive(Debug)]
pub enum Error {
    DatabaseError(fjall::Error),
}

pub enum CreateChannelError {
    /// Indicates that the R/W lock on the internal
    /// channel list has become poisoned somehow.
    PoisonedChannelLock,
    TextChannelError(TextChannelError),
}

impl From<TextChannelError> for CreateChannelError {
    fn from(value: TextChannelError) -> Self {
        CreateChannelError::TextChannelError(value)
    }
}

impl Server {
    /// Construct a new instance of the application.
    pub fn new(config: Config) -> Result<Self, Error> {
        // Open or create the database for the server.
        let database_dir: PathBuf = config.data_dir.clone().join("data");
        let db = Database::builder(database_dir)
            .open()
            .map_err(|e| Error::DatabaseError(e))?;

        // Construct the service for managing user authentication.
        let auth = Arc::new(RwLock::new(AuthService::new(config.auth.clone())));

        // Construct the service for managing connected client sessions.
        let clients = Arc::new(RwLock::new(ClientService::new()));

        Ok(Self {
            config,
            id_generator: snowflaked::Generator::new(0),
            db,
            auth,
            clients,
            text_channels: RwLock::new(HashMap::new()),
        })
    }

    /// Returns a handle to the auth service.
    pub fn auth(&self) -> Arc<RwLock<AuthService>> {
        Arc::clone(&self.auth)
    }

    /// Returns a handle to the client service.
    pub fn clients(&self) -> Arc<RwLock<ClientService>> {
        Arc::clone(&self.clients)
    }

    /// Create a new text channel on the server.
    ///
    /// Returns a handle to the created text channel.
    pub fn create_text_channel(
        &mut self,
        label: String,
    ) -> Result<Arc<TextChannel>, CreateChannelError> {
        // Generate a channel ID.
        let id: ChannelId = self.id_generator.generate();

        // Construct the data directory for the channel.
        let data_dir = self.config.data_dir.join("channels").join(id.0.to_string());

        // SAFETY: Fjall database is syncronized for thread-safe
        //  access and can be cloned without external locks.
        let channel = Arc::new(TextChannel::new(id, &data_dir, self.db.clone(), label)?);

        // Add the channel to the global channel list.
        self.text_channels
            .write()
            .map_err(|_| CreateChannelError::PoisonedChannelLock)?
            .insert(id, Arc::clone(&channel));

        Ok(channel)
    }

    /// Returns a list of handles to all the available channels.
    pub fn text_channels(&self) -> Vec<Arc<TextChannel>> {
        self.text_channels
            .read()
            .unwrap()
            .values()
            .map(|c| Arc::clone(c))
            .collect()
    }
}
