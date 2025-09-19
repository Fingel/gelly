use crate::jellyfin::api::MusicDto;
use crate::models::album_data::AlbumData;
use std::collections::HashSet;

pub fn albums_from_library(library: &[MusicDto]) -> Vec<AlbumData> {
    let mut seen_album_ids = HashSet::new();
    let albums: Vec<AlbumData> = library
        .iter()
        .filter_map(|dto| {
            if seen_album_ids.insert(&dto.album_id) {
                Some(AlbumData::from(dto))
            } else {
                None
            }
        })
        .collect();

    albums
}
