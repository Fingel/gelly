use std::fs;
use std::{fmt::Debug, sync::OnceLock};

use api::AuthenticateResponse;
use log::debug;
use reqwest::{Client, Response, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::json;
use thiserror::Error;

use crate::jellyfin::api::LibraryDtoList;

pub mod api;

static CLIENT_ID: &str = "Gelly"; //TODO: get this from the gtk app config
static VERSION: &str = "0.1"; //TODO: get this from build script?
static UUID: &str = "9770ae10-835f-422b-8125-81b8977b181d"; //TODO: generate and store in settings

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
}

impl Jellyfin {
    pub fn new(host: &str, token: &str, user_id: &str) -> Self {
        let client = Client::builder()
            .user_agent("Gelly/0.1")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            host: host.to_string(),
            token: token.to_string(),
            user_id: user_id.to_string(),
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
        self.handle_response(response).await
    }

    pub async fn get_views(&self) -> Result<LibraryDtoList, JellyfinError> {
        let response = self.get("UserViews", None).await?;
        self.handle_response(response).await
    }

    pub async fn get_albums(&self, library_id: &str) -> Result<LibraryDtoList, JellyfinError> {
        let params = vec![
            ("parentId", library_id),
            ("sortBy", "DateCreated"),
            ("sortOrder", "Ascending"),
            ("IncludeItemTypes", "MusicAlbum"),
            ("Recursive", "true"),
            ("Fields", "PrimaryImageAspectRatio,SortName"),
            ("ImageTypeLimit", "1"),
            ("EnableImageTypes", "Primary,Backdrop,Banner,Thumb"),
            ("StartIndex", "0"),
            ("Limit", "100"),
        ];
        self.get_items(params).await
    }

    async fn get_items(&self, params: Vec<(&str, &str)>) -> Result<LibraryDtoList, JellyfinError> {
        let response = self.get("Items", Some(&params)).await?;
        self.handle_response(response).await
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

        format!(
            "MediaBrowser Client=\"{}\", Device=\"{}\", DeviceId=\"{}\", Version=\"{}\"{}",
            CLIENT_ID, device, UUID, VERSION, auth
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

    /// Any GET request
    async fn get(
        &self,
        endpoint: &str,
        params: Option<&[(&str, &str)]>,
    ) -> Result<Response, JellyfinError> {
        let url = format!(
            "{}/{}",
            self.host.trim_end_matches('/'),
            endpoint.trim_start_matches('/')
        );
        debug!("Sending GET request to {}", url);
        let request = self
            .client
            .get(&url)
            .query(params.unwrap_or(&[]))
            .header("Authorization", self.auth_header());
        let response = request.send().await?;
        Ok(response)
    }

    /// Responsible for error handling when reading responses from Jellyfin
    async fn handle_response<T>(&self, response: Response) -> Result<T, JellyfinError>
    where
        T: DeserializeOwned + Debug,
    {
        let status = response.status();
        if status.is_success() {
            let json_response = response.json::<T>().await?;
            debug!("Received response: {:?}", json_response);
            Ok(json_response)
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
