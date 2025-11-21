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
    #[serde(deserialize_with = "deserialize_items_skip_errors")]
    pub items: Vec<MusicDto>,
    pub total_record_count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MusicDto {
    pub name: String,
    pub id: String,
    pub date_created: String,
    pub run_time_ticks: u64,
    pub user_data: UserDataDto,
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
pub struct UserDataDto {
    pub play_count: u64,
    pub is_favorite: bool,
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
    pub item_ids: Vec<String>,
}
