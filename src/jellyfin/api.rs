use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

/// For the endpoints that return a collection of items, we want to skip any
/// items that do not deserialize so that we can still return a usable library.
fn deserialize_items_skip_errors<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    let items = Vec::<Value>::deserialize(deserializer)?;
    let result: Vec<T> = items
        .into_iter()
        .filter_map(|item| match T::deserialize(item) {
            Ok(d_item) => Some(d_item),
            Err(e) => {
                log::warn!("Failed to deserialize jellyfin item, skipping: {}", e);
                None
            }
        })
        .collect();

    Ok(result)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AuthenticateResponse {
    pub access_token: String,
    pub user: User,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct User {
    pub id: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LibraryDto {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LibraryDtoList {
    #[serde(deserialize_with = "deserialize_items_skip_errors")]
    pub items: Vec<LibraryDto>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MusicDtoList {
    // Update the Cache version of this struct in cache.rs if changes are needed
    #[serde(deserialize_with = "deserialize_items_skip_errors")]
    pub items: Vec<MusicDto>,
    pub total_record_count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MusicDto {
    pub name: String,
    pub id: String,
    pub date_created: Option<String>,
    pub run_time_ticks: u64,
    pub album: String,
    pub album_artists: Vec<ArtistItemsDto>,
    pub album_id: String,
    pub normalization_gain: Option<f64>,
    pub production_year: Option<u32>,
    pub index_number: Option<u32>,
    pub parent_index_number: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PlaylistDtoList {
    // Update the Cache version of this struct in cache.rs if changes are needed
    #[serde(deserialize_with = "deserialize_items_skip_errors")]
    pub items: Vec<PlaylistDto>,
    pub total_record_count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PlaylistDto {
    pub name: String,
    pub id: String,
    pub child_count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ArtistItemsDto {
    pub name: String,
    pub id: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PlaylistItems {
    #[serde(deserialize_with = "deserialize_items_skip_errors")]
    pub items: Vec<MusicDto>,
    pub total_record_count: u64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MediaStream {
    #[serde(rename = "Type")]
    pub type_: Option<String>,
    pub codec: Option<String>,
    pub bit_rate: Option<u64>,
    pub sample_rate: Option<u64>,
    pub channels: Option<u32>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MediaSource {
    #[serde(deserialize_with = "deserialize_items_skip_errors")]
    pub media_streams: Vec<MediaStream>,
    pub container: Option<String>,
    pub size: Option<u64>,
    pub supports_direct_stream: Option<bool>,
    pub supports_direct_play: Option<bool>,
    pub supports_transcoding: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PlaybackInfo {
    #[serde(deserialize_with = "deserialize_items_skip_errors")]
    pub media_sources: Vec<MediaSource>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PlaylistUserPermissions {
    pub user_id: String,
    pub can_edit: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct NewPlaylist {
    pub name: String,
    pub ids: Vec<String>,
    pub user_id: String,
    pub media_type: String,
    pub users: Vec<PlaylistUserPermissions>,
    pub is_public: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NewPlaylistResponse {
    pub id: String,
}

pub enum PlaybackReportStatus {
    Started,
    InProgress,
    Stopped,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PlaybackReport {
    pub item_id: String,
    pub session_id: String,
    pub play_session_id: String,
    pub can_seek: bool,
    pub is_paused: bool,
    pub is_muted: bool,
    pub position_ticks: u64,
}
