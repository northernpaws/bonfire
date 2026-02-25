use tokio::sync::broadcast;

/// An event emitted by a voice channel.
#[derive(Clone)]
pub enum VoiceChannelEvent {}

/// Provides a voice channel used for voice discussion between users.
pub struct VoiceChannel {
    label: String,

    /// Receiver for events emitted by the channel.
    ///
    /// This is typically cloned by a transport (i.e. an HTTP WebSocket
    /// handler) to receive and forward the events to the client.
    event_receiver: broadcast::Receiver<VoiceChannelEvent>,
}

impl VoiceChannel {
    pub fn new(label: String) -> Self {
        let (event_sender, event_receiver) = broadcast::channel(25);

        // TODO: use rustrtc for voice comms

        Self {
            label,
            event_receiver,
        }
    }
}

impl super::Channel for VoiceChannel {
    type Event = VoiceChannelEvent;

    fn channel_type(&self) -> super::ChannelType {
        super::ChannelType::Voice
    }

    fn get_label(&self) -> &str {
        &self.label
    }

    fn subscribe(&self) -> broadcast::Receiver<Self::Event> {
        self.event_receiver.resubscribe()
    }
}
