use crate::{
    jellyfin::{Jellyfin, JellyfinError, api::MusicDto},
    library_utils::{shuffle_songs, songs_for_ids},
};

pub const DEFAULT_SMART_COUNT: u64 = 100;

#[derive(Debug, Clone, PartialEq)]
pub enum PlaylistType {
    Regular,
    ShuffleLibrary { count: u64 },
}

impl PlaylistType {
    pub fn to_id(&self) -> Option<String> {
        match self {
            PlaylistType::Regular => None,
            PlaylistType::ShuffleLibrary { count } => Some(format!("smart:shuffle:{count}")),
        }
    }

    pub fn from_id(id: &str) -> Self {
        if !id.starts_with("smart:") {
            return Self::Regular;
        }

        let parts: Vec<&str> = id.split(':').collect();
        match parts.get(1) {
            Some(&"shuffle") => {
                let count = parts
                    .get(2)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(DEFAULT_SMART_COUNT);
                Self::ShuffleLibrary { count }
            }
            _ => Self::Regular,
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            PlaylistType::Regular => "Playlist".to_string(),
            PlaylistType::ShuffleLibrary { count } => format!("{} Shuffled Songs", count),
        }
    }

    pub async fn load_song_data(
        &self,
        playlist_id: &str,
        jellyfin: &Jellyfin,
        library: &[MusicDto],
    ) -> Result<Vec<MusicDto>, JellyfinError> {
        match self {
            PlaylistType::Regular => {
                let playlist_items = jellyfin.get_playlist_items(playlist_id).await?;
                let songs = songs_for_ids(playlist_items.item_ids, library);
                Ok(songs)
            }
            PlaylistType::ShuffleLibrary { count } => {
                let songs = shuffle_songs(library, *count);
                Ok(songs)
            }
        }
    }

    pub fn estimated_count(&self) -> Option<u64> {
        match self {
            PlaylistType::Regular => None,
            PlaylistType::ShuffleLibrary { count } => Some(*count),
        }
    }

    pub fn icon_name(&self) -> &str {
        match self {
            PlaylistType::ShuffleLibrary { count: _ } => "media-playlist-shuffle-symbolic",
            _ => "audio-x-generic-symbolic",
        }
    }
}
