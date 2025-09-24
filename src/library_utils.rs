use crate::jellyfin::api::MusicDto;
use crate::models::album_data::AlbumData;
use std::collections::HashSet;

pub fn albums_from_library(library: &[MusicDto]) -> Vec<AlbumData> {
    let mut seen_album_ids = HashSet::new();
    let albums: Vec<AlbumData> = library
        .iter()
        .filter(|dto| seen_album_ids.insert(&dto.album_id))
        .map(AlbumData::from)
        .collect();

    albums
}

pub fn tracks_for_album(album_id: &str, library: &[MusicDto]) -> Vec<MusicDto> {
    let mut tracks: Vec<MusicDto> = library
        .iter()
        .filter(|dto| dto.album_id == album_id)
        .cloned()
        .collect();
    tracks.sort_by_key(|t| t.index_number);
    tracks
}
