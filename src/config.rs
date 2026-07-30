use gtk::gio;
use gtk::gio::prelude::SettingsExt;
use log::error;
use oo7::{Error, Keyring};
use std::cell::RefCell;
use uuid::Uuid;

pub static APP_ID: &str = "io.m51.Gelly";
pub static VERSION: &str = env!("CARGO_PKG_VERSION");

thread_local! {
    static SETTINGS: RefCell<Option<gio::Settings>> = const { RefCell::new(None) };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackendType {
    #[default]
    Jellyfin,
    Subsonic,
}

impl BackendType {
    pub fn as_str(self) -> &'static str {
        match self {
            BackendType::Jellyfin => "jellyfin",
            BackendType::Subsonic => "subsonic",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "subsonic" => BackendType::Subsonic,
            _ => BackendType::Jellyfin,
        }
    }
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

pub fn get_backend_type() -> BackendType {
    BackendType::from_str(settings().string("backend-type").as_str())
}

pub fn set_backend_type(backend_type: BackendType) {
    settings()
        .set_string("backend-type", backend_type.as_str())
        .expect("Failed to set backend type");
}

/// Sets jellyfin settings to blank values and clears the API token
pub fn logout() -> Result<(), Error> {
    let clear_res = clear_jellyfin_api_token(
        settings().string("hostname").as_str(),
        settings().string("user-id").as_str(),
    );
    settings()
        .set_string("user-id", "")
        .expect("Failed to clear user-id");
    settings()
        .set_string("subsonic-username", "")
        .expect("Failed to clear subsonic-username");
    settings()
        .set_string("library-id", "")
        .expect("Failed to clear library-id");
    settings()
        .set_string("backend-type", BackendType::Jellyfin.as_str())
        .expect("Failed to reset backend-type");

    clear_res
}

pub fn store_jellyfin_api_token(host: &str, user_id: &str, api_token: &str) -> Result<(), Error> {
    async_io::block_on(async {
        let keyring = Keyring::new().await?;
        keyring.unlock().await?;
        let attributes = &[("host", host), ("user-id", user_id)];
        keyring
            .create_item("Jellyfin API Token", attributes, api_token.as_bytes(), true)
            .await
    })
}

pub fn retrieve_jellyfin_api_token(host: &str, user_id: &str) -> Option<String> {
    retrieve_credentials(host, user_id, BackendType::Jellyfin)
}

pub fn clear_jellyfin_api_token(host: &str, user_id: &str) -> Result<(), Error> {
    async_io::block_on(async {
        let keyring = Keyring::new().await?;
        keyring.unlock().await?;
        let attributes = &[("host", host), ("user-id", user_id)];
        keyring.delete(attributes).await?;
        Ok(())
    })
}

pub fn store_subsonic_password(host: &str, username: &str, password: &str) -> Result<(), Error> {
    async_io::block_on(async {
        let keyring = Keyring::new().await?;
        keyring.unlock().await?;
        let attributes = &[("host", host), ("subsonic-username", username)];
        keyring
            .create_item("Subsonic Password", attributes, password.as_bytes(), true)
            .await?;
        Ok(())
    })
}

pub fn retrieve_subsonic_password(host: &str, username: &str) -> Option<String> {
    retrieve_credentials(host, username, BackendType::Subsonic)
}

fn retrieve_credentials(host: &str, identifier: &str, backend_type: BackendType) -> Option<String> {
    let identifier_key = match backend_type {
        BackendType::Jellyfin => "user-id",
        BackendType::Subsonic => "subsonic-username",
    };
    let result: Result<Option<String>, Error> = async_io::block_on(async {
        let keyring = Keyring::new().await?;
        keyring.unlock().await?;
        let attributes = &[("host", host), (identifier_key, identifier)];
        let items = keyring.search_items(attributes).await?;
        let Some(item) = items.first() else {
            return Ok(None);
        };
        let secret = item.secret().await?;
        Ok(Some(
            String::from_utf8_lossy(secret.as_bytes()).into_owned(),
        ))
    });
    match result {
        Ok(secret) => secret,
        Err(err) => {
            error!(
                "Failed to retrieve {} credentials: {err}",
                backend_type.as_str()
            );
            None
        }
    }
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
    if value == 0 {
        None
    } else {
        if get_backend_type() == BackendType::Jellyfin {
            Some(value.saturating_mul(1000))
        } else {
            Some(value)
        }
    }
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

pub fn get_playlist_favorites_enabled() -> bool {
    settings().boolean("playlist-favorites-enabled")
}

pub fn get_normalize_audio_enabled() -> bool {
    settings().boolean("normalize-audio")
}

pub fn get_gapless_playback_enabled() -> bool {
    settings().boolean("gapless-playback")
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

pub fn get_volume() -> f64 {
    settings().double("volume")
}

pub fn set_volume(volume: f64) {
    settings().set_double("volume", volume).unwrap();
}

pub fn get_playback_mode() -> u32 {
    settings().uint("playback-mode")
}

pub fn set_playback_mode(mode: u32) {
    settings().set_uint("playback-mode", mode).unwrap();
}

pub fn get_compact_mode_enabled() -> bool {
    settings().boolean("compact-mode")
}

pub fn get_album_art_window_background_enabled() -> bool {
    settings().boolean("album-art-window-background")
}
