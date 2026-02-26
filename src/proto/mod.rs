//! The Protobuf schema for the API.
//!
//! The gateway types are also used via `serde_json` to
//! encode and decode the event messages for sockets
//! that request JSON encoding.

pub mod v0 {
    include!(concat!(env!("OUT_DIR"), "/v0.gateway.rs"));
}
