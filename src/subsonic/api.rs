use serde::{Deserialize, Deserializer};
use serde_json::Value;

/// For collection-like payloads, skip malformed items so we can still use
/// partial server responses.
fn deserialize_items_skip_errors<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    let items = Vec::<Value>::deserialize(deserializer)?;
    let result: Vec<T> = items
        .into_iter()
        .filter_map(|item| match T::deserialize(item) {
            Ok(value) => Some(value),
            Err(err) => {
                log::warn!("Failed to deserialize Subsonic item, skipping: {}", err);
                None
            }
        })
        .collect();
    Ok(result)
}

fn deserialize_id_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::String(s) => Ok(s),
        Value::Number(n) => Ok(n.to_string()),
        _ => Err(serde::de::Error::custom(
            "expected music folder id as string or integer",
        )),
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubsonicEnvelope {
    #[serde(rename = "subsonic-response")]
    pub response: SubsonicResponse,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubsonicResponse {
    pub status: String,
    pub error: Option<SubsonicError>,
    pub music_folders: Option<MusicFoldersPayload>,

    // Needed by get_library flow:
    // - getAlbumList2 -> album_list2
    // - getAlbum      -> album
    pub album_list2: Option<AlbumList2Payload>,
    pub album: Option<Album>,
}

impl SubsonicResponse {
    pub fn is_ok(&self) -> bool {
        self.status.eq_ignore_ascii_case("ok")
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubsonicError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MusicFoldersPayload {
    #[serde(
        default,
        rename = "musicFolder",
        deserialize_with = "deserialize_items_skip_errors"
    )]
    pub music_folders: Vec<MusicFolder>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicFolder {
    #[serde(deserialize_with = "deserialize_id_string")]
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AlbumList2Payload {
    #[serde(default, deserialize_with = "deserialize_items_skip_errors")]
    pub album: Vec<AlbumListEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumListEntry {
    #[serde(deserialize_with = "deserialize_id_string")]
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    pub id: String,
    pub name: String,
    pub artist: Option<String>,
    pub artist_id: Option<String>,
    pub created: Option<String>,
    pub year: Option<u32>,

    #[serde(default, deserialize_with = "deserialize_items_skip_errors")]
    pub song: Vec<Song>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Song {
    pub id: String,
    pub title: String,
    pub album: Option<String>,
    pub album_id: Option<String>,
    pub artist: Option<String>,
    pub artist_id: Option<String>,
    pub album_artist: Option<String>,
    pub duration: Option<u64>,
    pub track: Option<u32>,
    pub disc_number: Option<u32>,
    pub year: Option<u32>,
    pub created: Option<String>,
    pub play_count: Option<u64>,
}