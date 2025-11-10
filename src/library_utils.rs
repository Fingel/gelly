use crate::jellyfin::api::MusicDto;
use crate::models::{AlbumModel, ArtistModel, SongModel};
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
    tracks.sort_by_key(|t| t.track_number());
    tracks
}

pub fn songs_for_ids(ids: Vec<String>, library: &[MusicDto]) -> Vec<SongModel> {
    library
        .iter()
        .filter(|dto| ids.contains(&dto.id))
        .map(SongModel::from)
        .collect()
}
