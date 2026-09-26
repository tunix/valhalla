//! API key storage in the Secret Service (GNOME Keyring).

use std::collections::HashMap;

use libsecret::{
    password_clear_sync, password_lookup_sync, password_store_sync, Schema, SchemaAttributeType,
    SchemaFlags,
};

const SCHEMA_NAME: &str = "io.github.tunix.valhalla.key";

fn schema() -> Schema {
    Schema::new(
        SCHEMA_NAME,
        SchemaFlags::NONE,
        HashMap::from([("source", SchemaAttributeType::String)]),
    )
}

pub fn get(source: &str) -> Option<String> {
    let attributes: HashMap<&str, &str> = HashMap::from([("source", source)]);
    password_lookup_sync(Some(&schema()), attributes, gio::Cancellable::NONE)
        .ok()
        .flatten()
        .map(|s| s.to_string())
}

pub fn set(source: &str, key: &str) -> Result<(), String> {
    let attributes: HashMap<&str, &str> = HashMap::from([("source", source)]);
    password_store_sync(
        Some(&schema()),
        attributes,
        None,
        "Valhalla — API key",
        key,
        gio::Cancellable::NONE,
    )
    .map_err(|e| e.to_string())
}

pub fn clear(source: &str) {
    let attributes: HashMap<&str, &str> = HashMap::from([("source", source)]);
    let _ = password_clear_sync(Some(&schema()), attributes, gio::Cancellable::NONE);
}
