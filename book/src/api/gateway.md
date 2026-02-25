# Gateway

The gateway is a WebSocket endpoint that a client connects to for exchanging events between the client and the server.

```,ignore
ws(s)://<url>/gateway
```

## Encoding

The gateway protocol is defined in Protobuf, but thanks to the flexibility of `prost` and `serde_json` the gateway supports both Protobuf and JSON messages.

> [!IMPORTANT]  
> The structure of the JSON encoding is slightly modified compared to the default Protobuf encoding. 
>
> Notably, the JSON representation of the `oneof` Protobuf types uses Serde's [enum representation](https://serde.rs/enum-representations.html) settings to represent the JSON objects better for web use.

To set the encoding the client want's to receive from the server, the client can specify the `?encoding=[protobuf|json]` query parameter on the WebSocket connection URL. Note that this only sets the encoding received by the client from the server (see below for server-to-client encoding).

The gateway server automatically decides what encoding to use for messages received from clients based on if they're `text` or `binary` WebSocket messages.
 * If the message is a `binary` message, the server assumes that Protobuf decoding is to be used.
 * If the message is a `text` message, the server assumes that JSON decoding is the be used.

## Versioning

The gateway protocol defined using a Protobuf schema and versioned with semantic versioning.

When a client connects to the gateway a handshake message is sent to the client indicating the latest gateway protocol version that the server supports. A client can also specify the protocol version it wants to talk as a `?version=x` query parameter on the gateway websocket endpoint.

