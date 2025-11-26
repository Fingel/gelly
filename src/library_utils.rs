use crate::application::Application;
use crate::async_utils::spawn_tokio;
use crate::jellyfin::JellyfinError;
use crate::jellyfin::api::MusicDto;
use crate::models::{AlbumModel, ArtistModel, PlaylistModel, SongModel};
use rand::prelude::*;
use std::collections::HashSet;

pub fn albums_from_library(library: &[MusicDto]) -> Vec<AlbumModel> {
    let mut seen_album_ids = HashSet::new();
    let albums: Vec<AlbumModel> = library
        .iter()
        .filter(|dto| seen_album_ids.insert(&dto.album_id))
        .map(AlbumModel::from)
        .collect();

    albums
}

pub fn artists_from_library(library: &[MusicDto]) -> Vec<ArtistModel> {
    let mut seen_artist_ids = HashSet::new();
    let mut artists: Vec<ArtistModel> = library
        .iter()
        .flat_map(|dto| &dto.album_artists)
        .filter(|artist| seen_artist_ids.insert(&artist.id))
        .map(ArtistModel::from)
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

pub fn songs_for_ids(ids: Vec<String>, library: &[MusicDto]) -> Vec<MusicDto> {
    library
        .iter()
        .filter(|dto| ids.contains(&dto.id))
        .cloned()
        .collect()
}

pub fn shuffle_songs(library: &[MusicDto], num: u64) -> Vec<MusicDto> {
    let mut rng = rand::rng();
    let chosen = library.choose_multiple(&mut rng, num as usize);
    chosen.into_iter().cloned().collect()
}

pub fn songs_for_playlist(
    playlist_model: &PlaylistModel,
    app: &Application,
    cb: impl Fn(Result<Vec<MusicDto>, JellyfinError>) + 'static,
) {
    let library_data = app.library().borrow().clone();
    let jellyfin = app.jellyfin();
    let id = playlist_model.id().to_string();
    let playlist_type = playlist_model.playlist_type();
    spawn_tokio(
        async move {
            playlist_type
                .load_song_data(&id, &jellyfin, &library_data)
                .await
        },
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
