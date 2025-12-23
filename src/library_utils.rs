use crate::application::Application;
use crate::jellyfin::JellyfinError;
use crate::jellyfin::api::MusicDto;
use crate::models::{AlbumModel, ArtistModel, PlaylistModel, SongModel};
use rand::prelude::*;
use std::collections::{HashMap, HashSet};

pub fn albums_from_library(library: &[MusicDto]) -> Vec<AlbumModel> {
    // Collect playcounts in a separate loop to avoid too many getter/setters
    // on the model gobject
    let mut play_count_map = HashMap::<String, u64>::new();
    for dto in library {
        *play_count_map.entry(dto.album_id.clone()).or_insert(0) += dto.user_data.play_count;
    }

    let mut seen_album_ids = HashSet::new();
    library
        .iter()
        .filter(|dto| seen_album_ids.insert(&dto.album_id))
        .map(|dto| {
            let album = AlbumModel::from(dto);
            if let Some(&total_play_count) = play_count_map.get(&dto.album_id) {
                album.set_play_count(total_play_count);
            }
            album
        })
        .collect()
}

pub fn artists_from_library(library: &[MusicDto]) -> Vec<ArtistModel> {
    let mut play_count_map = HashMap::<String, u64>::new();
    for dto in library {
        for artist in &dto.album_artists {
            *play_count_map.entry(artist.id.clone()).or_insert(0) += dto.user_data.play_count;
        }
    }
    let mut seen_artist_ids = HashSet::new();
    let mut artists: Vec<ArtistModel> = library
        .iter()
        .flat_map(|dto| &dto.album_artists)
        .filter(|artist| seen_artist_ids.insert(&artist.id))
        .map(|dto| {
            let artist = ArtistModel::from(dto);
            if let Some(&total_play_count) = play_count_map.get(&dto.id) {
                artist.set_play_count(total_play_count);
            }
            artist
        })
        .collect();
    artists.sort_by_key(|artist| artist.name().to_lowercase());

    artists
}

pub fn albums_for_artist(artist_id: &str, library: &[MusicDto]) -> Vec<AlbumModel> {
    let mut seen_album_ids = HashSet::new();
    let albums: Vec<AlbumModel> = library
        .iter()
        .filter(|dto| {
            dto.album_artists
                .iter()
                .any(|artist| artist.id == artist_id)
        })
        .filter(|dto| seen_album_ids.insert(&dto.album_id))
        .map(AlbumModel::from)
        .collect();

    albums
}

pub fn songs_for_album(album_id: &str, library: &[MusicDto]) -> Vec<SongModel> {
    let mut tracks: Vec<SongModel> = library
        .iter()
        .filter(|dto| dto.album_id == album_id)
        .map(SongModel::from)
        .collect();
    tracks.sort_by_key(|t| (t.parent_track_number(), t.track_number()));
    tracks
}

pub fn shuffle_songs(library: &[MusicDto], num: u64) -> Vec<MusicDto> {
    let mut rng = rand::rng();
    let chosen = library.choose_multiple(&mut rng, num as usize);
    chosen.into_iter().cloned().collect()
}

pub fn most_played_songs(library: &[MusicDto], num: u64) -> Vec<MusicDto> {
    let mut songs: Vec<MusicDto> = library
        .iter()
        .filter(|dto| dto.user_data.play_count > 0)
        .cloned()
        .collect();
    songs.sort_by_key(|dto| std::cmp::Reverse(dto.user_data.play_count));
    songs.into_iter().take(num as usize).collect()
}

pub fn songs_for_playlist(
    playlist_model: &PlaylistModel,
    app: &Application,
    cb: impl Fn(Result<Vec<MusicDto>, JellyfinError>) + 'static,
) {
    let library_data = app.library().borrow().clone();
    let jellyfin = app.jellyfin();
    let playlist_type = playlist_model.playlist_type();
    app.http_with_loading(
        async move { playlist_type.load_song_data(&jellyfin, &library_data).await },
        cb,
    );
}

pub fn play_album(id: &str, app: &Application) {
    let library = app.library().clone();
    let songs = songs_for_album(id, &library.borrow());
    if let Some(audio_model) = app.audio_model() {
        audio_model.set_queue(songs, 0);
    } else {
        log::warn!("No audio model found");
    }
}

pub fn play_artist(id: &str, app: &Application) {
    let library = app.library().clone();
    let albums = albums_for_artist(id, &library.borrow());
    let songs: Vec<SongModel> = albums
        .iter()
        .flat_map(|album| songs_for_album(&album.id(), &library.borrow()))
        .collect();
    if let Some(audio_model) = app.audio_model() {
        audio_model.set_queue(songs, 0);
    } else {
        log::warn!("No audio model found");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jellyfin::api::{ArtistItemsDto, MusicDto, UserDataDto};

    #[allow(clippy::too_many_arguments)]
    fn create_test_music_dto(
        id: &str,
        name: &str,
        album: &str,
        album_id: &str,
        artist_name: &str,
        artist_id: &str,
        index_number: Option<u32>,
        parent_index_number: Option<u32>,
    ) -> MusicDto {
        MusicDto {
            id: id.to_string(),
            name: name.to_string(),
            album: album.to_string(),
            album_id: album_id.to_string(),
            album_artists: vec![ArtistItemsDto {
                name: artist_name.to_string(),
                id: artist_id.to_string(),
            }],
            date_created: Some("2025-01-01".to_string()),
            run_time_ticks: 2000000000, // ~3 minutes
            normalization_gain: None,
            production_year: Some(2023),
            index_number,
            parent_index_number,
            user_data: UserDataDto {
                play_count: 1,
                is_favorite: false,
                played: true,
            },
        }
    }

    fn create_test_music_dto_multi_artists(
        id: &str,
        name: &str,
        album: &str,
        album_id: &str,
        artists: Vec<(&str, &str)>, // (name, id) pairs
    ) -> MusicDto {
        let album_artists = artists
            .into_iter()
            .map(|(name, id)| ArtistItemsDto {
                name: name.to_string(),
                id: id.to_string(),
            })
            .collect();

        MusicDto {
            id: id.to_string(),
            name: name.to_string(),
            album: album.to_string(),
            album_id: album_id.to_string(),
            album_artists,
            date_created: Some("2025-01-01".to_string()),
            run_time_ticks: 2000000000,
            normalization_gain: None,
            production_year: Some(2023),
            index_number: Some(1),
            parent_index_number: Some(1),
            user_data: UserDataDto {
                play_count: 1,
                is_favorite: false,
                played: true,
            },
        }
    }

    fn create_music_dto_user_data(play_count: u64, is_favorite: bool, played: bool) -> MusicDto {
        MusicDto {
            user_data: UserDataDto {
                play_count,
                is_favorite,
                played,
            },
            id: format!("user-data-{}", play_count),
            name: format!("user-data-{}", play_count),
            album: format!("user-data-{}", play_count),
            album_id: format!("user-data-{}", play_count),
            album_artists: vec![ArtistItemsDto {
                name: format!("user-data-{}", play_count),
                id: format!("user-data-{}", play_count),
            }],
            date_created: Some("2025-01-01".to_string()),
            run_time_ticks: 2000000000,
            normalization_gain: None,
            production_year: Some(2023),
            index_number: Some(1),
            parent_index_number: Some(1),
        }
    }

    #[test]
    fn test_albums_from_library_deduplicates() {
        let library = vec![
            create_test_music_dto(
                "1",
                "Song 1",
                "Album A",
                "album_1",
                "Artist A",
                "artist_1",
                Some(1),
                Some(1),
            ),
            create_test_music_dto(
                "2",
                "Song 2",
                "Album A",
                "album_1",
                "Artist A",
                "artist_1",
                Some(2),
                Some(1),
            ),
            create_test_music_dto(
                "3",
                "Song 3",
                "Album B",
                "album_2",
                "Artist B",
                "artist_2",
                Some(1),
                Some(1),
            ),
        ];

        let albums = albums_from_library(&library);

        assert_eq!(albums.len(), 2);
        assert!(
            albums
                .iter()
                .any(|a| a.id() == "album_1" && a.name() == "Album A")
        );
        assert!(
            albums
                .iter()
                .any(|a| a.id() == "album_2" && a.name() == "Album B")
        );
    }

    #[test]
    fn test_albums_from_library_empty_input() {
        let library = vec![];
        let albums = albums_from_library(&library);
        assert_eq!(albums.len(), 0);
    }

    #[test]
    fn test_artists_from_library_deduplicates_and_sorts() {
        let library = vec![
            create_test_music_dto(
                "1",
                "Song 1",
                "Album A",
                "album_1",
                "Zebra Artist",
                "artist_z",
                Some(1),
                Some(1),
            ),
            create_test_music_dto(
                "2",
                "Song 2",
                "Album A",
                "album_1",
                "Alpha Artist",
                "artist_a",
                Some(2),
                Some(1),
            ),
            create_test_music_dto(
                "3",
                "Song 3",
                "Album B",
                "album_2",
                "Zebra Artist",
                "artist_z",
                Some(1),
                Some(1),
            ), // Duplicate artist
        ];

        let artists = artists_from_library(&library);

        assert_eq!(artists.len(), 2);
        assert_eq!(artists[0].name(), "Alpha Artist");
        assert_eq!(artists[1].name(), "Zebra Artist");
    }

    #[test]
    fn test_artists_from_library_case_insensitive_sort() {
        let library = vec![
            create_test_music_dto(
                "1",
                "Song 1",
                "Album A",
                "album_1",
                "zebra",
                "artist_z",
                Some(1),
                Some(1),
            ),
            create_test_music_dto(
                "2",
                "Song 2",
                "Album B",
                "album_2",
                "Alpha",
                "artist_a",
                Some(1),
                Some(1),
            ),
            create_test_music_dto(
                "3",
                "Song 3",
                "Album C",
                "album_3",
                "beta",
                "artist_b",
                Some(1),
                Some(1),
            ),
        ];

        let artists = artists_from_library(&library);

        assert_eq!(artists.len(), 3);
        assert_eq!(artists[0].name(), "Alpha");
        assert_eq!(artists[1].name(), "beta");
        assert_eq!(artists[2].name(), "zebra");
    }

    #[test]
    fn test_artists_from_library_multiple_artists_per_song() {
        let library = vec![
            create_test_music_dto_multi_artists(
                "1",
                "Collaboration",
                "Album A",
                "album_1",
                vec![("Artist A", "artist_a"), ("Artist B", "artist_b")],
            ),
            create_test_music_dto(
                "2",
                "Solo Track",
                "Album B",
                "album_2",
                "Artist A",
                "artist_a",
                Some(1),
                Some(1),
            ),
        ];

        let artists = artists_from_library(&library);

        assert_eq!(artists.len(), 2);
        assert_eq!(artists[0].name(), "Artist A");
        assert_eq!(artists[1].name(), "Artist B");
    }

    #[test]
    fn test_albums_for_artist_filters_correctly() {
        let library = vec![
            create_test_music_dto(
                "1",
                "Song 1",
                "Album A",
                "album_1",
                "Artist A",
                "artist_1",
                Some(1),
                Some(1),
            ),
            create_test_music_dto(
                "2",
                "Song 2",
                "Album B",
                "album_2",
                "Artist B",
                "artist_2",
                Some(1),
                Some(1),
            ),
            create_test_music_dto(
                "3",
                "Song 3",
                "Album A",
                "album_1",
                "Artist A",
                "artist_1",
                Some(2),
                Some(1),
            ), // Same album
            create_test_music_dto(
                "4",
                "Song 4",
                "Album C",
                "album_3",
                "Artist A",
                "artist_1",
                Some(1),
                Some(1),
            ),
        ];

        let albums = albums_for_artist("artist_1", &library);

        assert_eq!(albums.len(), 2); // Should deduplicate Album A
        assert!(albums.iter().any(|a| a.id() == "album_1"));
        assert!(albums.iter().any(|a| a.id() == "album_3"));
        assert!(!albums.iter().any(|a| a.id() == "album_2")); // Artist B's album should not be included
    }

    #[test]
    fn test_albums_for_artist_no_matches() {
        let library = vec![create_test_music_dto(
            "1",
            "Song 1",
            "Album A",
            "album_1",
            "Artist A",
            "artist_1",
            Some(1),
            Some(1),
        )];

        let albums = albums_for_artist("nonexistent_artist", &library);
        assert_eq!(albums.len(), 0);
    }

    #[test]
    fn test_songs_for_album_filters_and_sorts() {
        let library = vec![
            create_test_music_dto(
                "1",
                "Song 1",
                "Album A",
                "album_1",
                "Artist A",
                "artist_1",
                Some(3),
                Some(1),
            ),
            create_test_music_dto(
                "2",
                "Song 2",
                "Album B",
                "album_2",
                "Artist B",
                "artist_2",
                Some(1),
                Some(1),
            ), // Different album
            create_test_music_dto(
                "3",
                "Song 3",
                "Album A",
                "album_1",
                "Artist A",
                "artist_1",
                Some(1),
                Some(1),
            ),
            create_test_music_dto(
                "4",
                "Song 4",
                "Album A",
                "album_1",
                "Artist A",
                "artist_1",
                Some(2),
                Some(1),
            ),
            create_test_music_dto(
                "5",
                "Song 5",
                "Album A",
                "album_1",
                "Artist A",
                "artist_1",
                Some(1),
                Some(2),
            ), // Different disc
        ];

        let songs = songs_for_album("album_1", &library);

        assert_eq!(songs.len(), 4);

        // Check sorting: first by parent_track_number (disc), then by track_number
        assert_eq!(songs[0].title(), "Song 3"); // Disc 1, Track 1
        assert_eq!(songs[1].title(), "Song 4"); // Disc 1, Track 2
        assert_eq!(songs[2].title(), "Song 1"); // Disc 1, Track 3
        assert_eq!(songs[3].title(), "Song 5"); // Disc 2, Track 1
    }

    #[test]
    fn test_songs_for_album_handles_none_track_numbers() {
        let library = vec![
            create_test_music_dto(
                "1", "Song 1", "Album A", "album_1", "Artist A", "artist_1", None, None,
            ),
            create_test_music_dto(
                "2",
                "Song 2",
                "Album A",
                "album_1",
                "Artist A",
                "artist_1",
                Some(1),
                Some(1),
            ),
        ];

        let songs = songs_for_album("album_1", &library);

        assert_eq!(songs.len(), 2);
        // Song with None values should come first (0, 0) < (1, 1)
        assert_eq!(songs[0].title(), "Song 1");
        assert_eq!(songs[1].title(), "Song 2");
    }

    #[test]
    fn test_shuffle_songs_returns_requested_count() {
        let library = vec![
            create_test_music_dto(
                "1",
                "Song 1",
                "Album A",
                "album_1",
                "Artist A",
                "artist_1",
                Some(1),
                Some(1),
            ),
            create_test_music_dto(
                "2",
                "Song 2",
                "Album B",
                "album_2",
                "Artist B",
                "artist_2",
                Some(1),
                Some(1),
            ),
            create_test_music_dto(
                "3",
                "Song 3",
                "Album C",
                "album_3",
                "Artist C",
                "artist_3",
                Some(1),
                Some(1),
            ),
            create_test_music_dto(
                "4",
                "Song 4",
                "Album D",
                "album_4",
                "Artist D",
                "artist_4",
                Some(1),
                Some(1),
            ),
            create_test_music_dto(
                "5",
                "Song 5",
                "Album E",
                "album_5",
                "Artist E",
                "artist_5",
                Some(1),
                Some(1),
            ),
        ];

        let shuffled = shuffle_songs(&library, 3);
        assert_eq!(shuffled.len(), 3);
    }

    #[test]
    fn test_shuffle_songs_handles_request_larger_than_library() {
        let library = vec![
            create_test_music_dto(
                "1",
                "Song 1",
                "Album A",
                "album_1",
                "Artist A",
                "artist_1",
                Some(1),
                Some(1),
            ),
            create_test_music_dto(
                "2",
                "Song 2",
                "Album B",
                "album_2",
                "Artist B",
                "artist_2",
                Some(1),
                Some(1),
            ),
        ];

        let shuffled = shuffle_songs(&library, 10);
        assert_eq!(shuffled.len(), 2);
    }

    #[test]
    fn test_shuffle_songs_empty_library() {
        let library = vec![];
        let shuffled = shuffle_songs(&library, 5);
        assert_eq!(shuffled.len(), 0);
    }

    #[test]
    fn test_shuffle_songs_zero_request() {
        let library = vec![create_test_music_dto(
            "1",
            "Song 1",
            "Album A",
            "album_1",
            "Artist A",
            "artist_1",
            Some(1),
            Some(1),
        )];

        let shuffled = shuffle_songs(&library, 0);
        assert_eq!(shuffled.len(), 0);
    }

    #[test]
    fn test_albums_from_library_preserves_artist_order() {
        // Test that when converting to AlbumModel, the artist order from MusicDto is preserved
        let library = vec![create_test_music_dto_multi_artists(
            "1",
            "Song 1",
            "Album A",
            "album_1",
            vec![
                ("Primary Artist", "primary"),
                ("Secondary Artist", "secondary"),
            ],
        )];

        let albums = albums_from_library(&library);
        assert_eq!(albums.len(), 1);

        let album = &albums[0];
        assert_eq!(album.primary_artist(), "Primary Artist");
        assert_eq!(album.artists().len(), 2);
        assert_eq!(album.artists()[0], "Primary Artist");
        assert_eq!(album.artists()[1], "Secondary Artist");
    }

    #[test]
    fn test_most_played_songs() {
        let library = vec![
            create_music_dto_user_data(1, false, false),
            create_music_dto_user_data(2, true, false),
            create_music_dto_user_data(3, true, false),
            create_music_dto_user_data(0, false, false),
        ];

        let most_played = most_played_songs(&library, 100);
        assert_eq!(most_played.len(), 3);
        assert_eq!(most_played[0].id, "user-data-3");
        assert_eq!(most_played[1].id, "user-data-2");
        assert_eq!(most_played[2].id, "user-data-1");
        // 0 not included
    }
}
