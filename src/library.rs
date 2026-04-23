use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use rand::prelude::*;

use crate::{
    jellyfin::api::{FavoriteDto, ItemType, MusicDto},
    models::{AlbumModel, ArtistModel, SongModel},
};

#[derive(Debug, Clone, Default)]
struct Favorites {
    song_ids: HashSet<String>,
    album_ids: HashSet<String>,
    artist_ids: HashSet<String>,
    playlist_ids: HashSet<String>,
}

impl Favorites {
    fn contains_song(&self, id: &str) -> bool {
        self.song_ids.contains(id)
    }
    fn contains_album(&self, id: &str) -> bool {
        self.album_ids.contains(id)
    }
    fn contains_artist(&self, id: &str) -> bool {
        self.artist_ids.contains(id)
    }
    fn contains_playlist(&self, id: &str) -> bool {
        self.playlist_ids.contains(id)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Library {
    pub songs: Rc<RefCell<Vec<MusicDto>>>, // TODO make this private
    favorites: Rc<RefCell<Favorites>>,
    album_play_counts: Rc<RefCell<HashMap<String, u64>>>,
    artist_play_counts: Rc<RefCell<HashMap<String, u64>>>,
}

impl Library {
    pub fn new() -> Self {
        Self {
            songs: Rc::new(RefCell::new(Vec::new())),
            favorites: Rc::new(RefCell::new(Favorites::default())),
            album_play_counts: Rc::new(RefCell::new(HashMap::new())),
            artist_play_counts: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    pub fn update_songs(&self, songs: Vec<MusicDto>) {
        let mut album_counts = HashMap::new();
        let mut artist_counts = HashMap::new();
        for dto in songs.iter() {
            if dto.album.is_some() {
                *album_counts.entry(dto.effective_album_id()).or_insert(0) +=
                    dto.user_data.play_count;
            }
            for artist in &dto.album_artists {
                *artist_counts.entry(artist.id.clone()).or_insert(0) += dto.user_data.play_count;
            }
        }
        self.album_play_counts.replace(album_counts);
        self.artist_play_counts.replace(artist_counts);
        self.songs.replace(songs);
    }

    pub fn update_favorites(&self, favorites_list: &[FavoriteDto]) {
        let mut favorites = self.favorites.borrow_mut();
        favorites.song_ids.clear();
        favorites.album_ids.clear();
        favorites.artist_ids.clear();
        favorites.playlist_ids.clear();
        for favorite in favorites_list.iter().filter(|f| f.user_data.is_favorite) {
            match favorite.item_type {
                ItemType::Audio => {
                    favorites.song_ids.insert(favorite.id.clone());
                }
                ItemType::MusicAlbum => {
                    favorites.album_ids.insert(favorite.id.clone());
                }
                ItemType::MusicArtist => {
                    favorites.artist_ids.insert(favorite.id.clone());
                }
                ItemType::Playlist => {
                    favorites.playlist_ids.insert(favorite.id.clone());
                }
                _ => log::warn!("Unknown favorite type: {:?}", favorite.item_type),
            }
        }
    }

    pub fn albums_from_library(&self) -> Vec<AlbumModel> {
        let play_counts = self.album_play_counts.borrow();
        let favorites = self.favorites.borrow();
        let mut seen_album_ids = HashSet::<String>::new();
        let mut albums: Vec<AlbumModel> = self
            .songs
            .borrow()
            .iter()
            .filter(|dto| dto.album.is_some())
            .filter(|dto| seen_album_ids.insert(dto.effective_album_id()))
            .map(|dto| {
                let id = dto.effective_album_id();
                AlbumModel::new(
                    dto,
                    favorites.contains_album(&id),
                    play_counts.get(&id).copied().unwrap_or(0),
                )
            })
            .collect();
        albums.sort_by_key(|album| std::cmp::Reverse(album.date_created()));
        albums
    }

    pub fn artists_from_library(&self) -> Vec<ArtistModel> {
        let play_counts = self.artist_play_counts.borrow();
        let favorites = self.favorites.borrow();
        let mut seen_artist_ids = HashSet::new();
        let mut artists: Vec<ArtistModel> = self
            .songs
            .borrow()
            .iter()
            .flat_map(|dto| &dto.album_artists)
            .filter(|artist| seen_artist_ids.insert(&artist.id))
            .map(|dto| {
                let play_count = play_counts.get(&dto.id).copied().unwrap_or(0);
                let favorite = favorites.contains_artist(&dto.id);
                ArtistModel::new(dto, favorite, play_count)
            })
            .collect();
        artists.sort_by_key(|artist| artist.name().to_lowercase());
        artists
    }

    pub fn all_songs(&self) -> Vec<SongModel> {
        let favorites = self.favorites.borrow();
        let mut songs: Vec<SongModel> = self
            .songs
            .borrow()
            .iter()
            .map(|dto| {
                let favorite = favorites.contains_song(&dto.id);
                SongModel::new(dto, favorite)
            })
            .collect();
        songs.sort_by_key(|s| std::cmp::Reverse(s.date_created()));
        songs
    }

    pub fn albums_for_artist(&self, artist_id: &str) -> Vec<AlbumModel> {
        let play_counts = self.album_play_counts.borrow();
        let favorites = self.favorites.borrow();
        let mut seen_album_ids = HashSet::<String>::new();
        let mut albums: Vec<AlbumModel> = self
            .songs
            .borrow()
            .iter()
            .filter(|dto| {
                dto.album_artists
                    .iter()
                    .any(|artist| artist.id == artist_id)
            })
            .filter(|dto| seen_album_ids.insert(dto.effective_album_id()))
            .map(|dto| {
                let id = dto.effective_album_id();
                AlbumModel::new(
                    dto,
                    favorites.contains_album(&id),
                    play_counts.get(&id).copied().unwrap_or(0),
                )
            })
            .collect();
        albums.sort_by_key(|album| std::cmp::Reverse(album.year()));
        albums
    }

    pub fn songs_for_album(&self, album_id: &str) -> Vec<SongModel> {
        let favorites = self.favorites.borrow();
        let mut tracks: Vec<SongModel> = self
            .songs
            .borrow()
            .iter()
            .filter(|dto| dto.effective_album_id() == album_id)
            .map(|dto| {
                let favorite = favorites.contains_song(&dto.id);
                SongModel::new(dto, favorite)
            })
            .collect();
        tracks.sort_by_key(|t| (t.parent_track_number(), t.track_number()));
        tracks
    }

    pub fn shuffle_songs(&self, num: u64) -> Vec<MusicDto> {
        let mut rng = rand::rng();
        let songs = self.songs.borrow();
        let chosen = songs.sample(&mut rng, num as usize);
        chosen.into_iter().cloned().collect()
    }

    pub fn most_played_songs(&self, num: u64) -> Vec<MusicDto> {
        let mut songs: Vec<MusicDto> = self
            .songs
            .borrow()
            .iter()
            .filter(|dto| dto.user_data.play_count > 0)
            .cloned()
            .collect();
        songs.sort_by_key(|dto| std::cmp::Reverse(dto.user_data.play_count));
        songs.into_iter().take(num as usize).collect()
    }

    pub fn songs_for_artist(&self, id: &str) -> Vec<SongModel> {
        let albums = self.albums_for_artist(id);
        albums
            .iter()
            .flat_map(|album| self.songs_for_album(&album.id()))
            .collect()
    }

    pub fn artist_for_item(&self, item_id: &str) -> Option<ArtistModel> {
        let play_counts = self.artist_play_counts.borrow();
        let favorites = self.favorites.borrow();
        self.songs
            .borrow()
            .iter()
            .find(|dto| dto.id == item_id)
            .and_then(|dto| {
                dto.album_artists.first().map(|artist| {
                    let play_count = play_counts.get(&artist.id).copied().unwrap_or(0);
                    let favorite = favorites.contains_artist(&artist.id);
                    ArtistModel::new(artist, favorite, play_count)
                })
            })
    }

    pub fn album_for_item(&self, item_id: &str) -> Option<AlbumModel> {
        let play_counts = self.album_play_counts.borrow();
        let favorites = self.favorites.borrow();
        self.songs
            .borrow()
            .iter()
            .find(|dto| dto.id == item_id)
            .map(|dto| {
                let id = dto.effective_album_id();
                AlbumModel::new(
                    dto,
                    favorites.contains_album(&id),
                    play_counts.get(&id).copied().unwrap_or(0),
                )
            })
    }

    pub fn songs(&self) -> Vec<SongModel> {
        self.all_songs()
    }

    pub fn playlist_is_favorite(&self, id: &str) -> bool {
        // Playlists are weird so we handle this in the list view instead
        self.favorites.borrow().contains_playlist(id)
    }

    pub fn song_is_favorite(&self, id: &str) -> bool {
        self.favorites.borrow().contains_song(id)
    }

    pub fn library_size(&self) -> usize {
        self.songs.borrow().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jellyfin::api::{ArtistItemsDto, FavoriteUserDataDto, UserDataDto};

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
            album: Some(album.to_string()),
            album_id: Some(album_id.to_string()),
            album_artists: vec![ArtistItemsDto {
                name: artist_name.to_string(),
                id: artist_id.to_string(),
            }],
            date_created: Some("2025-01-01".to_string()),
            run_time_ticks: 2000000000,
            normalization_gain: None,
            production_year: Some(2023),
            index_number,
            parent_index_number,
            has_lyrics: false,
            user_data: UserDataDto { play_count: 1 },
        }
    }

    fn create_test_music_dto_multi_artists(
        id: &str,
        name: &str,
        album: &str,
        album_id: &str,
        artists: Vec<(&str, &str)>,
    ) -> MusicDto {
        MusicDto {
            id: id.to_string(),
            name: name.to_string(),
            album: Some(album.to_string()),
            album_id: Some(album_id.to_string()),
            album_artists: artists
                .into_iter()
                .map(|(name, id)| ArtistItemsDto {
                    name: name.to_string(),
                    id: id.to_string(),
                })
                .collect(),
            date_created: Some("2025-01-01".to_string()),
            run_time_ticks: 2000000000,
            normalization_gain: None,
            production_year: Some(2023),
            index_number: Some(1),
            parent_index_number: Some(1),
            has_lyrics: false,
            user_data: UserDataDto { play_count: 1 },
        }
    }

    fn create_music_dto_user_data(play_count: u64) -> MusicDto {
        MusicDto {
            user_data: UserDataDto { play_count },
            id: format!("user-data-{}", play_count),
            name: format!("user-data-{}", play_count),
            album: Some(format!("user-data-{}", play_count)),
            album_id: Some(format!("user-data-{}", play_count)),
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
            has_lyrics: false,
        }
    }

    fn make_library(songs: Vec<MusicDto>) -> Library {
        let lib = Library::new();
        lib.update_songs(songs);
        lib
    }

    #[test]
    fn test_albums_from_library_deduplicates() {
        let lib = make_library(vec![
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
        ]);
        let albums = lib.albums_from_library();
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
        let lib = make_library(vec![]);
        assert_eq!(lib.albums_from_library().len(), 0);
    }

    #[test]
    fn test_artists_from_library_deduplicates_and_sorts() {
        let lib = make_library(vec![
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
            ),
        ]);
        let artists = lib.artists_from_library();
        assert_eq!(artists.len(), 2);
        assert_eq!(artists[0].name(), "Alpha Artist");
        assert_eq!(artists[1].name(), "Zebra Artist");
    }

    #[test]
    fn test_artists_from_library_case_insensitive_sort() {
        let lib = make_library(vec![
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
        ]);
        let artists = lib.artists_from_library();
        assert_eq!(artists.len(), 3);
        assert_eq!(artists[0].name(), "Alpha");
        assert_eq!(artists[1].name(), "beta");
        assert_eq!(artists[2].name(), "zebra");
    }

    #[test]
    fn test_artists_from_library_multiple_artists_per_song() {
        let lib = make_library(vec![
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
        ]);
        let artists = lib.artists_from_library();
        assert_eq!(artists.len(), 2);
        assert_eq!(artists[0].name(), "Artist A");
        assert_eq!(artists[1].name(), "Artist B");
    }

    #[test]
    fn test_albums_for_artist_filters_correctly() {
        let lib = make_library(vec![
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
            ),
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
        ]);
        let albums = lib.albums_for_artist("artist_1");
        assert_eq!(albums.len(), 2);
        assert!(albums.iter().any(|a| a.id() == "album_1"));
        assert!(albums.iter().any(|a| a.id() == "album_3"));
        assert!(!albums.iter().any(|a| a.id() == "album_2"));
    }

    #[test]
    fn test_albums_for_artist_no_matches() {
        let lib = make_library(vec![create_test_music_dto(
            "1",
            "Song 1",
            "Album A",
            "album_1",
            "Artist A",
            "artist_1",
            Some(1),
            Some(1),
        )]);
        assert_eq!(lib.albums_for_artist("nonexistent_artist").len(), 0);
    }

    #[test]
    fn test_songs_for_album_filters_and_sorts() {
        let lib = make_library(vec![
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
            ),
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
            ),
        ]);
        let songs = lib.songs_for_album("album_1");
        assert_eq!(songs.len(), 4);
        assert_eq!(songs[0].title(), "Song 3");
        assert_eq!(songs[1].title(), "Song 4");
        assert_eq!(songs[2].title(), "Song 1");
        assert_eq!(songs[3].title(), "Song 5");
    }

    #[test]
    fn test_songs_for_album_handles_none_track_numbers() {
        let lib = make_library(vec![
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
        ]);
        let songs = lib.songs_for_album("album_1");
        assert_eq!(songs.len(), 2);
        assert_eq!(songs[0].title(), "Song 1");
        assert_eq!(songs[1].title(), "Song 2");
    }

    #[test]
    fn test_shuffle_songs_returns_requested_count() {
        let lib = make_library(vec![
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
        ]);
        assert_eq!(lib.shuffle_songs(3).len(), 3);
    }

    #[test]
    fn test_shuffle_songs_handles_request_larger_than_library() {
        let lib = make_library(vec![
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
        ]);
        assert_eq!(lib.shuffle_songs(10).len(), 2);
    }

    #[test]
    fn test_shuffle_songs_empty_library() {
        let lib = make_library(vec![]);
        assert_eq!(lib.shuffle_songs(5).len(), 0);
    }

    #[test]
    fn test_shuffle_songs_zero_request() {
        let lib = make_library(vec![create_test_music_dto(
            "1",
            "Song 1",
            "Album A",
            "album_1",
            "Artist A",
            "artist_1",
            Some(1),
            Some(1),
        )]);
        assert_eq!(lib.shuffle_songs(0).len(), 0);
    }

    #[test]
    fn test_albums_from_library_preserves_artist_order() {
        let lib = make_library(vec![create_test_music_dto_multi_artists(
            "1",
            "Song 1",
            "Album A",
            "album_1",
            vec![
                ("Primary Artist", "primary"),
                ("Secondary Artist", "secondary"),
            ],
        )]);
        let albums = lib.albums_from_library();
        assert_eq!(albums.len(), 1);
        assert_eq!(albums[0].primary_artist(), "Primary Artist");
        assert_eq!(albums[0].artists().len(), 2);
        assert_eq!(albums[0].artists()[0], "Primary Artist");
        assert_eq!(albums[0].artists()[1], "Secondary Artist");
    }

    #[test]
    fn test_most_played_songs() {
        let lib = make_library(vec![
            create_music_dto_user_data(1),
            create_music_dto_user_data(2),
            create_music_dto_user_data(3),
            create_music_dto_user_data(0),
        ]);
        let most_played = lib.most_played_songs(100);
        assert_eq!(most_played.len(), 3);
        assert_eq!(most_played[0].id, "user-data-3");
        assert_eq!(most_played[1].id, "user-data-2");
        assert_eq!(most_played[2].id, "user-data-1");
    }

    #[test]
    fn test_favorites() {
        let lib = make_library(vec![
            create_music_dto_user_data(1),
            create_music_dto_user_data(2),
        ]);
        lib.update_favorites(&[
            FavoriteDto {
                id: lib.all_songs()[0].id().to_string(),
                item_type: ItemType::Audio,
                user_data: FavoriteUserDataDto { is_favorite: true },
            },
            FavoriteDto {
                id: lib.all_songs()[1].id().to_string(),
                item_type: ItemType::Audio,
                user_data: FavoriteUserDataDto { is_favorite: false },
            },
        ]);
        assert!(
            lib.favorites
                .borrow()
                .contains_song(&lib.all_songs()[0].id())
        );
        assert!(
            !lib.favorites
                .borrow()
                .contains_song(&lib.all_songs()[1].id())
        );
    }
}
