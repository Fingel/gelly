use serde::Deserialize;

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

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MusicDtoList {
    pub items: Vec<MusicDto>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MusicDto {
    pub name: String,
    pub id: String,
    pub container: String,
    pub date_created: String,
    pub run_time_ticks: u64,
    pub user_data: UserDataDto,
    pub album: String,
    pub artist_items: Vec<ArtistItemsDto>,
    pub album_id: String,
    pub album_primary_image_tag: String,
    pub media_type: String,
    pub normalization_gain: Option<f64>,
    pub production_year: Option<u32>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UserDataDto {
    pub play_count: u64,
    pub is_favorite: bool,
    pub played: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ArtistItemsDto {
    pub name: String,
    pub id: String,
}
