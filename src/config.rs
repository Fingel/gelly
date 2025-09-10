use gtk::gio;
use libsecret::{self, Schema, SchemaAttributeType, SchemaFlags};
use std::{cell::RefCell, collections::HashMap};

pub static APP_ID: &str = "io.m51.Gelly";

thread_local! {
    static SETTINGS: RefCell<Option<gio::Settings>> = const { RefCell::new(None) };
}

/// Returns the application settings. Constructor called at most once per thread.
pub fn settings() -> gio::Settings {
    SETTINGS.with(|s| {
        s.borrow_mut()
            .get_or_insert_with(|| gio::Settings::new(APP_ID))
            .clone()
    })
}

/// Application secret schema
fn secret_schema() -> Schema {
    let mut attributes = HashMap::new();
    attributes.insert("host", SchemaAttributeType::String);
    attributes.insert("user-id", SchemaAttributeType::String);
    Schema::new(APP_ID, SchemaFlags::NONE, attributes)
}

//TODO: Remove the expects here and just don't store the token if no secret service is available.
pub fn store_jellyfin_api_token(host: &str, user_id: &str, api_token: &str) {
    let mut attributes = HashMap::new();
    attributes.insert("host", host);
    attributes.insert("user-id", user_id);
    let collection = libsecret::COLLECTION_DEFAULT;
    let schema = secret_schema();
    libsecret::password_store_sync(
        Some(&schema),
        attributes,
        Some(collection),
        "Jellyfin API Token",
        api_token,
        gio::Cancellable::NONE,
    )
    .expect("Unable to store Jellyfin API token");
}

pub fn retrieve_jellyfin_api_token(host: &str, user_id: &str) -> Option<String> {
    let mut attributes = HashMap::new();
    attributes.insert("host", host);
    attributes.insert("user-id", user_id);
    let schema = secret_schema();
    libsecret::password_lookup_sync(Some(&schema), attributes, gio::Cancellable::NONE)
        .expect("Unable to retrieve Jellyfin API token")
        .map(|password| password.to_string())
}

#[allow(unused)]
pub fn clear_jellyfin_api_token(host: &str, user_id: &str) {
    let mut attributes = HashMap::new();
    attributes.insert("host", host);
    attributes.insert("user-id", user_id);
    let schema = secret_schema();
    libsecret::password_clear_sync(Some(&schema), attributes, gio::Cancellable::NONE)
        .expect("Unable to clear Jellyfin API token")
}
