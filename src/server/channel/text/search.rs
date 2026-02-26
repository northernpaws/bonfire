//! Full-text search functionality of text channel messages.

use tantivy::schema::Schema;

// keys used for the full-text schema fields.
pub const SCHEMA_KEY_TIMESTAMP: &str = "timestamp";
pub const SCHEMA_KEY_CONTENT: &str = "content";
pub const SCHEMA_KEY_AUTHOR: &str = "author";

/// Builds the schema used by the full text search database.
pub fn text_search_schema() -> Schema {
    let mut schema_builder = Schema::builder();

    // Add the ßtimestamp as an indexed field that we can reference later for retriving
    // (ranges) of messages from the time-series database using text-search query results.
    //
    // Note that we assume the timestamps stored will be in UTC, no timezone conversions are performed.
    schema_builder.add_date_field(
        SCHEMA_KEY_TIMESTAMP,
        tantivy::schema::DateOptions::from(tantivy::schema::INDEXED)
            .set_stored() // needs to be stored so we can reference it later
            .set_fast() // will be random-accessed lots
            // TODO: we may want to increase the datetime precision to better-handle fast logging.
            .set_precision(tantivy::schema::DateTimePrecision::Seconds),
    );

    // Add the message body as a tokenized "text" field.
    //
    // TODO: we should probably drop the STORED attribute and instead
    // introduce a layer where text search results are retrieved from
    // the LSM time series database by their timestamps for speed.
    // Retriving stored documents from the text search engine is slow.
    schema_builder.add_text_field(
        SCHEMA_KEY_CONTENT,
        tantivy::schema::TEXT | tantivy::schema::STORED,
    );

    // Add the message autor as a tokenized field.
    schema_builder.add_u64_field(
        SCHEMA_KEY_AUTHOR,
        tantivy::schema::NumericOptions::from(tantivy::schema::INDEXED).set_fast(), // will be random-accessed lots,
    );

    schema_builder.build()
}
