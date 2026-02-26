//! The common types for messages, including message encoding and decoding.

use crate::{channel::ChannelId, role::RoleId, user::UserId};

pub const ROLE_BLOCK_PREFIX: &str = "@&";
pub const USER_BLOCK_PREFIX: &str = "@";
pub const CHANNEL_BLOCK_PREFIX: &str = "#";
pub const TIMESTAMP_BLOCK_PREFIX: &str = "t:";

/// Creates the formatted string for mentioning a role in a message's content.
pub fn format_role(role_id: RoleId) -> String {
    format!("<{}{}>", ROLE_BLOCK_PREFIX, role_id)
}

/// Creates the formatted string for mentioning a user in a message's content.
pub fn format_user(user_id: UserId) -> String {
    format!("<{}{}>", USER_BLOCK_PREFIX, user_id)
}

/// Creates the formatted string for mentioning a channel in a message's content.
pub fn format_channel(channel_id: ChannelId) -> String {
    format!("<{}{}>", CHANNEL_BLOCK_PREFIX, channel_id)
}

/// Creates the formatted string for mentioning a channel in a message's content.
pub fn format_timestamp(unix_timestamp: i64) -> String {
    format!("<{}{}>", TIMESTAMP_BLOCK_PREFIX, unix_timestamp)
}

/// Represents a block of message content.
#[derive(Clone, PartialEq, Eq)]
pub enum MessageBlock {
    User(UserId),
    Channel(ChannelId),
    Role(RoleId),
    Timestamp(i64),
    Text(String),
}

/// Represents the contents of a message.
pub struct MessageContent(pub Vec<MessageBlock>);

/// Decode a message contents block from a string.
impl From<&str> for MessageContent {
    fn from(value: &str) -> Self {
        decode_message(value)
    }
}

/// Decode a message contents block from a string.
impl From<String> for MessageContent {
    fn from(value: String) -> Self {
        decode_message(value.as_str())
    }
}

/// Attempts to decode a string as a message block.
///
/// If parsing any special blocks fails, then
/// we return the original string.
///
/// No errors are returned because we _always_ need to
/// process the message string, if it fails then we
/// fall back to just displaying the text.
pub fn decode_message_part(original_part: &str) -> MessageBlock {
    // Check if the message part starts with a block previx.
    //
    // If the prefix or suffix is missing then we return the original message.

    let Some(stripped_part) = original_part.strip_prefix("<") else {
        return MessageBlock::Text(original_part.to_string());
    };

    let Some(stripped_part) = stripped_part.strip_suffix(">") else {
        return MessageBlock::Text(original_part.to_string());
    };

    // Check the special prefixes of the sections with the `<` prefix stripped.
    if let Some(role) = stripped_part.strip_prefix(ROLE_BLOCK_PREFIX) {
        // Attempt to parse the string to an integer ID.
        let Ok(role_id) = role.parse() else {
            return MessageBlock::Text(original_part.to_string());
        };

        return MessageBlock::Role(role_id);
    } else if let Some(user) = stripped_part.strip_prefix(USER_BLOCK_PREFIX) {
        // Attempt to parse the string to an integer ID.
        let Ok(user_id) = user.parse() else {
            return MessageBlock::Text(original_part.to_string());
        };

        return MessageBlock::User(user_id);
    } else if let Some(channel) = stripped_part.strip_prefix(CHANNEL_BLOCK_PREFIX) {
        // Attempt to parse the string to an integer ID.
        let Ok(channel_id) = channel.parse() else {
            return MessageBlock::Text(original_part.to_string());
        };

        return MessageBlock::User(channel_id);
    } else if let Some(timestamp) = stripped_part.strip_prefix(TIMESTAMP_BLOCK_PREFIX) {
        // Attempt to parse the string to an integer ID.
        let Ok(timestamp) = timestamp.parse() else {
            return MessageBlock::Text(original_part.to_string());
        };

        return MessageBlock::Timestamp(timestamp);
    } else {
        return MessageBlock::Text(original_part.to_string());
    }
}

/// Decodes the provided string as message contents.
pub fn decode_message(contents: &str) -> MessageContent {
    MessageContent(
        contents
            .split_whitespace()
            .map(|w| decode_message_part(w))
            .collect(),
    )
}
