use gtk::gio::prelude::SettingsExt;
use gtk::{gio, glib::Error};
use std::{cell::RefCell, collections::HashMap};
use uuid::Uuid;

use crate::secret::{self};

pub static APP_ID: &str = "io.m51.Gelly";
pub static VERSION: &str = env!("CARGO_PKG_VERSION");

thread_local! {
    static SETTINGS: RefCell<Option<gio::Settings>> = const { RefCell::new(None) };
}

#[derive(Debug, Clone, PartialEq)]
pub struct TranscodingProfile {
    pub name: &'static str,
    pub codec: &'static str,
    pub container: &'static str,
}

impl TranscodingProfile {
    pub const OPUS_MP4: Self = Self {
        name: "OPUS+MP4",
        codec: "opus",
        container: "mp4",
    };

    pub const AAC_TS: Self = Self {
        name: "AAC+TS",
        codec: "aac",
        container: "ts",
    };

    pub const PROFILES: [Self; 2] = [Self::OPUS_MP4, Self::AAC_TS];

    pub fn as_string_list() -> gtk::StringList {
        let names: Vec<&str> = Self::PROFILES.iter().map(|p| p.name).collect();
        gtk::StringList::new(&names)
    }
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
    secret::clear_secrets();
    settings()
        .set_string("user-id", "")
        .expect("Failed to clear user-id");
    settings()
        .set_string("library-id", "")
        .expect("Failed to clear library-id");
}

pub async fn store_jellyfin_api_token(token: &str) -> Result<(), Error> {
    let mut properties: HashMap<String, String> = HashMap::new();
    properties.insert("token".into(), token.into());
    secret::save_secrets(properties)
        .await
        .expect("Could not save secrets");
    Ok(())
}

pub async fn retrieve_jellyfin_api_token() -> Option<String> {
    let ss = secret::load_secrets()
        .await
        .expect("Could not load secrets");
    ss.get("token").cloned()
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

pub fn get_transcoding_profile() -> TranscodingProfile {
    let profile_name = settings().string("transcoding-profile");
    TranscodingProfile::PROFILES
        .iter()
        .find(|&p| p.name == profile_name)
        .unwrap_or(&TranscodingProfile::OPUS_MP4)
        .clone()
}

pub fn set_transcoding_profile(profile: TranscodingProfile) {
    settings()
        .set_string("transcoding-profile", profile.name)
        .unwrap();
}

pub fn get_max_bitrate() -> Option<i32> {
    // from settings as kbps
    let value = settings().int("max-bitrate");
    if value == 0 { None } else { Some(value * 1000) }
}

pub fn get_refresh_on_startup() -> bool {
    settings().boolean("refresh-on-startup")
}

pub fn get_playlist_shuffle_enabled() -> bool {
    settings().boolean("playlist-shuffle-enabled")
}

pub fn get_playlist_most_played_enabled() -> bool {
    settings().boolean("playlist-most-played-enabled")
}

pub fn get_normalize_audio_enabled() -> bool {
    settings().boolean("normalize-audio")
}

pub fn get_inhibit_suspend_enabled() -> bool {
    settings().boolean("inhibit-suspend")
}

pub fn get_albums_sort_by() -> u32 {
    settings().uint("sort-albums-by")
}

pub fn set_albums_sort_by(sort_by: u32) {
    settings().set_uint("sort-albums-by", sort_by).unwrap();
}

pub fn get_albums_sort_direction() -> u32 {
    settings().uint("sort-albums-direction")
}

pub fn set_albums_sort_direction(direction: u32) {
    settings()
        .set_uint("sort-albums-direction", direction)
        .unwrap();
}

pub fn get_artists_sort_by() -> u32 {
    settings().uint("sort-artists-by")
}

pub fn set_artists_sort_by(sort_by: u32) {
    settings().set_uint("sort-artists-by", sort_by).unwrap();
}

pub fn get_artists_sort_direction() -> u32 {
    settings().uint("sort-artists-direction")
}

pub fn set_artists_sort_direction(direction: u32) {
    settings()
        .set_uint("sort-artists-direction", direction)
        .unwrap();
}

pub fn get_playlists_sort_by() -> u32 {
    settings().uint("sort-playlists-by")
}

pub fn set_playlists_sort_by(sort_by: u32) {
    settings().set_uint("sort-playlists-by", sort_by).unwrap();
}

pub fn get_playlists_sort_direction() -> u32 {
    settings().uint("sort-playlists-direction")
}

pub fn set_playlists_sort_direction(direction: u32) {
    settings()
        .set_uint("sort-playlists-direction", direction)
        .unwrap();
}
