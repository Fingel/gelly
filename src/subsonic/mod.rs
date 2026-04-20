use log::{debug, warn};
use rand::RngExt;
use reqwest::{Client, Response, StatusCode, Url};
use serde::de::DeserializeOwned;

use crate::backend::BackendError;
use crate::config;
use crate::jellyfin::api::{
    ArtistItemsDto, FavoriteDto, FavoriteDtoList, FavoriteUserDataDto, ImageType, ItemType,
    LibraryDto, LibraryDtoList, LyricsResponse, MediaSource, MediaStream, MusicDto, MusicDtoList,
    PlaybackInfo, PlaybackReport, PlaybackReportStatus, PlaylistDtoList, PlaylistItems,
    UserDataDto,
};
use crate::subsonic::api::{Song, SubsonicEnvelope, SubsonicResponse};

pub mod api;

const SUBSONIC_API_VERSION: &str = "1.15.0";
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

struct AlbumFallback {
    album_id: Option<String>,
    album_name: Option<String>,
    artist_name: Option<String>,
    artist_id: Option<String>,
    year: Option<u32>,
    created: Option<String>,
}

impl Subsonic {
    pub fn new(host: &str, username: &str, password: &str) -> Self {
        let client = Client::builder()
            .user_agent(format!("Gelly/{}", config::VERSION))
            .build()
            .expect("Failed to create HTTP client");

        debug!("Subsonic::new(host={host}, username={username})");
        Self {
            client,
            host: host.to_string(),
            username: username.to_string(),
            password: password.to_string(),
        }
    }

    pub fn is_authenticated(&self) -> bool {
        debug!("Subsonic::is_authenticated()");
        !self.host.is_empty() && !self.username.is_empty() && !self.password.is_empty()
    }

    pub async fn new_authenticate(
        host: &str,
        username: &str,
        password: &str,
    ) -> Result<Self, BackendError> {
        debug!("Subsonic::new_authenticate(host={host}, username={username})");

        if host.trim().is_empty() || username.trim().is_empty() || password.trim().is_empty() {
            return Err(BackendError::AuthenticationFailed {
                message: "Missing host/username/password".to_string(),
            });
        }

        let subsonic = Self::new(host, username, password);
        subsonic.ping().await?;
        Ok(subsonic)
    }

    // https://github.com/opensubsonic/open-subsonic-api/blob/main/content/en/docs/Endpoints/ping.md
    async fn ping(&self) -> Result<(), BackendError> {
        let response = self.get_subsonic("ping", &[]).await?;
        self.ensure_ok_response(&response)?;
        Ok(())
    }

    // https://github.com/opensubsonic/open-subsonic-api/blob/main/content/en/docs/Endpoints/getmusicfolders.md
    pub async fn get_views(&self) -> Result<LibraryDtoList, BackendError> {
        debug!("Subsonic::get_views()");

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

    pub async fn get_library(&self, library_id: &str) -> Result<MusicDtoList, BackendError> {
        debug!("Subsonic::get_library(library_id={library_id})");

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

    // https://opensubsonic.netlify.app/docs/endpoints/getstarred2/
    pub async fn get_favorites(&self) -> Result<FavoriteDtoList, BackendError> {
        let response = self.get_subsonic("getStarred2", &[]).await?;
        self.ensure_ok_response(&response)?;

        let mut items = Vec::new();
        if let Some(starred) = response.starred2 {
            for i in starred.song {
                items.push(FavoriteDto {
                    id: i.id,
                    item_type: ItemType::Audio,
                    user_data: FavoriteUserDataDto { is_favorite: true },
                });
            }
            for i in starred.album {
                items.push(FavoriteDto {
                    id: i.id,
                    item_type: ItemType::MusicAlbum,
                    user_data: FavoriteUserDataDto { is_favorite: true },
                });
            }
            for i in starred.artist {
                items.push(FavoriteDto {
                    id: i.id,
                    item_type: ItemType::MusicArtist,
                    user_data: FavoriteUserDataDto { is_favorite: true },
                });
            }
        }

        Ok(FavoriteDtoList { items })
    }

    // https://github.com/opensubsonic/open-subsonic-api/blob/main/content/en/docs/Endpoints/getalbumlist2.md
    async fn get_album_ids(&self, library_id: &str) -> Result<Vec<String>, BackendError> {
        let mut album_ids = Vec::new();
        let mut offset: u32 = 0;

        loop {
            let mut params = vec![
                ("type".to_string(), "alphabeticalByName".to_string()),
                ("size".to_string(), ALBUM_LIST_PAGE_SIZE.to_string()),
                ("offset".to_string(), offset.to_string()),
            ];

            let normalized_library_id = library_id.trim();
            if !normalized_library_id.is_empty() && normalized_library_id != ALL_FOLDERS_LIBRARY_ID
            {
                params.push((
                    "musicFolderId".to_string(),
                    normalized_library_id.to_string(),
                ));
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

    // https://github.com/opensubsonic/open-subsonic-api/blob/main/content/en/docs/Endpoints/getalbum.md
    async fn get_album(&self, album_id: &str) -> Result<Vec<MusicDto>, BackendError> {
        let response = self
            .get_subsonic("getAlbum", &[("id".to_string(), album_id.to_string())])
            .await?;
        self.ensure_ok_response(&response)?;

        let album = response.album.ok_or_else(|| BackendError::Http {
            status: StatusCode::BAD_GATEWAY,
            message: "Subsonic response missing album payload".to_string(),
        })?;

        let fallback = AlbumFallback {
            album_id: Some(album.id.clone()),
            album_name: Some(album.name.clone()),
            artist_name: album.artist.clone(),
            artist_id: album.artist_id.clone(),
            year: album.year,
            created: album.created.clone(),
        };

        let songs = album
            .song
            .into_iter()
            .map(|song| self.song_to_music_dto(song, &fallback))
            .collect();

        Ok(songs)
    }

    fn song_to_music_dto(&self, song: Song, fallback: &AlbumFallback) -> MusicDto {
        let album = song.album.or_else(|| fallback.album_name.clone());
        let album_id = song.album_id.or_else(|| fallback.album_id.clone());

        let artist_name = song
            .album_artist
            .or(song.artist)
            .or(fallback.artist_name.clone())
            .unwrap_or_else(|| "Unknown Artist".to_string());

        let artist_id = song
            .artist_id
            .or(fallback.artist_id.clone())
            .unwrap_or_default();

        let duration_ticks = song.duration.unwrap_or(0).saturating_mul(10_000_000);

        let date_created = song.created.or_else(|| fallback.created.clone());
        let production_year = song.year.or(fallback.year);

        let normalization_gain = song
            .replay_gain
            .and_then(|rg| rg.track_gain.or(rg.album_gain).or(rg.base_gain));

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
            normalization_gain,
            production_year,
            index_number: song.track,
            parent_index_number: song.disc_number,
            user_data: UserDataDto {
                play_count: song.play_count.unwrap_or(0),
            },
            // there's no eqvivalent to that except for doing another API call
            // so it'll be `true` and just show an empty window for the time being
            has_lyrics: true,
        }
    }

    // https://github.com/opensubsonic/open-subsonic-api/blob/main/content/en/docs/Endpoints/getplaylists.md
    pub async fn get_playlists(&self) -> Result<PlaylistDtoList, BackendError> {
        debug!("Subsonic::get_playlists()");

        let response = self.get_subsonic("getPlaylists", &[]).await?;
        self.ensure_ok_response(&response)?;

        let items = response
            .playlists
            .map(|payload| {
                payload
                    .playlist
                    .into_iter()
                    .map(|playlist| crate::jellyfin::api::PlaylistDto {
                        id: playlist.id,
                        name: playlist.name,
                        child_count: playlist.song_count.unwrap_or(0),
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(PlaylistDtoList {
            total_record_count: items.len() as u64,
            items,
        })
    }

    // https://github.com/opensubsonic/open-subsonic-api/blob/main/content/en/docs/Endpoints/getplaylist.md
    pub async fn get_playlist_items(
        &self,
        playlist_id: &str,
    ) -> Result<PlaylistItems, BackendError> {
        debug!("Subsonic::get_playlist_items(playlist_id={playlist_id})");

        let response = self
            .get_subsonic(
                "getPlaylist",
                &[("id".to_string(), playlist_id.to_string())],
            )
            .await?;
        self.ensure_ok_response(&response)?;

        let playlist = response.playlist.ok_or_else(|| BackendError::Http {
            status: StatusCode::BAD_GATEWAY,
            message: "Subsonic response missing playlist payload".to_string(),
        })?;

        let album_fallback = AlbumFallback {
            album_id: None,
            album_name: None,
            artist_name: None,
            artist_id: None,
            year: None,
            created: None,
        };

        let items = playlist
            .entry
            .into_iter()
            .map(|song| self.song_to_music_dto(song, &album_fallback))
            .collect::<Vec<_>>();

        Ok(PlaylistItems {
            total_record_count: items.len() as u64,
            items,
        })
    }

    // https://github.com/opensubsonic/open-subsonic-api/blob/main/content/en/docs/Endpoints/createplaylist.md
    pub async fn new_playlist(
        &self,
        name: &str,
        items: Vec<String>,
    ) -> Result<String, BackendError> {
        debug!("Subsonic::new_playlist(name={name})");
        let mut params = vec![("name".to_string(), name.to_string())];

        for item_id in &items {
            params.push(("songId".to_string(), item_id.clone()));
        }

        let response = self.get_subsonic("createPlaylist", &params).await?;
        self.ensure_ok_response(&response)?;

        let playlist = response.playlist.ok_or_else(|| BackendError::Http {
            status: StatusCode::BAD_GATEWAY,
            message: "Subsonic createPlaylist response missing playlist payload".to_string(),
        })?;

        Ok(playlist.id)
    }

    // https://github.com/opensubsonic/open-subsonic-api/blob/main/content/en/docs/Endpoints/updateplaylist.md
    pub async fn add_playlist_items(
        &self,
        playlist_id: &str,
        item_ids: &[String],
    ) -> Result<(), BackendError> {
        debug!(
            "Subsonic::add_playlist_items(playlist_id={playlist_id}, count={})",
            item_ids.len()
        );

        let mut params = vec![("playlistId".to_string(), playlist_id.to_string())];

        for item_id in item_ids {
            params.push(("songIdToAdd".to_string(), item_id.clone()));
        }

        let response = self.get_subsonic("updatePlaylist", &params).await?;
        self.ensure_ok_response(&response)?;
        Ok(())
    }

    // https://github.com/opensubsonic/open-subsonic-api/blob/main/content/en/docs/Endpoints/getplaylist.md
    // https://github.com/opensubsonic/open-subsonic-api/blob/main/content/en/docs/Endpoints/createplaylist.md
    pub async fn move_playlist_item(
        &self,
        playlist_id: &str,
        item_id: &str,
        new_index: i32,
    ) -> Result<(), BackendError> {
        debug!(
            "Subsonic::move_playlist_item(playlist_id={playlist_id}, item_id={item_id}, new_index={new_index})"
        );

        let response = self
            .get_subsonic(
                "getPlaylist",
                &[("id".to_string(), playlist_id.to_string())],
            )
            .await?;
        self.ensure_ok_response(&response)?;

        let playlist = response.playlist.ok_or_else(|| BackendError::Http {
            status: StatusCode::BAD_GATEWAY,
            message: "Subsonic getPlaylist response missing playlist payload".to_string(),
        })?;

        let mut song_ids: Vec<String> = playlist.entry.into_iter().map(|s| s.id).collect();
        if let Some(current_pos) = song_ids.iter().position(|id| id == item_id) {
            song_ids.remove(current_pos);
            let insert_at = (new_index as usize).min(song_ids.len());
            song_ids.insert(insert_at, item_id.to_string());
        }

        let mut params = vec![("playlistId".to_string(), playlist_id.to_string())];
        for id in &song_ids {
            params.push(("songId".to_string(), id.clone()));
        }

        let response = self.get_subsonic("createPlaylist", &params).await?;
        self.ensure_ok_response(&response)?;
        Ok(())
    }

    // https://github.com/opensubsonic/open-subsonic-api/blob/main/content/en/docs/Endpoints/getplaylist.md
    // https://github.com/opensubsonic/open-subsonic-api/blob/main/content/en/docs/Endpoints/updateplaylist.md
    pub async fn remove_playlist_item(
        &self,
        playlist_id: &str,
        item_id: &str,
    ) -> Result<(), BackendError> {
        debug!("Subsonic::remove_playlist_item(playlist_id={playlist_id}, item_id={item_id})");

        let response = self
            .get_subsonic(
                "getPlaylist",
                &[("id".to_string(), playlist_id.to_string())],
            )
            .await?;
        self.ensure_ok_response(&response)?;

        let playlist = response.playlist.ok_or_else(|| BackendError::Http {
            status: StatusCode::BAD_GATEWAY,
            message: "Subsonic getPlaylist response missing playlist payload".to_string(),
        })?;

        let index = playlist
            .entry
            .iter()
            .position(|s| s.id == item_id)
            .ok_or_else(|| BackendError::Http {
                status: StatusCode::NOT_FOUND,
                message: format!("Item {item_id} not found in playlist {playlist_id}"),
            })?;

        let params = vec![
            ("playlistId".to_string(), playlist_id.to_string()),
            ("songIndexToRemove".to_string(), index.to_string()),
        ];

        let response = self.get_subsonic("updatePlaylist", &params).await?;
        self.ensure_ok_response(&response)?;
        Ok(())
    }

    // https://github.com/opensubsonic/open-subsonic-api/blob/main/content/en/docs/Endpoints/deleteplaylist.md
    pub async fn delete_item(&self, item_id: &str) -> Result<(), BackendError> {
        debug!("Subsonic::delete_item(item_id={item_id})");
        let params = vec![("id".to_string(), item_id.to_string())];
        let response = self.get_subsonic("deletePlaylist", &params).await?;
        self.ensure_ok_response(&response)?;
        Ok(())
    }

    // https://github.com/opensubsonic/open-subsonic-api/blob/main/content/en/docs/Endpoints/startscan.md
    pub async fn request_library_rescan(&self, _library_id: &str) -> Result<(), BackendError> {
        debug!("Subsonic::request_library_rescan()");
        let response = self.get_subsonic("startScan", &[]).await?;
        self.ensure_ok_response(&response)?;
        Ok(())
    }

    // https://github.com/opensubsonic/open-subsonic-api/blob/main/content/en/docs/Endpoints/getcoverart.md
    pub async fn get_image(
        &self,
        item_id: &str,
        image_type: ImageType,
    ) -> Result<Vec<u8>, BackendError> {
        debug!(
            "Subsonic::get_image(item_id={item_id}, image_type={})",
            image_type.as_str()
        );

        let url = self.rest_url("getCoverArt");
        let mut params = self.auth_params();
        params.retain(|(k, _)| k != "f");
        if !matches!(image_type, ImageType::Backdrop) {
            params.push(("size".to_string(), "200".to_string()));
        }
        params.push(("id".to_string(), item_id.to_string()));

        let response = self.client.get(url).query(&params).send().await?;
        let status = response.status();

        if status.is_success() {
            Ok(response.bytes().await?.to_vec())
        } else if status == StatusCode::UNAUTHORIZED {
            Err(BackendError::AuthenticationFailed {
                message: response.text().await?,
            })
        } else {
            Err(BackendError::Http {
                status,
                message: response.text().await?,
            })
        }
    }

    // https://github.com/opensubsonic/open-subsonic-api/blob/main/content/en/docs/Endpoints/stream.md
    pub fn get_stream_uri(&self, item_id: &str) -> String {
        let max_bitrate = config::get_max_bitrate().unwrap_or(0);
        let format = if max_bitrate > 0 {
            config::get_transcoding_profile().codec.to_string()
        } else {
            "raw".to_string()
        };
        debug!("Subsonic::get_stream_uri(item_id={item_id} bitrate={max_bitrate} format={format})");

        let mut url = self.rest_url("stream");

        let mut params = self.auth_params();
        params.retain(|(k, _)| k != "f");
        params.push(("id".to_string(), item_id.to_string()));
        params.push(("maxBitRate".to_string(), max_bitrate.to_string()));
        params.push(("format".to_string(), format));

        {
            let mut pairs = url.query_pairs_mut();
            for (k, v) in &params {
                pairs.append_pair(k, v);
            }
        }

        url.to_string()
    }

    // https://github.com/opensubsonic/open-subsonic-api/blob/main/content/en/docs/Endpoints/getsong.md
    pub async fn get_playback_info(&self, item_id: &str) -> Result<PlaybackInfo, BackendError> {
        debug!("Subsonic::get_playback_info(item_id={item_id})");

        let response = self
            .get_subsonic("getSong", &[("id".to_string(), item_id.to_string())])
            .await?;
        self.ensure_ok_response(&response)?;

        let song = response.song.ok_or_else(|| BackendError::Http {
            status: StatusCode::BAD_GATEWAY,
            message: "Subsonic getSong response missing song payload".to_string(),
        })?;

        let media_stream = MediaStream {
            type_: Some("Audio".to_string()),
            codec: song.suffix.clone(),
            bit_rate: song.bit_rate,
            sample_rate: song.sampling_rate,
            channels: song.channel_count,
        };

        let media_source = MediaSource {
            media_streams: vec![media_stream],
            container: song.suffix,
            size: song.size,
            supports_direct_play: Some(true),
            supports_direct_stream: Some(true),
            supports_transcoding: Some(true),
        };

        Ok(PlaybackInfo {
            media_sources: vec![media_source],
        })
    }

    // https://github.com/opensubsonic/open-subsonic-api/blob/main/content/en/docs/Endpoints/scrobble.md
    pub async fn playback_report(
        &self,
        report: &PlaybackReport,
        state: &PlaybackReportStatus,
    ) -> Result<(), BackendError> {
        let submission = match state {
            PlaybackReportStatus::Started => "false",
            PlaybackReportStatus::InProgress => return Ok(()),
            PlaybackReportStatus::Stopped => "true",
        };

        debug!(
            "Subsonic::playback_report(item_id={}, submission={submission})",
            report.item_id
        );

        let params = vec![
            ("id".to_string(), report.item_id.clone()),
            ("submission".to_string(), submission.to_string()),
        ];

        let response = self.get_subsonic("scrobble", &params).await?;
        self.ensure_ok_response(&response)?;
        Ok(())
    }

    // https://github.com/opensubsonic/open-subsonic-api/blob/main/content/en/docs/Endpoints/getLyricsBySongId.md
    pub async fn fetch_lyrics(&self, item_id: &str) -> Result<LyricsResponse, BackendError> {
        debug!("Subsonic::fetch_lyrics(item_id={item_id})");

        let response = self
            .get_subsonic(
                "getLyricsBySongId",
                &[("id".to_string(), item_id.to_string())],
            )
            .await?;
        self.ensure_ok_response(&response)?;

        let lyrics = response
            .lyrics_list
            .map(|list| {
                // Use synced lyrics if available
                let entry = list
                    .structured_lyrics
                    .iter()
                    .find(|l| l.synced)
                    .or_else(|| list.structured_lyrics.first());
                entry
                    .map(|l| {
                        l.line
                            .iter()
                            .map(|line| crate::jellyfin::api::Lyric {
                                text: line.value.clone(),
                                // Subsonic timestamps are in ms; Jellyfin uses ticks (100ns).
                                start: line.start.map(|ms| ms * 10_000),
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        Ok(LyricsResponse { lyrics })
    }

    async fn get_subsonic(
        &self,
        endpoint: &str,
        extra_params: &[(String, String)],
    ) -> Result<SubsonicResponse, BackendError> {
        let envelope: SubsonicEnvelope = self.get_json(endpoint, extra_params).await?;
        Ok(envelope.response)
    }

    async fn get_json<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        extra_params: &[(String, String)],
    ) -> Result<T, BackendError> {
        let url = self.rest_url(endpoint);
        let mut params = self.auth_params();
        params.extend_from_slice(extra_params);

        let response = self.client.get(url).query(&params).send().await?;
        let body = self.handle_http_response(response).await?;
        Ok(serde_json::from_str::<T>(&body)?)
    }

    fn ensure_ok_response(&self, response: &SubsonicResponse) -> Result<(), BackendError> {
        if response.is_ok() {
            return Ok(());
        }

        if let Some(error) = &response.error {
            return Err(self.map_api_error(error.code, error.message.clone()));
        }

        Err(BackendError::Http {
            status: StatusCode::BAD_GATEWAY,
            message: "Subsonic API returned non-ok status".to_string(),
        })
    }

    fn map_api_error(&self, code: i32, message: String) -> BackendError {
        match code {
            40 => BackendError::AuthenticationFailed { message },
            _ => BackendError::Http {
                status: StatusCode::BAD_GATEWAY,
                message: format!("Subsonic error {}: {}", code, message),
            },
        }
    }

    fn auth_params(&self) -> Vec<(String, String)> {
        let salt: String = rand::rng()
            .sample_iter(rand::distr::Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();

        let token = format!("{:x}", md5::compute(format!("{}{}", self.password, salt)));

        vec![
            ("u".to_string(), self.username.clone()),
            ("t".to_string(), token),
            ("s".to_string(), salt),
            ("v".to_string(), SUBSONIC_API_VERSION.to_string()),
            ("c".to_string(), SUBSONIC_CLIENT_NAME.to_string()),
            ("f".to_string(), "json".to_string()),
        ]
    }

    fn rest_url(&self, endpoint: &str) -> Url {
        let host = self.host.trim_end_matches('/');
        let endpoint = endpoint.trim_start_matches('/').trim_end_matches(".view");
        Url::parse(&format!("{host}/rest/{endpoint}.view"))
            .expect("Failed to construct Subsonic endpoint URL")
    }

    async fn handle_http_response(&self, response: Response) -> Result<String, BackendError> {
        let status = response.status();
        let body = response.text().await?;
        if status.is_success() {
            Ok(body)
        } else if status == StatusCode::UNAUTHORIZED {
            Err(BackendError::AuthenticationFailed { message: body })
        } else {
            Err(BackendError::Http {
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
