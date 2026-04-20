use reqwest::StatusCode;
use thiserror::Error;

use crate::jellyfin::{
    Jellyfin,
    api::{
        FavoriteDtoList, ImageType, LibraryDtoList, LyricsResponse, MusicDtoList, PlaybackInfo,
        PlaybackReport, PlaybackReportStatus, PlaylistDtoList, PlaylistItems,
    },
};
use crate::subsonic::Subsonic;

#[derive(Error, Debug)]
pub enum BackendError {
    #[error("Transport error: {0}")]
    Transport(#[from] reqwest::Error),

    #[error("HTTP error: {status} - {message}")]
    Http { status: StatusCode, message: String },

    #[error("Authentication failed: {message}")]
    AuthenticationFailed { message: String },

    #[error("JSON parsing error: {0}")]
    JsonParsing(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub enum Backend {
    Jellyfin(Jellyfin),
    Subsonic(Subsonic),
}

impl Backend {
    pub fn is_authenticated(&self) -> bool {
        match self {
            Self::Jellyfin(jellyfin) => jellyfin.is_authenticated(),
            Self::Subsonic(subsonic) => subsonic.is_authenticated(),
        }
    }

    pub async fn get_views(&self) -> Result<LibraryDtoList, BackendError> {
        match self {
            Self::Jellyfin(jellyfin) => jellyfin.get_views().await,
            Self::Subsonic(subsonic) => subsonic.get_views().await,
        }
    }

    pub async fn get_library(&self, library_id: &str) -> Result<MusicDtoList, BackendError> {
        match self {
            Self::Jellyfin(jellyfin) => jellyfin.get_library(library_id).await,
            Self::Subsonic(subsonic) => subsonic.get_library(library_id).await,
        }
    }

    pub async fn get_favorites(&self) -> Result<FavoriteDtoList, BackendError> {
        match self {
            Self::Jellyfin(jellyfin) => jellyfin.get_favorites().await,
            Self::Subsonic(subsonic) => subsonic.get_favorites().await,
        }
    }

    pub async fn get_playlists(&self) -> Result<PlaylistDtoList, BackendError> {
        match self {
            Self::Jellyfin(jellyfin) => jellyfin.get_playlists().await,
            Self::Subsonic(subsonic) => subsonic.get_playlists().await,
        }
    }

    pub async fn get_playlist_items(
        &self,
        playlist_id: &str,
    ) -> Result<PlaylistItems, BackendError> {
        match self {
            Self::Jellyfin(jellyfin) => jellyfin.get_playlist_items(playlist_id).await,
            Self::Subsonic(subsonic) => subsonic.get_playlist_items(playlist_id).await,
        }
    }

    pub async fn new_playlist(
        &self,
        name: &str,
        items: Vec<String>,
    ) -> Result<String, BackendError> {
        match self {
            Self::Jellyfin(jellyfin) => jellyfin.new_playlist(name, items).await,
            Self::Subsonic(subsonic) => subsonic.new_playlist(name, items).await,
        }
    }

    pub async fn add_playlist_items(
        &self,
        playlist_id: &str,
        item_ids: &[String],
    ) -> Result<(), BackendError> {
        match self {
            Self::Jellyfin(jellyfin) => jellyfin.add_playlist_items(playlist_id, item_ids).await,
            Self::Subsonic(subsonic) => subsonic.add_playlist_items(playlist_id, item_ids).await,
        }
    }

    pub async fn move_playlist_item(
        &self,
        playlist_id: &str,
        item_id: &str,
        new_index: i32,
    ) -> Result<(), BackendError> {
        match self {
            Self::Jellyfin(jellyfin) => {
                jellyfin
                    .move_playlist_item(playlist_id, item_id, new_index)
                    .await
            }
            Self::Subsonic(subsonic) => {
                subsonic
                    .move_playlist_item(playlist_id, item_id, new_index)
                    .await
            }
        }
    }

    pub async fn remove_playlist_item(
        &self,
        playlist_id: &str,
        item_id: &str,
    ) -> Result<(), BackendError> {
        match self {
            Self::Jellyfin(jellyfin) => jellyfin.remove_playlist_item(playlist_id, item_id).await,
            Self::Subsonic(subsonic) => subsonic.remove_playlist_item(playlist_id, item_id).await,
        }
    }

    pub async fn delete_item(&self, item_id: &str) -> Result<(), BackendError> {
        match self {
            Self::Jellyfin(jellyfin) => jellyfin.delete_item(item_id).await,
            Self::Subsonic(subsonic) => subsonic.delete_item(item_id).await,
        }
    }

    pub async fn request_library_rescan(&self, library_id: &str) -> Result<(), BackendError> {
        match self {
            Self::Jellyfin(jellyfin) => jellyfin.request_library_rescan(library_id).await,
            Self::Subsonic(subsonic) => subsonic.request_library_rescan(library_id).await,
        }
    }

    pub async fn get_image(
        &self,
        item_id: &str,
        image_type: ImageType,
    ) -> Result<Vec<u8>, BackendError> {
        match self {
            Self::Jellyfin(jellyfin) => jellyfin.get_image(item_id, image_type).await,
            Self::Subsonic(subsonic) => subsonic.get_image(item_id, image_type).await,
        }
    }

    pub fn get_stream_uri(&self, item_id: &str) -> String {
        match self {
            Self::Jellyfin(jellyfin) => jellyfin.get_stream_uri(item_id),
            Self::Subsonic(subsonic) => subsonic.get_stream_uri(item_id),
        }
    }

    pub async fn get_playback_info(&self, item_id: &str) -> Result<PlaybackInfo, BackendError> {
        match self {
            Self::Jellyfin(jellyfin) => jellyfin.get_playback_info(item_id).await,
            Self::Subsonic(subsonic) => subsonic.get_playback_info(item_id).await,
        }
    }

    pub async fn playback_report(
        &self,
        report: &PlaybackReport,
        state: &PlaybackReportStatus,
    ) -> Result<(), BackendError> {
        match self {
            Self::Jellyfin(jellyfin) => jellyfin.playback_report(report, state).await,
            Self::Subsonic(subsonic) => subsonic.playback_report(report, state).await,
        }
    }

    pub async fn fetch_lyrics(&self, item_id: &str) -> Result<LyricsResponse, BackendError> {
        match self {
            Self::Jellyfin(jellyfin) => jellyfin.fetch_lyrics(item_id).await,
            Self::Subsonic(subsonic) => subsonic.fetch_lyrics(item_id).await,
        }
    }
}

impl Default for Backend {
    fn default() -> Self {
        Self::Jellyfin(Jellyfin::default())
    }
}
