use crate::jellyfin::api::MusicDto;
use crate::models::{AlbumModel, ArtistModel};
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
    let artists: Vec<ArtistModel> = library
        .iter()
        .flat_map(|dto| &dto.artist_items)
        .filter(|artist| seen_artist_ids.insert(&artist.id))
        .map(ArtistModel::from)
        .collect();

    artists
}

pub fn tracks_for_album(album_id: &str, library: &[MusicDto]) -> Vec<MusicDto> {
    // TODO: should we be converting to SongModel here?
    let mut tracks: Vec<MusicDto> = library
        .iter()
        .filter(|dto| dto.album_id == album_id)
        .cloned()
        .collect();
    tracks.sort_by_key(|t| t.index_number);
    tracks
}
