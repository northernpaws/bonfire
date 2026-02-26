//! Module for text and voice channel functionality.
//!
//! The channel types (voice and text) get their own
//! submodules that encapsulate their functionality.

use tokio::sync::broadcast;

use crate::channel::ChannelId;

/// Indicates the type of a channel.
pub enum ChannelType {
    Text,
    Voice,
}

/// Generic trait for channel types.
pub trait Channel {
    type Event;

    /// Returns the ID of the channel.
    fn channel_id(&self) -> ChannelId;

    /// Returns the type of the channel.
    fn channel_type(&self) -> ChannelType;

    /// Returns the user-friendly label for the channel.
    fn get_label(&self) -> &str;

    /// Returns a subscriber for receiving channel events.
    fn subscribe(&self) -> broadcast::Receiver<Self::Event>;
}

pub mod text;
pub mod voice;
