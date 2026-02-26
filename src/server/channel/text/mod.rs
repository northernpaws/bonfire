//! Provides text channel functionality.

use std::{io, path::PathBuf};

use fjall::KeyspaceCreateOptions;
use tantivy::{TantivyError, directory::error::OpenDirectoryError};
use tokio::sync::broadcast;

use crate::{
    server::channel::{
        ChannelId,
        text::search::{
            SCHEMA_KEY_AUTHOR, SCHEMA_KEY_CONTENT, SCHEMA_KEY_TIMESTAMP, text_search_schema,
        },
    },
    user::UserId,
};

pub mod search;
pub mod worker;

/// A text message received on a channel.
#[derive(Clone)]
pub struct TextChannelMessage {
    /// The author of the message.
    pub author: UserId,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Text body of the message.
    pub content: String,
}

/// These are sent to a channel to tell it to do something.
pub enum TextChannelAction {
    /// Informs the channel that a new message should be created and
    /// the new message should be distributed to connected clients.
    ///
    /// This adds the message to the time-series database for the
    /// channel and adds it as an indexed document for search.
    MessageCreated(TextChannelMessage),

    /// Informs the channel that an existing message should be edited
    /// with new contents, and the edit distributed to clients.
    ///
    /// This update's the message's contents stored in the time-series
    /// database and indexed for full-text search.
    MessageEdited(),
}

/// Events that can occur in a text channel.
///
/// These are emitted by the channel to inform
/// clients of a change in chanel status.
#[derive(Clone)]
pub enum TextChannelEvent {
    NewMessage(TextChannelMessage),
    MessageEdited(TextChannelMessage),
}

/// Indiciates there's was an error creating or loading a channel.
pub enum TextChannelError {
    /// Indicates that a blank label was supplied.
    LabelRequired,
    /// Indicates there was an error creating the channel
    /// keyspace for storing the time-series message data.
    KeyspaceError(fjall::Error),

    /// Indicates there was an error creating or reading the
    /// data directory for storing the search index.
    SearchIndexPathError(io::Error),
    /// Indicates there was an error opening the directory for the search index.
    SearchIndexDirectoryError(OpenDirectoryError),
    /// Indicates there was an error creating
    /// the full-text search database.
    SearchError(TantivyError),
}

pub type TextChannelSender = tachyonix::Sender<TextChannelAction>;

/// A channel on a server.
pub struct TextChannel {
    /// The unique ID used to identify the channel.
    id: ChannelId,

    /// User-facing label for the channel.
    label: String,

    /// Keyspace for storing the time-series data for channel messages.
    keyspace: fjall::Keyspace,

    /// Sender for sending messages to the channel.
    message_sender: TextChannelSender,

    /// Receiver for events emitted by the channel.
    ///
    /// This is typically cloned by a transport (i.e. an HTTP WebSocket
    /// handler) to receive and forward the events to the client.
    event_receiver: broadcast::Receiver<TextChannelEvent>,
}

impl TextChannel {
    /// Constructs a new channel instance.
    pub fn new(
        id: ChannelId,
        data_dir: &PathBuf,
        db: fjall::Database,
        label: String,
    ) -> Result<Self, TextChannelError> {
        if label.is_empty() {
            return Err(TextChannelError::LabelRequired);
        }

        // Construct the database keyspace for storing the channel messages.
        //
        // This will create a new keyspace if none exists, or open an existing one.
        let keyspace = db
            .keyspace(&id.0.to_string(), keyspace_create_options)
            .map_err(|e| TextChannelError::KeyspaceError(e))?;

        // Create the text search schema used for querying logs.
        let schema = text_search_schema();

        // Create the directory for the search index if required.
        let index_dir_path: PathBuf = data_dir.join("search");
        std::fs::create_dir_all(&index_dir_path)
            .map_err(|e| TextChannelError::SearchIndexPathError(e))?;

        // Open or create the search index.
        let index_directory = tantivy::directory::MmapDirectory::open(index_dir_path)
            .map_err(|e| TextChannelError::SearchIndexDirectoryError(e))?;
        let index = tantivy::Index::open_or_create(index_directory, schema.clone())
            .map_err(|e| TextChannelError::SearchError(e))?;

        // Create the index writing for the channel's message worker task.
        let index_writer: tantivy::IndexWriter = index
            .writer(50_000_000) // 50MB
            .map_err(|e| TextChannelError::SearchError(e))?;

        // Create the channel used to forward messages to the text channel's worker task.
        let (message_sender, message_receiver) = tachyonix::channel(25);

        let (event_sender, event_receiver) = broadcast::channel(25);

        // Spawn the text channel's worker.
        // TODO: restart worker if task crashes.
        let _handle = tokio::spawn(worker::channel_worker(
            message_receiver,
            keyspace.clone(),
            index_writer,
            schema.get_field(SCHEMA_KEY_TIMESTAMP).unwrap(),
            schema.get_field(SCHEMA_KEY_CONTENT).unwrap(),
            schema.get_field(SCHEMA_KEY_AUTHOR).unwrap(),
            event_sender,
        ));

        Ok(Self {
            id,
            label,
            keyspace,
            message_sender,
            event_receiver,
        })
    }

    /// Returns a new text channel message sender that
    /// forwards messages to the text channel worker.
    pub fn message_sender(&self) -> TextChannelSender {
        self.message_sender.clone()
    }
}

impl super::Channel for TextChannel {
    type Event = TextChannelEvent;

    fn channel_id(&self) -> ChannelId {
        self.id
    }

    fn channel_type(&self) -> super::ChannelType {
        super::ChannelType::Text
    }

    fn get_label(&self) -> &str {
        &self.label
    }

    fn subscribe(&self) -> broadcast::Receiver<Self::Event> {
        self.event_receiver.resubscribe()
    }
}

/// Options for creating fjall keyspaces for channels.
fn keyspace_create_options() -> KeyspaceCreateOptions {
    KeyspaceCreateOptions::default()
}
