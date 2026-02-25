use std::io::Result;

fn main() -> Result<()> {
    let mut config = prost_build::Config::new();

    // Prost renames fields named `in` to `in_`. But if serialized through serde,
    // they should as `in`.
    config.field_attribute("in", "#[serde(rename = \"in\")]");

    // Add the serde serialization attributes so messages can easily be transcoded to JSON.
    config.type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]");
    config.type_attribute(".", "#[serde(rename_all = \"snake_case\")]");
    config.type_attribute(".", "#[derive(schemars::JsonSchema)]");

    // Encode enums to json with a "type" field instead of a nested object.
    config.type_attribute(
        "v0.gateway.GatewayServerEvent.event",
        "#[serde(tag = \"type\")]",
    );
    config.type_attribute(
        "v0.gateway.GatewayClientEvent.event",
        "#[serde(tag = \"type\")]",
    );

    // Compile the specified protobuf files into Rust code.
    config.compile_protos(&["src/proto/v0/gateway.proto"], &["src/proto"])?;

    Ok(())
}
