use crate::jellyfin::api::MusicDto;
use std::collections::HashSet;

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct GellyAlbum {
    pub date_created: String,
    pub name: String,
    pub id: String,
    pub artists: Vec<String>,
    pub image_tag: String,
}

impl From<&MusicDto> for GellyAlbum {
    fn from(dto: &MusicDto) -> Self {
        let artists = dto
            .artist_items
            .iter()
            .map(|artist| artist.name.clone())
            .collect();

        GellyAlbum {
            date_created: dto.date_created.clone(),
            name: dto.album.clone(),
            id: dto.album_id.clone(),
            image_tag: dto.album_primary_image_tag.clone(),
            artists,
        }
    }
}

pub fn albums_from_library(library: &[MusicDto]) -> Vec<GellyAlbum> {
    let mut seen_album_ids = HashSet::new();
    let albums: Vec<GellyAlbum> = library
        .iter()
        .filter_map(|dto| {
            if seen_album_ids.insert(&dto.album_id) {
                Some(GellyAlbum::from(dto))
            } else {
                None
            }
        })
        .collect();

    albums
}
