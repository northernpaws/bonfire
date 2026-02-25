use tantivy::{DateTime, TantivyDocument};
use tokio::sync::broadcast;
use tracing::{Instrument, info_span};

use crate::server::channel::text::{TextChannelAction, TextChannelEvent};

#[tracing::instrument(skip(keyspace, index_writer))]
/// The channel worker task that runs for each channel to process messages and events.
pub async fn channel_worker(
    mut message_receiver: tachyonix::Receiver<TextChannelAction>,
    keyspace: fjall::Keyspace,
    index_writer: tantivy::IndexWriter,
    field_timestamp: tantivy::schema::Field,
    field_body: tantivy::schema::Field,
    field_author: tantivy::schema::Field,
    event_notifier: broadcast::Sender<TextChannelEvent>,
) {
    tracing::info!("channel worker started");

    // Primary text channel worker loop.
    loop {
        // Wait to receive the next message.
        let Ok(action) = message_receiver
            .recv()
            .instrument(info_span!("message_receiver_recv"))
            .await
        else {
            tracing::error!("channel message receiver was closed");

            break;
        };

        match action {
            TextChannelAction::MessageCreated(msg) => {
                // Store the message in the FSM-tree time-series database.
                if let Err(err) = keyspace.insert(msg.timestamp_ms.to_be_bytes(), msg.body.clone())
                {
                    tracing::error!(%err, "failed to insert message to keyspace")
                }

                // Create a document from the message for search.
                let mut document = TantivyDocument::default();
                document.add_date(
                    field_timestamp,
                    DateTime::from_timestamp_secs(msg.timestamp_ms as i64),
                );
                document.add_text(field_body, msg.body.clone());
                document.add_u64(field_author, msg.author);

                // Write the full-text search log entry.
                if let Err(err) = index_writer.add_document(document) {
                    tracing::error!(%err, "failed to add document to index");
                    // TODO: should retry
                }

                // Emit a channel event for the next message to inform clients.
                if let Err(err) = event_notifier.send(TextChannelEvent::NewMessage(msg)) {
                    tracing::error!(%err, "failed to add document to index");
                }
            }
            TextChannelAction::MessageEdited() => todo!(),
        }
    }

    tracing::info!("channel worker exit");
}
