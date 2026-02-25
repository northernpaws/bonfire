use bonfire::proto::v0;
use schemars::schema_for;

fn main() {
    // Schema of the JSON message used to send data
    // from the gateway server to the client.
    let gateway_server_event = schema_for!(v0::gateway_server_event::Event);

    // Schema of the JSON message used to send data
    // from the client to the gateway server.
    let gateway_client_event = schema_for!(v0::gateway_client_event::Event);
}
