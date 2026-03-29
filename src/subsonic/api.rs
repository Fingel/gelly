#![allow(dead_code)]

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
    let result = items
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

#[derive(Debug, Clone, Deserialize)]
pub struct SubsonicEnvelope {
    #[serde(rename = "subsonic-response")]
    pub response: SubsonicResponse,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubsonicResponse {
    pub status: String,
    pub version: Option<String>,
    pub error: Option<SubsonicError>,
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
