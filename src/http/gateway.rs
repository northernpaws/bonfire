use axum::{
    extract::{
        ConnectInfo, Query, State,
        ws::{self, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::{TypedHeader, headers};
use futures::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use serde::Deserialize;
use serde_json::Value;
use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
};
use tracing::{Instrument, debug_span, info_span};

use prost::Message;

use crate::{
    proto::v0::{self, GatewayServerEvent, gateway_client_event},
    server::client,
};

/// Identifies the encoding used by the gateway.
#[derive(Clone, Copy, Debug, Deserialize)]
pub enum Encoding {
    /// Encodes and decodes messages using the Protobuf specification.
    Protobuf,

    /// Encodes and decodes messages using the JSON serialized
    /// representation of the Protobuf specification.
    Json,
}

// impl tracing::Value for Encoding {
//     fn record(&self, key: &tracing::field::Field, visitor: &mut dyn tracing::field::Visit) {
//         match self {
//             Encoding::Protobuf => visitor.record_str(key, "protobuf"),
//             Encoding::Json => visitor.record_str(key, "json"),
//         }
//     }
// }

/// Query parameters supported by the gateway endpoint.
#[derive(Deserialize)]
pub struct GatewayQuery {
    version: Option<String>,
    encoding: Option<Encoding>,
}

/// The initial handler for the HTTP request to initiate WebSocket negotiation.
///
/// After this completes, switch from HTTP to websocket protocol will occur.
///
/// This is the last point where we can extract TCP/IP metadata such as IP address of the
/// client as well as things from HTTP headers such as user-agent of the browser etc.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    query: Query<GatewayQuery>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<super::SharedState>,
) -> impl IntoResponse {
    // Short-circuit early if we can't support the requested version.
    match query.0.version {
        Some(version) => {
            if version == "v0" {
                return StatusCode::BAD_REQUEST.into_response();
            }
        }
        None => {}
    };

    // Grab the user agent for logging and identification.
    let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
        user_agent.to_string()
    } else {
        String::from("Unknown browser")
    };

    println!("`{user_agent}` at {addr} connected to gateway");

    // Either extract the encoding from the query
    // parameters, or use the default JSON encoding.
    let encoding = match query.0.encoding {
        Some(encoding) => encoding,
        None => Encoding::Json,
    };

    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| handle_socket(socket, addr, state, encoding))
}

/// The WebSocket state machine spawned per connection.
async fn handle_socket(
    mut socket: WebSocket,
    who: SocketAddr,
    state: super::SharedState,
    encoding: Encoding,
) {
    tracing::info!(
        encoding_test = ?encoding,
        who = ?who,
        "new gateway socket connection, sending handshake to client"
    );

    // First, send a handshake message to the client to
    // identify the server version and capabilities.
    send_handshake_message(&mut socket, &encoding)
        .instrument(info_span!("gateway_handshake_send"))
        .await;

    tracing::info!(encoding_test = ?encoding, who = ?who, "waiting for client to identify to gateway");

    // Decode the identity message sent from the client to the websocket.
    //
    // This retries until a valid identify message is received.
    let Some(identity) = receive_identity_message(&mut socket)
        .instrument(info_span!("gateway_ident_recv"))
        .await
    else {
        tracing::error!("failed to get gateway identity message from client");
        return;
    };

    // TODO: Support a "Resume" message to allow a client to recall an existing
    //       session cached on the server instead of creating a new one.

    tracing::info!(
        encoding_test = ?encoding,
        who = ?who,
        client_agent = ?identity.client_agent,
        "successfully received client identity");

    let Some(user_id) = state
        .write()
        .unwrap()
        .server
        .auth()
        .write()
        .unwrap()
        .validate_token(&identity.token)
    else {
        tracing::error!("failed to validate gateway client's identity token");
        return;
    };

    tracing::info!(
        encoding_test = ?encoding,
        who = ?who,
        client_agent = ?identity.client_agent,
        "successfully authenticated gateway client token");

    // Create the client connection session.
    let session = state
        .write()
        .unwrap()
        .server
        .clients()
        .write()
        .unwrap()
        .create_session(user_id, identity.clone());

    tracing::info!(
        encoding_test = ?encoding,
        who = ?who,
        client_agent = ?identity.client_agent,
        session_id = ?session.read().unwrap().session_id(),
        "created gateway session for authenticated client");

    tracing::info!(
        encoding_test = ?encoding,
        who = ?who,
        client_agent = ?identity.client_agent,
        session_id = ?session.read().unwrap().session_id(),
        "starting gateway send and receive tasks");

    // Split the socket into a sender and receiver so that we
    // can process events in both directions simultaniously.
    let (sender, receiver) = socket.split();

    // Spawn the task to handle sending messages to the client.
    //
    // This is used to inform the client of events, such as new
    // messages message edits, reactions, etc. and notifications.
    let mut send_task = tokio::spawn(task_send(sender, Arc::clone(&session), encoding));

    // Spawn the task to handle receiving messages from the client.
    //
    // This is used by the client to send new messages and user events (i.e. status messages).
    let mut receive_task = tokio::spawn(task_receive(receiver, Arc::clone(&session), encoding));

    // If any one of the tasks exit, abort the other.
    tokio::select! {
        rv_a = (&mut send_task) => {
            if let Err(err) = rv_a {
                tracing::error!(%err, "unexpected panic sending gateway messages to client")
            };

            receive_task.abort();
        },
        rv_b = (&mut receive_task) => {
            if let Err(err) = rv_b {
                tracing::error!(%err, "unexpected panic receiving gateway messages from client")
            };

            send_task.abort();
        }
    }

    // If we hit this point then the WebSocket
    // tasks exited and we need to do cleanup.

    tracing::info!(who = ?who,
        client_agent = ?identity.client_agent,
        session_id = ?session.read().unwrap().session_id(),
        "gateway websocket connection closed");
}

/// Sends a handshake message from the gateway server to the connected client.
///
/// This informs the client of the server's version and capabilities.
async fn send_handshake_message(socket: &mut WebSocket, encoding: &Encoding) {
    // Build the gateway handshake.
    let handshake = v0::GatewayHandshake {
        version: "0.0.0".into(),
    };

    // Encode the gateway handshake.
    let handshake_message = match encoding {
        Encoding::Protobuf => {
            let mut handshake_buf = Vec::new();
            handshake_buf.reserve(handshake.encoded_len());
            handshake.encode(&mut handshake_buf).unwrap();

            ws::Message::Binary(handshake_buf.into())
        }
        Encoding::Json => {
            let j = serde_json::to_string(&handshake).unwrap();

            ws::Message::Text(j.into())
        }
    };

    // First, send a handshake to the client.
    socket
        .send(handshake_message)
        .instrument(info_span!("socket_send"))
        .await
        .unwrap();
}

/// Waits until it receives a valid identity message from the connected client.
///
/// This informs the server of the client's capabilities and identity.
async fn receive_identity_message(socket: &mut WebSocket) -> Option<v0::GatewayIdentify> {
    // Wait for the client to identify it's self.
    loop {
        // Wait for the next message from the client.
        let Some(revc) = socket.recv().instrument(info_span!("socket_recv")).await else {
            tracing::error!("websocket stream unexpectedly closed!");
            return None;
        };

        // Check if a message or an error was received.
        let message = match revc {
            Ok(message) => message,
            Err(err) => {
                tracing::error!(%err, "error reciving from gateway websocket");
                return None;
            }
        };

        // Decode the identity message sent from the client to the websocket.
        let ident_message = match message {
            ws::Message::Text(text) => {
                // Convert the message to a serde_json::Value.
                let value: Value = axum::Json::from_bytes(text.as_bytes()).unwrap().0;

                // Now that we know the received json struct is valid, actually decode it.
                match serde_path_to_error::deserialize(value) {
                    Ok(v) => v,
                    Err(error) => {
                        tracing::error!(%error,
                            "failed to decode client identity message as JSON"
                        );

                        continue;
                    }
                }
            }
            ws::Message::Binary(bytes) => match v0::GatewayIdentify::decode(bytes) {
                Ok(msg) => msg,
                Err(err) => {
                    tracing::error!(%err, "error reciving client identify message");
                    continue;
                }
            },
            _ => continue,
        };

        return Some(ident_message);
    }
}

/// Task used to handle the sending gateway messages from the session to the client.
async fn task_send(
    mut sender: SplitSink<WebSocket, ws::Message>,
    session: Arc<RwLock<client::Session>>,
    encoding: Encoding,
) {
    // Get a receiver for server-generated gateway events for the session.
    let mut sub = session.read().unwrap().subscribe();

    loop {
        // Wait for the next session event generated by the server
        // that needs to be forwarded to the client session.
        let event: GatewayServerEvent = match sub
            .recv()
            .instrument(info_span!("gateway_socket_wait_server_event"))
            .await
        {
            Ok(event) => event,
            Err(err) => {
                tracing::error!(%err, "failed to receive gateway event from server");
                continue;
            }
        };

        // Encode the event as specified by the encoding
        // query parameter and send it to the client.
        match encoding {
            Encoding::Protobuf => {
                // Encode the event to Protobuf.
                let mut buf = Vec::new();
                buf.reserve(event.encoded_len());
                if let Err(err) = event.encode(&mut buf) {
                    tracing::error!(%err, "failed to encode gateway server event to protobuf");
                    break;
                };

                // Send the encoded Protobuf bytes as a binary message.
                sender.send(ws::Message::Binary(buf.into()))
            }
            Encoding::Json => {
                // Encode the event as a JSON text message.
                let j = serde_json::to_string(&event).unwrap();
                sender.send(ws::Message::Text(j.into()))
            }
        }
        .instrument(debug_span!("gateway_socket_send"))
        .await
        .unwrap();

        continue;
    }

    tracing::info!(session_id = ?session.read().unwrap().session_id(), "gateway to client socket closed");
}

/// Task used to handle ingesting gateway messages from the client.
async fn task_receive(
    mut receiver: SplitStream<WebSocket>,
    session: Arc<RwLock<client::Session>>,
    _encoding: Encoding,
) {
    // Get a channel sender for ingesting received client events to the server.
    let sender = session.read().unwrap().client_event_sender();

    loop {
        // Wait to receive the next message.
        //
        // This returns 'None' in the event the
        // receiver channel has been closed.
        let Some(recv) = receiver
            .next()
            .instrument(info_span!("gateway_socket_recv"))
            .await
        else {
            break;
        };

        // Check if the event receive was valid.
        let Ok(message) = recv else {
            tracing::error!("failed to receive gateway event from client");
            continue;
        };

        // If we get a ping message, update the last-seen for the client session.
        if let ws::Message::Ping(_ping) = message {
            // Update the last-seen timestamp for the client session.
            session.write().unwrap().contacted();
            break;
        }

        tracing::trace!("gateway received encoded client event");

        // Attempt to decode the client event.
        //
        // If the WebSocket message is binary then the message is decoded
        // as Protobuf, if it's text then it'll be decoded as JSON.
        let event: v0::GatewayClientEvent = match message {
            ws::Message::Text(text) => {
                // Convert the message to a serde_json::Value.
                let value: Value = axum::Json::from_bytes(text.as_bytes()).unwrap().0;

                // Now that we know the received json struct is valid, actually decode it.
                match serde_path_to_error::deserialize(value) {
                    Ok(v) => v,
                    Err(error) => {
                        tracing::error!(%error,
                            "failed to decode client identity message as JSON"
                        );

                        continue;
                    }
                }
            }

            ws::Message::Binary(bytes) => match v0::GatewayClientEvent::decode(bytes) {
                Ok(event) => event,
                Err(err) => {
                    tracing::error!(%err,
                        "failed to decode client identity message as Protobuf"
                    );

                    continue;
                }
            },
            _ => continue,
        };

        tracing::trace!(
            event = ?event.clone(),
            "gateway decoded client event");

        // Ingest the decoded event by sending
        // it to the client session worker.
        sender
            .send(event.clone())
            .instrument(info_span!("gateway_ingest_client_event"))
            .await
            .unwrap();

        tracing::trace!(
            event = ?event,
            "gateway ingested decoded client event to session");
    }

    tracing::info!(session_id = ?session.read().unwrap().session_id(), "client to gateway socket closed");
}
