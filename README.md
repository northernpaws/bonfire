# Bonfire

A self-hosted, open-source, decentralized community platform for text and voice communication.

**insert links and badges**

## Overview 

Created as an alternative to Discord, Matrix, IRCv3, etc. that includes a full set of modern chat features, wrapped up in an entirely open-source server and protocol.

 * **Self Managed**: You own and manage your community's data. Download it, process it through bots, access it in any formats your community needs without violating a TOS.

 * **Expressive**: Supports the expressive feature sets of modern chat - message replies, reactions, custom emoji, stickers, etc. out of the box.

## Features

> Checked features have already been implemented to prototype with, but will need to be ratified in an API specification prior to a v1.0 release.

 - [ ] **Media management**: File uploading for pictures, videos, and files in messages. Possibly with the ability to enable temporary media links so the media server can't be abused.
 - [ ] **Custom server stickers & emojis**: Ties into media management to store images/gifs. Each resources has a unique ID used to reference them in messages. Can be shared and used externally in other servers if allowed by the source and destination server.
 - [ ] **Multiple user identities**: Support one authenticated user having multiple identifies linked to their account that can be selected per-message and for voice presence. Replaces bots like Séance and PluralKit with native support.

### Text Channels

 - [ ] **Message emoji reactions**: Users can react to a message with one or more emoji's defined on the server. Should be expanded later to allow emoji's from other servers. Should have a notification channel for received reactions on messages.
 - [ ] **Message replies**: Users can reply to a message, quoting it and supplying a link or reference to the original message. Replies can also optionally notify the original message author.
 - [ ] **Message edits**: Users can edit their messages after being sent, and they update for all connected users.
 - [ ] **Message deletion**: Users can edit delete their messages after being sent, and they update for all connected users.
  - [ ] **User history/unread tracking**: Server tracks the last known timestamp of the last message in a channel. Clients can read the pointer to update it's read message location.

### Voice Channels

 - [ ] **Voice "lobby" channels**: Passive channels that users can join at any time. Implemented with WebRTC for voice and video streams using `rustrtc`.
 - [ ] **Screen sharting**: WebRTC video support for screen sharing in calls.
 - [ ] **Video Support**: WebRTC video support for cameras in calls.

### Integrations

 - [ ] **Discord Bridge**: Embedded Discord bot to bridge in channels.
 - [ ] **IRCv3 Bridge**: Embedded IRCv3 client to bridge in channels.
 - [ ] **Matrix Bridge**: Embedded Matrix client to bridge in channels.
