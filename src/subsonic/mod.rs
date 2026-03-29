use log::info;
use reqwest::{Client, Response, StatusCode, Url};
use serde::de::DeserializeOwned;

use crate::config;
use crate::jellyfin::JellyfinError;
use crate::jellyfin::api::{
    ImageType, LibraryDto, LibraryDtoList, LyricsResponse, MusicDtoList, PlaybackInfo,
    PlaybackReport, PlaybackReportStatus, PlaylistDtoList, PlaylistItems,
};
use crate::subsonic::api::{SubsonicEnvelope, SubsonicResponse};

pub mod api;

const SUBSONIC_API_VERSION: &str = "1.16.1";
const SUBSONIC_CLIENT_NAME: &str = "gelly";

#[derive(Debug, Clone)]
pub struct Subsonic {
    client: Client,
    pub host: String,
    pub username: String,
    pub password: String,
}

impl Subsonic {
    pub fn new(host: &str, username: &str, password: &str) -> Self {
        let client = Client::builder()
            .user_agent(format!("Gelly/{}", config::VERSION))
            .build()
            .expect("Failed to create HTTP client");

        info!("Subsonic::new(host={host}, username={username})");
        Self {
            client,
            host: host.to_string(),
            username: username.to_string(),
            password: password.to_string(),
        }
    }

    pub fn is_authenticated(&self) -> bool {
        info!("Subsonic::is_authenticated()");
        !self.host.is_empty() && !self.username.is_empty() && !self.password.is_empty()
    }

    pub async fn new_authenticate(
        host: &str,
        username: &str,
        password: &str,
    ) -> Result<Self, JellyfinError> {
        info!("Subsonic::new_authenticate(host={host}, username={username}) [stub]");

        if host.trim().is_empty() || username.trim().is_empty() || password.trim().is_empty() {
            return Err(JellyfinError::AuthenticationFailed {
                message: "Missing host/username/password".to_string(),
            });
        }

        let subsonic = Self::new(host, username, password);
        subsonic.ping().await?;
        Ok(subsonic)
    }

    async fn ping(&self) -> Result<(), JellyfinError> {
        let response = self.get_subsonic("ping", &[]).await?;
        self.ensure_ok_response(&response)?;
        Ok(())
    }

    pub async fn get_views(&self) -> Result<LibraryDtoList, JellyfinError> {
        info!("Subsonic::get_views() [stub]");
        Ok(LibraryDtoList {
            items: vec![LibraryDto {
                id: "stub-music".to_string(),
                name: "Music".to_string(),
            }],
        })
    }

    pub async fn get_library(&self, library_id: &str) -> Result<MusicDtoList, JellyfinError> {
        info!("Subsonic::get_library(library_id={library_id}) [stub]");
        Ok(MusicDtoList {
            items: vec![],
            total_record_count: 0,
        })
    }

    pub async fn get_playlists(&self) -> Result<PlaylistDtoList, JellyfinError> {
        info!("Subsonic::get_playlists() [stub]");
        Ok(PlaylistDtoList {
            items: vec![],
            total_record_count: 0,
        })
    }

    pub async fn get_playlist_items(
        &self,
        playlist_id: &str,
    ) -> Result<PlaylistItems, JellyfinError> {
        info!("Subsonic::get_playlist_items(playlist_id={playlist_id}) [stub]");
        Ok(PlaylistItems {
            items: vec![],
            total_record_count: 0,
        })
    }

    pub async fn new_playlist(
        &self,
        name: &str,
        _items: Vec<String>,
    ) -> Result<String, JellyfinError> {
        info!("Subsonic::new_playlist(name={name}) [stub]");
        Ok(format!("stub-playlist-{name}"))
    }

    pub async fn add_playlist_items(
        &self,
        playlist_id: &str,
        item_ids: &[String],
    ) -> Result<(), JellyfinError> {
        info!(
            "Subsonic::add_playlist_items(playlist_id={playlist_id}, count={}) [stub]",
            item_ids.len()
        );
        Ok(())
    }

    pub async fn move_playlist_item(
        &self,
        playlist_id: &str,
        item_id: &str,
        new_index: i32,
    ) -> Result<(), JellyfinError> {
        info!(
            "Subsonic::move_playlist_item(playlist_id={playlist_id}, item_id={item_id}, new_index={new_index}) [stub]"
        );
        Ok(())
    }

    pub async fn remove_playlist_item(
        &self,
        playlist_id: &str,
        item_id: &str,
    ) -> Result<(), JellyfinError> {
        info!(
            "Subsonic::remove_playlist_item(playlist_id={playlist_id}, item_id={item_id}) [stub]"
        );
        Ok(())
    }

    pub async fn delete_item(&self, item_id: &str) -> Result<(), JellyfinError> {
        info!("Subsonic::delete_item(item_id={item_id}) [stub]");
        Ok(())
    }

    pub async fn request_library_rescan(&self, library_id: &str) -> Result<(), JellyfinError> {
        info!("Subsonic::request_library_rescan(library_id={library_id}) [stub]");
        Ok(())
    }

    pub async fn get_image(
        &self,
        item_id: &str,
        image_type: ImageType,
    ) -> Result<Vec<u8>, JellyfinError> {
        info!(
            "Subsonic::get_image(item_id={item_id}, image_type={}) [stub]",
            image_type.as_str()
        );

        // Return a "not found"-style error so existing UI fallback paths stay in control.
        Err(JellyfinError::Http {
            status: StatusCode::NOT_FOUND,
            message: "Subsonic skeleton has no image support".to_string(),
        })
    }

    pub fn get_stream_uri(&self, item_id: &str) -> String {
        info!("Subsonic::get_stream_uri(item_id={item_id}) [stub]");

        // Returning a harmless placeholder URI for now.
        // Playback is expected to fail until real stream mapping is implemented.
        "about:blank".to_string()
    }

    pub async fn get_playback_info(&self, item_id: &str) -> Result<PlaybackInfo, JellyfinError> {
        info!("Subsonic::get_playback_info(item_id={item_id}) [stub]");
        Ok(PlaybackInfo {
            media_sources: vec![],
        })
    }

    pub async fn playback_report(
        &self,
        report: &PlaybackReport,
        state: &PlaybackReportStatus,
    ) -> Result<(), JellyfinError> {
        let state_name = match state {
            PlaybackReportStatus::Started => "Started",
            PlaybackReportStatus::InProgress => "InProgress",
            PlaybackReportStatus::Stopped => "Stopped",
        };
        info!(
            "Subsonic::playback_report(item_id={}, state={state_name}) [stub]",
            report.item_id
        );
        Ok(())
    }

    pub async fn fetch_lyrics(&self, item_id: &str) -> Result<LyricsResponse, JellyfinError> {
        info!("Subsonic::fetch_lyrics(item_id={item_id}) [stub]");
        Ok(LyricsResponse { lyrics: vec![] })
    }

    async fn get_subsonic(
        &self,
        endpoint: &str,
        extra_params: &[(String, String)],
    ) -> Result<SubsonicResponse, JellyfinError> {
        let envelope: SubsonicEnvelope = self.get_json(endpoint, extra_params).await?;
        Ok(envelope.response)
    }

    async fn get_json<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        extra_params: &[(String, String)],
    ) -> Result<T, JellyfinError> {
        let url = self.rest_url(endpoint);
        let mut params = self.auth_params();
        params.extend_from_slice(extra_params);

        let response = self.client.get(url).query(&params).send().await?;
        let body = self.handle_http_response(response).await?;
        Ok(serde_json::from_str::<T>(&body)?)
    }

    fn ensure_ok_response(&self, response: &SubsonicResponse) -> Result<(), JellyfinError> {
        if response.is_ok() {
            return Ok(());
        }

        if let Some(error) = &response.error {
            return Err(self.map_api_error(error.code, error.message.clone()));
        }

        Err(JellyfinError::Http {
            status: StatusCode::BAD_GATEWAY,
            message: "Subsonic API returned non-ok status".to_string(),
        })
    }

    fn map_api_error(&self, code: i32, message: String) -> JellyfinError {
        match code {
            40 => JellyfinError::AuthenticationFailed { message },
            _ => JellyfinError::Http {
                status: StatusCode::BAD_GATEWAY,
                message: format!("Subsonic error {}: {}", code, message),
            },
        }
    }

    fn auth_params(&self) -> Vec<(String, String)> {
        vec![
            ("u".to_string(), self.username.clone()),
            ("p".to_string(), self.password.clone()),
            ("v".to_string(), SUBSONIC_API_VERSION.to_string()),
            ("c".to_string(), SUBSONIC_CLIENT_NAME.to_string()),
            ("f".to_string(), "json".to_string()),
        ]
    }

    fn rest_url(&self, endpoint: &str) -> Url {
        let host = self.host.trim_end_matches('/');
        let endpoint = endpoint
            .trim_start_matches('/')
            .trim_end_matches(".view");
        Url::parse(&format!("{host}/rest/{endpoint}.view"))
            .expect("Failed to construct Subsonic endpoint URL")
    }

    async fn handle_http_response(&self, response: Response) -> Result<String, JellyfinError> {
        let status = response.status();
        let body = response.text().await?;
        if status.is_success() {
            Ok(body)
        } else if status == StatusCode::UNAUTHORIZED {
            Err(JellyfinError::AuthenticationFailed { message: body })
        } else {
            Err(JellyfinError::Http {
                status,
                message: body,
            })
        }
    }
}

impl Default for Subsonic {
    fn default() -> Self {
        Self::new("", "", "")
    }
}
