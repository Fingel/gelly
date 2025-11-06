use serde::{Deserialize, Serialize};

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
    pub items: Vec<LibraryDto>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MusicDtoList {
    pub items: Vec<MusicDto>,
    pub total_record_count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MusicDto {
    pub name: String,
    pub id: String,
    pub container: String,
    pub date_created: String,
    pub run_time_ticks: u64,
    pub user_data: UserDataDto,
    pub album: String,
    pub album_artists: Vec<ArtistItemsDto>,
    pub album_id: String,
    pub media_type: String,
    pub normalization_gain: Option<f64>,
    pub production_year: Option<u32>,
    pub index_number: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PlaylistDtoList {
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
    pub played: bool,
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
