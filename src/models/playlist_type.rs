use crate::{
    jellyfin::{Jellyfin, JellyfinError, api::MusicDto},
    library_utils::{most_played_songs, shuffle_songs},
};

pub const DEFAULT_SMART_COUNT: u64 = 100;

#[derive(Debug, Clone, PartialEq)]
pub enum PlaylistType {
    Regular {
        id: String,
        name: String,
        child_count: u64,
    },
    ShuffleLibrary {
        count: u64,
    },
    MostPlayed {
        count: u64,
    },
}

impl PlaylistType {
    pub fn new_regular(id: String, name: String, child_count: u64) -> Self {
        if id.is_empty() {
            log::warn!("Creating regular playlist with empty ID");
        }
        Self::Regular {
            id,
            name,
            child_count,
        }
    }
    pub fn to_id(&self) -> String {
        match self {
            PlaylistType::Regular { id, .. } => id.clone(),
            PlaylistType::ShuffleLibrary { count } => format!("smart:shuffle:{count}"),
            PlaylistType::MostPlayed { count } => format!("smart:most-played:{count}"),
        }
    }

    pub fn smart_from_id(id: &str) -> Option<Self> {
        if !id.starts_with("smart:") {
            return None;
        }
        let parts: Vec<&str> = id.split(':').collect();
        match parts.get(1) {
            Some(&"shuffle") => {
                let count = parts
                    .get(2)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(DEFAULT_SMART_COUNT);
                Some(Self::ShuffleLibrary { count })
            }
            Some(&"most-played") => {
                let count = parts
                    .get(2)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(DEFAULT_SMART_COUNT);
                Some(Self::MostPlayed { count })
            }
            _ => None,
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            PlaylistType::Regular { name, .. } => name.clone(),
            PlaylistType::ShuffleLibrary { count } => format!("{} Shuffled Songs", count),
            PlaylistType::MostPlayed { count } => format!("Top {} Played Songs", count),
        }
    }

    pub async fn load_song_data(
        &self,
        jellyfin: &Jellyfin,
        library: &[MusicDto],
    ) -> Result<Vec<MusicDto>, JellyfinError> {
        match self {
            PlaylistType::Regular { id, .. } => {
                let playlist_items = jellyfin.get_playlist_items(id).await?;
                Ok(playlist_items.items)
            }
            PlaylistType::ShuffleLibrary { count } => {
                let songs = shuffle_songs(library, *count);
                Ok(songs)
            }
            PlaylistType::MostPlayed { count } => {
                let songs = most_played_songs(library, *count);
                Ok(songs)
            }
        }
    }

    pub fn estimated_count(&self) -> u64 {
        match self {
            PlaylistType::Regular { child_count, .. } => *child_count,
            PlaylistType::ShuffleLibrary { count } => *count,
            PlaylistType::MostPlayed { count } => *count,
        }
    }

    pub fn icon_name(&self) -> &str {
        match self {
            PlaylistType::ShuffleLibrary { count: _ } => "media-playlist-shuffle-symbolic",
            PlaylistType::MostPlayed { count: _ } => "starred-symbolic",
            _ => "audio-x-generic-symbolic",
        }
    }

    pub fn is_smart(&self) -> bool {
        !matches!(self, PlaylistType::Regular { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_regular(id: &str, name: &str, count: u64) -> PlaylistType {
        PlaylistType::new_regular(id.to_string(), name.to_string(), count)
    }

    #[test]
    fn test_new_regular() {
        let playlist = create_test_regular("jellyfin-123", "My Playlist", 42);
        match playlist {
            PlaylistType::Regular {
                id,
                name,
                child_count,
            } => {
                assert_eq!(id, "jellyfin-123");
                assert_eq!(name, "My Playlist");
                assert_eq!(child_count, 42);
            }
            _ => panic!("Expected Regular playlist"),
        }
    }

    #[test]
    fn test_new_regular_empty_id_logs_warning() {
        // TODO: figure out how to test logs here
        let playlist = create_test_regular("", "Empty ID Test", 10);
        assert_eq!(playlist.to_id(), "");
        assert_eq!(playlist.display_name(), "Empty ID Test");
    }

    #[test]
    fn test_to_id_regular_playlist() {
        let playlist = create_test_regular("jellyfin-456", "Test Playlist", 25);
        assert_eq!(playlist.to_id(), "jellyfin-456");
    }

    #[test]
    fn test_to_id_shuffle_library() {
        let playlist = PlaylistType::ShuffleLibrary { count: 50 };
        assert_eq!(playlist.to_id(), "smart:shuffle:50");
    }

    #[test]
    fn test_to_id_most_played() {
        let playlist = PlaylistType::MostPlayed { count: 50 };
        assert_eq!(playlist.to_id(), "smart:most-played:50");
    }

    #[test]
    fn test_to_id_shuffle_library_zero() {
        // TODO: Allow this?
        let playlist = PlaylistType::ShuffleLibrary { count: 0 };
        assert_eq!(playlist.to_id(), "smart:shuffle:0");
    }

    #[test]
    fn test_smart_from_id_shuffle_valid() {
        let playlist = PlaylistType::smart_from_id("smart:shuffle:50");
        assert_eq!(playlist, Some(PlaylistType::ShuffleLibrary { count: 50 }));
    }

    #[test]
    fn test_smart_from_id_shuffle_zero() {
        // TODO: Allow this?
        let playlist = PlaylistType::smart_from_id("smart:shuffle:0");
        assert_eq!(playlist, Some(PlaylistType::ShuffleLibrary { count: 0 }));
    }

    #[test]
    fn test_smart_from_id_shuffle_missing_count() {
        let playlist = PlaylistType::smart_from_id("smart:shuffle:");
        assert_eq!(
            playlist,
            Some(PlaylistType::ShuffleLibrary {
                count: DEFAULT_SMART_COUNT
            })
        );
    }

    #[test]
    fn test_smart_from_id_shuffle_no_count() {
        let playlist = PlaylistType::smart_from_id("smart:shuffle");
        assert_eq!(
            playlist,
            Some(PlaylistType::ShuffleLibrary {
                count: DEFAULT_SMART_COUNT
            })
        );
    }

    #[test]
    fn test_smart_from_id_shuffle_invalid_count() {
        let playlist = PlaylistType::smart_from_id("smart:shuffle:not-a-number");
        assert_eq!(
            playlist,
            Some(PlaylistType::ShuffleLibrary {
                count: DEFAULT_SMART_COUNT
            })
        );
    }

    #[test]
    fn test_smart_from_id_shuffle_negative_count() {
        let playlist = PlaylistType::smart_from_id("smart:shuffle:-50");
        assert_eq!(
            playlist,
            Some(PlaylistType::ShuffleLibrary {
                count: DEFAULT_SMART_COUNT
            })
        );
    }

    #[test]
    fn test_smart_from_id_shuffle_float_count() {
        let playlist = PlaylistType::smart_from_id("smart:shuffle:50.5");
        assert_eq!(
            playlist,
            Some(PlaylistType::ShuffleLibrary {
                count: DEFAULT_SMART_COUNT
            })
        );
    }

    #[test]
    fn test_smart_from_id_most_played_songs() {
        let playlist = PlaylistType::smart_from_id("smart:most-played:5");
        assert_eq!(playlist, Some(PlaylistType::MostPlayed { count: 5 }));
    }

    #[test]
    fn test_smart_from_id_regular_playlist_returns_none() {
        let playlist = PlaylistType::smart_from_id("regular-playlist-id-123");
        assert_eq!(playlist, None);
    }

    #[test]
    fn test_smart_from_id_empty_string_returns_none() {
        let playlist = PlaylistType::smart_from_id("");
        assert_eq!(playlist, None);
    }

    #[test]
    fn test_smart_from_id_unknown_smart_type() {
        let playlist = PlaylistType::smart_from_id("smart:unknown:50");
        assert_eq!(playlist, None);
    }

    #[test]
    fn test_smart_from_id_smart_prefix_only() {
        let playlist = PlaylistType::smart_from_id("smart:");
        assert_eq!(playlist, None);
    }

    #[test]
    fn test_smart_from_id_smart_prefix_empty_parts() {
        let playlist = PlaylistType::smart_from_id("smart:::");
        assert_eq!(playlist, None);
    }

    #[test]
    fn test_smart_from_id_case_sensitive() {
        let playlist = PlaylistType::smart_from_id("SMART:shuffle:50");
        assert_eq!(playlist, None);

        let playlist = PlaylistType::smart_from_id("smart:SHUFFLE:50");
        assert_eq!(playlist, None);
    }

    #[test]
    fn test_smart_from_id_extra_colons() {
        let playlist = PlaylistType::smart_from_id("smart:shuffle:50:extra:parts");
        assert_eq!(playlist, Some(PlaylistType::ShuffleLibrary { count: 50 }));
    }

    #[test]
    fn test_smart_from_id_whitespace_in_count() {
        let playlist = PlaylistType::smart_from_id("smart:shuffle: 50 ");
        assert_eq!(
            playlist,
            Some(PlaylistType::ShuffleLibrary {
                count: DEFAULT_SMART_COUNT
            })
        );
    }

    #[test]
    fn test_smart_from_id_max_u64() {
        let max_u64_str = u64::MAX.to_string();
        let playlist = PlaylistType::smart_from_id(&format!("smart:shuffle:{}", max_u64_str));
        assert_eq!(
            playlist,
            Some(PlaylistType::ShuffleLibrary { count: u64::MAX })
        );
    }

    #[test]
    fn test_smart_from_id_overflow_u64() {
        let playlist = PlaylistType::smart_from_id("smart:shuffle:999999999999999999999999999999");
        assert_eq!(
            playlist,
            Some(PlaylistType::ShuffleLibrary {
                count: DEFAULT_SMART_COUNT
            })
        );
    }

    #[test]
    fn test_smart_from_id_leading_zeros() {
        let playlist = PlaylistType::smart_from_id("smart:shuffle:00050");
        assert_eq!(playlist, Some(PlaylistType::ShuffleLibrary { count: 50 }));
    }

    #[test]
    fn test_smart_from_id_unicode_digits() {
        let playlist = PlaylistType::smart_from_id("smart:shuffle:५०");
        assert_eq!(
            playlist,
            Some(PlaylistType::ShuffleLibrary {
                count: DEFAULT_SMART_COUNT
            })
        );
    }

    #[test]
    fn test_display_name_regular() {
        let playlist = create_test_regular("id", "My Dope AF Playlist", 15);
        assert_eq!(playlist.display_name(), "My Dope AF Playlist");
    }

    #[test]
    fn test_display_name_regular_empty_name() {
        let playlist = create_test_regular("id", "", 10);
        assert_eq!(playlist.display_name(), "");
    }

    #[test]
    fn test_display_name_shuffle_library() {
        let playlist = PlaylistType::ShuffleLibrary { count: 50 };
        assert_eq!(playlist.display_name(), "50 Shuffled Songs");
    }

    #[test]
    fn test_display_name_shuffle_library_singular() {
        let playlist = PlaylistType::ShuffleLibrary { count: 1 };
        assert_eq!(playlist.display_name(), "1 Shuffled Songs");
        // Note: Current implementation doesn't handle singular/plural
    }

    #[test]
    fn test_display_name_shuffle_library_zero() {
        let playlist = PlaylistType::ShuffleLibrary { count: 0 };
        assert_eq!(playlist.display_name(), "0 Shuffled Songs");
    }

    #[test]
    fn test_estimated_count_regular() {
        let playlist = create_test_regular("id", "Test", 42);
        assert_eq!(playlist.estimated_count(), 42);
    }

    #[test]
    fn test_estimated_count_regular_zero() {
        let playlist = create_test_regular("id", "Empty", 0);
        assert_eq!(playlist.estimated_count(), 0);
    }

    #[test]
    fn test_estimated_count_shuffle_library() {
        let playlist = PlaylistType::ShuffleLibrary { count: 75 };
        assert_eq!(playlist.estimated_count(), 75);
    }

    #[test]
    fn test_estimated_count_shuffle_library_zero() {
        let playlist = PlaylistType::ShuffleLibrary { count: 0 };
        assert_eq!(playlist.estimated_count(), 0);
    }

    #[test]
    fn test_is_smart_regular() {
        let playlist = create_test_regular("id", "Regular Playlist", 20);
        assert!(!playlist.is_smart());
    }

    #[test]
    fn test_is_smart_shuffle_library() {
        let playlist = PlaylistType::ShuffleLibrary { count: 50 };
        assert!(playlist.is_smart());
    }

    #[test]
    fn test_roundtrip_shuffle_library() {
        let original = PlaylistType::ShuffleLibrary { count: 75 };
        let id = original.to_id();
        let parsed = PlaylistType::smart_from_id(&id).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_roundtrip_most_played_library() {
        let original = PlaylistType::MostPlayed { count: 75 };
        let id = original.to_id();
        let parsed = PlaylistType::smart_from_id(&id).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_very_long_names_and_ids() {
        let long_id = "a".repeat(1000);
        let long_name = "b".repeat(1000);
        let playlist = create_test_regular(&long_id, &long_name, u64::MAX);

        assert_eq!(playlist.to_id().len(), 1000);
        assert_eq!(playlist.display_name().len(), 1000);
        assert_eq!(playlist.estimated_count(), u64::MAX);
    }

    #[test]
    fn test_special_characters_in_names_and_ids() {
        let playlist = create_test_regular(
            "id-with-special-chars-!@#$%^&*()",
            "Playlist with émojis 🎵 and unicode ñ",
            123,
        );

        assert_eq!(playlist.to_id(), "id-with-special-chars-!@#$%^&*()");
        assert_eq!(
            playlist.display_name(),
            "Playlist with émojis 🎵 and unicode ñ"
        );
    }
}
