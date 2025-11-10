use dbus_secret_service::{EncryptionType, SecretService};
use gtk::gio;
use gtk::gio::prelude::SettingsExt;
use std::{cell::RefCell, collections::HashMap};
use uuid::Uuid;

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

/// Sets jellyfin settings to blank values and clears the API token
pub fn logout() {
    clear_jellyfin_api_token(
        settings().string("hostname").as_str(),
        settings().string("user-id").as_str(),
    );
    settings()
        .set_string("user-id", "")
        .expect("Failed to clear user-id");
    settings()
        .set_string("library-id", "")
        .expect("Failed to clear library-id");
}

pub fn store_jellyfin_api_token(host: &str, user_id: &str, api_token: &str) {
    let ss = SecretService::connect(EncryptionType::Plain).unwrap();
    let collection = ss.get_default_collection().unwrap();
    let mut properties = HashMap::new();
    properties.insert("host", host);
    properties.insert("user-id", user_id);
    collection
        .create_item(
            "Jellyfin API Token",
            properties,
            api_token.as_bytes(),
            true,
            "text/plain",
        )
        .expect("Failed to store API token");
}

pub fn retrieve_jellyfin_api_token(host: &str, user_id: &str) -> Option<String> {
    let ss =
        SecretService::connect(EncryptionType::Plain).expect("Could not connect to secret service");

    let search_items = ss
        .search_items(HashMap::from([("host", host), ("user-id", user_id)]))
        .unwrap();

    let item = match search_items.unlocked.first() {
        Some(item) => item,
        None => {
            // if there aren't any, try to unlock them
            if let Some(locked_item) = search_items.locked.first() {
                locked_item.unlock().unwrap();
                locked_item
            } else {
                return None;
            }
        }
    };

    let secret = item
        .get_secret()
        .expect("Unable to retrieve secret from keyring");
    Some(String::from_utf8(secret).unwrap())
}

pub fn clear_jellyfin_api_token(host: &str, user_id: &str) {
    let ss =
        SecretService::connect(EncryptionType::Plain).expect("Could not connect to secret service");

    let search_items = ss
        .search_items(HashMap::from([("host", host), ("user-id", user_id)]))
        .unwrap();

    let item = match search_items.unlocked.first() {
        Some(item) => item,
        None => {
            // if there aren't any, try to unlock them
            if let Some(locked_item) = search_items.locked.first() {
                locked_item.unlock().unwrap();
                locked_item
            } else {
                return;
            }
        }
    };
    item.delete().expect("Unable to remove secret from keyring");
}

/// Return the client UUID, generating it if it doesn't exist
pub fn application_uuid() -> String {
    let uuid = settings().string("uuid").as_str().to_string();
    if uuid.is_empty() {
        let uuid = Uuid::new_v4().to_string();
        settings().set_string("uuid", &uuid).unwrap();
        uuid
    } else {
        uuid
    }
}
