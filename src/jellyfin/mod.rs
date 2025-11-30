use std::fs;
use std::{fmt::Debug, sync::OnceLock};

use api::AuthenticateResponse;
use log::{debug, warn};
use reqwest::{Client, Response, StatusCode};
use serde_json::json;
use thiserror::Error;
use tokio::time::Instant;

use crate::cache::LibraryCache;
use crate::config;
use crate::jellyfin::api::{LibraryDtoList, MusicDtoList, PlaylistDtoList, PlaylistItems};

pub mod api;
pub mod utils;

static CLIENT_ID: &str = "Gelly";

#[derive(Error, Debug)]
pub enum JellyfinError {
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
pub struct Jellyfin {
    client: Client,
    pub host: String,
    pub token: String,
    pub user_id: String,
    pub cache: LibraryCache,
}

impl Jellyfin {
    pub fn new(host: &str, token: &str, user_id: &str) -> Self {
        let client = Client::builder()
            .user_agent(format!("Gelly/{}", config::VERSION))
            .build()
            .expect("Failed to create HTTP client");

        let cache = LibraryCache::new().expect("Could not create cache");

        Self {
            client,
            host: host.to_string(),
            token: token.to_string(),
            user_id: user_id.to_string(),
            cache,
        }
    }

    pub fn is_authenticated(&self) -> bool {
        !self.token.is_empty() && !self.user_id.is_empty() && !self.host.is_empty()
    }

    pub async fn new_authenticate(
        host: &str,
        username: &str,
        password: &str,
    ) -> Result<Self, JellyfinError> {
        let mut jellyfin = Self::new(host, "", "");
        let resp = jellyfin.authenticate(username, password).await?;
        jellyfin.token = resp.access_token;
        jellyfin.user_id = resp.user.id;

        Ok(jellyfin)
    }

    async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> Result<AuthenticateResponse, JellyfinError> {
        let body = json!({
            "Username": username,
            "Pw": password
        });
        let response = self.post_json("Users/authenticatebyname", &body).await?;
        let body = self.handle_response(response).await?;
        Ok(serde_json::from_str(&body)?)
    }

    pub async fn get_views(&self) -> Result<LibraryDtoList, JellyfinError> {
        let response = self.get("UserViews", None).await?;
        let body = self.handle_response(response).await?;
        Ok(serde_json::from_str(&body)?)
    }

    pub async fn get_library(
        &self,
        library_id: &str,
        refresh: bool,
    ) -> Result<MusicDtoList, JellyfinError> {
        if !refresh
            && let Ok(cached_data) = self.cache.load_from_disk("library.json")
            && let Ok(music_list) = serde_json::from_slice::<MusicDtoList>(&cached_data)
        {
            debug!("Loaded library from cache");
            return Ok(music_list);
        }

        // Time library download so we can keep an eye on it.
        let now = Instant::now();
        const LIMIT: u64 = 1000;

        // Make the first request to get total count
        let mut all_items = Vec::new();
        let first_page = self.get_library_page(library_id, 0, LIMIT).await?;
        let total_count = first_page.total_record_count;

        debug!(
            "Total library items: {}, fetched first {} items",
            total_count,
            first_page.items.len()
        );
        all_items.extend(first_page.items);

        // If we have more items to fetch, create concurrent requests for remaining pages
        if total_count > LIMIT {
            let remaining_items = total_count - LIMIT;
            let additional_pages = remaining_items.div_ceil(LIMIT);

            debug!("Fetching {} additional pages", additional_pages);

            // Create futures for all remaining pages using the futures crate
            let page_futures: Vec<_> = (1..=additional_pages)
                .map(|page| {
                    let start_index = page * LIMIT;
                    self.get_library_page(library_id, start_index, LIMIT)
                })
                .collect();
            let page_results = futures::future::join_all(page_futures).await;

            for result in page_results {
                match result {
                    Ok(page) => {
                        debug!("Fetched page with {} items", page.items.len());
                        all_items.extend(page.items);
                    }
                    Err(e) => {
                        warn!("Failed to fetch library page: {}", e);
                        // Continue with other pages rather than failing completely
                    }
                }
            }
        }

        let elapsed = now.elapsed();
        debug!(
            "Total time taken to fetch library: {:?} Limit: {}",
            elapsed, LIMIT
        );

        debug!("Total items collected: {}", all_items.len());

        // Create the final result with all items
        let final_result = MusicDtoList {
            items: all_items,
            total_record_count: total_count,
        };

        if let Ok(json_data) = serde_json::to_string(&final_result)
            && let Err(e) = self
                .cache
                .save_to_disk("library.json", json_data.as_bytes())
        {
            warn!("Failed to save library to cache: {}", e);
        }

        Ok(final_result)
    }

    async fn get_library_page(
        &self,
        library_id: &str,
        start_index: u64,
        limit: u64,
    ) -> Result<MusicDtoList, JellyfinError> {
        let start_index = start_index.to_string();
        let limit = limit.to_string();
        let params = vec![
            ("parentId", library_id),
            ("IncludeItemTypes", "Audio"),
            ("sortBy", "DateCreated"),
            ("sortOrder", "Descending"),
            ("recursive", "true"),
            ("fields", "DateCreated"),
            ("ImageTypeLimit", "1"),
            ("EnableImageTypes", "Primary"),
            ("StartIndex", &start_index),
            ("Limit", &limit),
        ];

        let response = self.get("Items", Some(&params)).await?;
        let body = self.handle_response(response).await?;
        Ok(serde_json::from_str(&body)?)
    }

    pub async fn get_playlists(&self, refresh: bool) -> Result<PlaylistDtoList, JellyfinError> {
        if !refresh
            && let Ok(cached_data) = self.cache.load_from_disk("playlists.json")
            && let Ok(playlist_list) = serde_json::from_slice::<PlaylistDtoList>(&cached_data)
        {
            debug!("Loaded playlists from cache");
            return Ok(playlist_list);
        } else {
            warn!("Could not load playlists from cache");
        }

        let params = vec![
            ("IncludeItemTypes", "Playlist"),
            ("sortBy", "DateCreated"),
            ("sortOrder", "Descending"),
            ("recursive", "true"),
            ("fields", "DateCreated"),
            ("ImageTypeLimit", "1"),
            ("EnableImageTypes", "Primary"),
            ("StartIndex", "0"),
        ];
        let response = self.get("Items", Some(&params)).await?;
        let body = self.handle_response(response).await?;
        let final_result = serde_json::from_str(&body)?;

        if let Ok(json_data) = serde_json::to_string(&final_result)
            && let Err(e) = self
                .cache
                .save_to_disk("playlists.json", json_data.as_bytes())
        {
            warn!("Failed to save playlists to cache: {}", e);
        }

        Ok(final_result)
    }

    pub async fn get_playlist_items(
        &self,
        playlist_id: &str,
    ) -> Result<PlaylistItems, JellyfinError> {
        let path = format!("Playlists/{}", playlist_id);
        let response = self.get(&path, None).await?;
        let body = self.handle_response(response).await?;
        Ok(serde_json::from_str(&body)?)
    }

    pub async fn add_playlist_item(
        &self,
        playlist_id: &str,
        item_id: &str,
    ) -> Result<(), JellyfinError> {
        let path = format!("Playlists/{}/Items", playlist_id);
        let params = vec![("ids", item_id), ("userId", &self.user_id)];
        let response = self.post(&path, Some(&params), None).await?;
        self.handle_response(response).await?;
        Ok(())
    }

    pub async fn move_playlist_item(
        &self,
        playlist_id: &str,
        item_id: &str,
        new_index: i32,
    ) -> Result<(), JellyfinError> {
        let path = format!(
            "Playlists/{}/Items/{}/Move/{}",
            playlist_id, item_id, new_index
        );
        let response = self.post(&path, None, None).await?;
        self.handle_response(response).await?;
        Ok(())
    }

    pub async fn remove_playlist_item(
        &self,
        playlist_id: &str,
        item_id: &str,
    ) -> Result<(), JellyfinError> {
        let path = format!("Playlists/{}/Items/", playlist_id);
        let params = vec![("entryIds", item_id)];
        self.delete(&path, Some(&params)).await?;
        Ok(())
    }

    pub fn clear_cache(&self) {
        if let Err(e) = self.cache.clear() {
            warn!("Failed to clear cache: {}", e);
        }
    }

    pub async fn request_library_rescan(&self, library_id: &str) -> Result<(), JellyfinError> {
        let params = vec![
            ("itemId", library_id),
            ("Recursive", "true"),
            ("ImageRefreshMode", "Default"),
            ("MetadataRefreshMode", "Default"),
            ("ReplaceAllImages", "false"),
            ("RegenerateTrickplay", "false"),
            ("ReplaceAllMetadata", "false"),
        ];
        let response = self
            .post(
                &format!("Items/{}/Refresh", library_id),
                Some(&params),
                None,
            )
            .await?;
        self.handle_response(response).await?;
        Ok(())
    }

    pub async fn get_image(&self, item_id: &str) -> Result<Vec<u8>, JellyfinError> {
        let params = vec![
            ("fillHeight", "200"),
            ("fillWidth", "200"),
            ("quality", "96"),
        ];
        let response = self
            .get(&format!("Items/{}/Images/Primary", item_id), Some(&params))
            .await?;
        self.handle_binary_response(response).await
    }

    pub fn get_stream_uri(&self, item_id: &str) -> String {
        // Prioritize FLAC for lossless audio, with fallbacks for compatibility
        let containers = "flac,opus,mp3,aac,m4a,ogg,wav,webm|opus,webm|webma,webma";
        format!(
            "{}/Audio/{item_id}/universal?api_key={}&userId={}&container={}",
            self.host.trim_end_matches("/"),
            self.token,
            self.user_id,
            containers
        )
    }

    fn get_hostname(&self) -> &'static str {
        static HOSTNAME: OnceLock<String> = OnceLock::new();
        HOSTNAME.get_or_init(|| {
            fs::read_to_string("/proc/sys/kernel/hostname")
                .unwrap_or_else(|_| "Gelly-Device".to_string())
                .trim()
                .to_string()
        })
    }

    fn auth_header(&self) -> String {
        let device = self.get_hostname();
        let auth = if !self.token.is_empty() {
            format!(", Token=\"{}\"", self.token)
        } else {
            "".to_string()
        };
        let uuid = config::application_uuid();

        format!(
            "MediaBrowser Client=\"{}\", Device=\"{}\", DeviceId=\"{}\", Version=\"{}\"{}",
            CLIENT_ID,
            device,
            uuid,
            config::VERSION,
            auth
        )
    }

    async fn post_json<T>(&self, endpoint: &str, body: &T) -> Result<Response, JellyfinError>
    where
        T: serde::Serialize,
    {
        let url = format!(
            "{}/{}",
            self.host.trim_end_matches('/'),
            endpoint.trim_start_matches('/')
        );
        debug!("Sending POST request to {}", url);
        let request = self
            .client
            .post(&url)
            .json(&body)
            .header("Authorization", self.auth_header());
        let response = request.send().await?;
        Ok(response)
    }

    async fn post(
        &self,
        endpoint: &str,
        params: Option<&[(&str, &str)]>,
        body: Option<String>,
    ) -> Result<Response, JellyfinError> {
        let url = self.format_url(endpoint);
        debug!("Sending POST request to {}", url);
        let request = self
            .client
            .post(&url)
            .query(params.unwrap_or(&[]))
            .body(body.unwrap_or_default())
            .header("Authorization", self.auth_header());
        let response = request.send().await?;
        Ok(response)
    }

    /// Any GET request
    async fn get(
        &self,
        endpoint: &str,
        params: Option<&[(&str, &str)]>,
    ) -> Result<Response, JellyfinError> {
        let url = self.format_url(endpoint);
        debug!("Sending GET request to {}", url);
        let request = self
            .client
            .get(&url)
            .query(params.unwrap_or(&[]))
            .header("Authorization", self.auth_header());
        let response = request.send().await?;
        Ok(response)
    }

    async fn delete(
        &self,
        endpoint: &str,
        params: Option<&[(&str, &str)]>,
    ) -> Result<Response, JellyfinError> {
        let url = self.format_url(endpoint);
        debug!("Sending DELETE request to {}", url);
        let request = self
            .client
            .delete(&url)
            .query(params.unwrap_or(&[]))
            .header("Authorization", self.auth_header());
        let response = request.send().await?;
        Ok(response)
    }

    fn format_url(&self, endpoint: &str) -> String {
        format!(
            "{}/{}",
            self.host.trim_end_matches('/'),
            endpoint.trim_start_matches('/')
        )
    }

    /// Responsible for error handling when reading responses from Jellyfin
    async fn handle_response(&self, response: Response) -> Result<String, JellyfinError> {
        let status = response.status();
        if status.is_success() {
            let response_body = response.text().await?;
            Ok(response_body)
        } else {
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown Error".to_string());

            match status {
                StatusCode::UNAUTHORIZED => Err(JellyfinError::AuthenticationFailed { message }),
                _ => Err(JellyfinError::Http { status, message }),
            }
        }
    }

    /// Same as handle_response but does not deserialize the response body.
    async fn handle_binary_response(&self, response: Response) -> Result<Vec<u8>, JellyfinError> {
        let status = response.status();
        if status.is_success() {
            let response_body = response.bytes().await?;
            Ok(response_body.to_vec())
        } else {
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown Error".to_string());

            match status {
                StatusCode::UNAUTHORIZED => Err(JellyfinError::AuthenticationFailed { message }),
                _ => Err(JellyfinError::Http { status, message }),
            }
        }
    }
}

impl Default for Jellyfin {
    fn default() -> Self {
        Self::new("", "", "")
    }
}
