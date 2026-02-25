use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use fjall::Database;

use crate::server::{
    auth::AuthService,
    channel::text::{TextChannel, TextChannelError},
};

pub mod auth;
pub mod channel;
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

    /// FSM-tree database for storing the time-series channel messages.
    db: fjall::Database,

    auth: Arc<AuthService>,

    /// A hashmap of the available channels on the server.
    text_channels: RwLock<HashMap<u64, Arc<TextChannel>>>,
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

        let auth = Arc::new(AuthService::new(config.auth.clone()));

        Ok(Self {
            config,
            db,
            auth,
            text_channels: RwLock::new(HashMap::new()),
        })
    }

    /// Returns a handle to the auth service.
    pub fn auth(&self) -> Arc<AuthService> {
        Arc::clone(&self.auth)
    }

    /// Create a new text channel on the server.
    ///
    /// Returns a handle to the created text channel.
    pub fn create_text_channel(
        &self,
        label: String,
    ) -> Result<Arc<TextChannel>, CreateChannelError> {
        let id = 0; // tODO: generate channel id

        // Construct the data directory for the channel.
        let data_dir = self.config.data_dir.join("channels").join(id.to_string());

        // SAFETY: Fjall database is syncronized for thread-safe
        //  access and can be cloned without external locks.
        let channel = Arc::new(TextChannel::new(&data_dir, self.db.clone(), id, label)?);

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
