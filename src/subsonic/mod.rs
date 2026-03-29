use log::{info, warn};
use reqwest::{Client, Response, StatusCode, Url};
use serde::de::DeserializeOwned;

use crate::config;
use crate::jellyfin::JellyfinError;
use crate::jellyfin::api::{
    ArtistItemsDto, ImageType, LibraryDto, LibraryDtoList, LyricsResponse, MusicDto, MusicDtoList,
    PlaybackInfo, PlaybackReport, PlaybackReportStatus, PlaylistDtoList, PlaylistItems,
    UserDataDto,
};
use crate::subsonic::api::{Song, SubsonicEnvelope, SubsonicResponse};

pub mod api;

const SUBSONIC_API_VERSION: &str = "1.16.1";
const SUBSONIC_CLIENT_NAME: &str = "gelly";
const ALL_FOLDERS_LIBRARY_ID: &str = "__gelly_subsonic_all__";
const ALBUM_LIST_PAGE_SIZE: u32 = 500;

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
        info!("Subsonic::get_views()");

        let response = self.get_subsonic("getMusicFolders", &[]).await?;
        self.ensure_ok_response(&response)?;

        let mut items = response
            .music_folders
            .map(|folders| {
                folders
                    .music_folders
                    .into_iter()
                    .map(|folder| LibraryDto {
                        id: folder.id,
                        name: folder.name,
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if items.is_empty() {
            items.push(LibraryDto {
                id: ALL_FOLDERS_LIBRARY_ID.to_string(),
                name: "Music".to_string(),
            });
        }

        Ok(LibraryDtoList { items })
    }

    pub async fn get_library(&self, library_id: &str) -> Result<MusicDtoList, JellyfinError> {
        info!("Subsonic::get_library(library_id={library_id})");

        let album_ids = self.get_album_ids(library_id).await?;
        let mut items = Vec::<MusicDto>::new();

        for album_id in album_ids {
            match self.get_album(&album_id).await {
                Ok(mut songs) => items.append(&mut songs),
                Err(err) => warn!("Failed to fetch album {}: {}", album_id, err),
            }
        }

        Ok(MusicDtoList {
            total_record_count: items.len() as u64,
            items,
        })
    }

    async fn get_album_ids(&self, library_id: &str) -> Result<Vec<String>, JellyfinError> {
        let mut album_ids = Vec::new();
        let mut offset: u32 = 0;

        loop {
            let mut params = vec![
                ("type".to_string(), "alphabeticalByName".to_string()),
                ("size".to_string(), ALBUM_LIST_PAGE_SIZE.to_string()),
                ("offset".to_string(), offset.to_string()),
            ];

            let normalized_library_id = library_id.trim();
            if !normalized_library_id.is_empty() && normalized_library_id != ALL_FOLDERS_LIBRARY_ID {
                params.push(("musicFolderId".to_string(), normalized_library_id.to_string()));
            }

            let response = self.get_subsonic("getAlbumList2", &params).await?;
            self.ensure_ok_response(&response)?;

            let page = response
                .album_list2
                .map(|payload| {
                    payload
                        .album
                        .into_iter()
                        .map(|album| album.id)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            if page.is_empty() {
                break;
            }

            let count = page.len() as u32;
            album_ids.extend(page);

            if count < ALBUM_LIST_PAGE_SIZE {
                break;
            }

            offset += count;
        }

        Ok(album_ids)
    }

    async fn get_album(&self, album_id: &str) -> Result<Vec<MusicDto>, JellyfinError> {
        let response = self
            .get_subsonic("getAlbum", &[("id".to_string(), album_id.to_string())])
            .await?;
        self.ensure_ok_response(&response)?;

        let album = response.album.ok_or_else(|| JellyfinError::Http {
            status: StatusCode::BAD_GATEWAY,
            message: "Subsonic response missing album payload".to_string(),
        })?;

        let album_name = album.name.clone();
        let album_id_fallback = album.id.clone();
        let album_artist = album.artist.clone();
        let album_artist_id = album.artist_id.clone();
        let album_year = album.year;
        let album_created = album.created.clone();

        let songs = album
            .song
            .into_iter()
            .map(|song| {
                self.song_to_music_dto(
                    song,
                    album_id_fallback.clone(),
                    album_name.clone(),
                    album_artist.clone(),
                    album_artist_id.clone(),
                    album_year,
                    album_created.clone(),
                )
            })
            .collect();

        Ok(songs)
    }

    fn song_to_music_dto(
        &self,
        song: Song,
        fallback_album_id: String,
        fallback_album_name: String,
        fallback_artist_name: Option<String>,
        fallback_artist_id: Option<String>,
        fallback_year: Option<u32>,
        fallback_created: Option<String>,
    ) -> MusicDto {
        let album = song.album.clone().unwrap_or(fallback_album_name);
        let album_id = song.album_id.clone().unwrap_or(fallback_album_id);

        let artist_name = song
            .album_artist
            .clone()
            .or(song.artist.clone())
            .or(fallback_artist_name)
            .unwrap_or_else(|| "Unknown Artist".to_string());

        let artist_id = song
            .artist_id
            .clone()
            .or(fallback_artist_id)
            .unwrap_or_default();

        let duration_ticks = song.duration.unwrap_or(0).saturating_mul(10_000_000);

        let date_created = song.created.clone().or(fallback_created);
        let production_year = song.year.or(fallback_year);

        MusicDto {
            name: song.title,
            id: song.id,
            date_created,
            run_time_ticks: duration_ticks,
            album,
            album_artists: vec![ArtistItemsDto {
                name: artist_name,
                id: artist_id,
            }],
            album_id,
            normalization_gain: None,
            production_year,
            index_number: song.track,
            parent_index_number: song.disc_number,
            user_data: UserDataDto {
                play_count: song.play_count.unwrap_or(0),
            },
            has_lyrics: false,
        }
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
