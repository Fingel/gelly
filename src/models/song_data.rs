use crate::jellyfin::api::MusicDto;
use glib::Object;
use gtk::glib;

glib::wrapper! {
    pub struct SongData(ObjectSubclass<imp::SongData>);
}

impl SongData {
    pub fn new(
        id: &str,
        title: &str,
        artists: Vec<String>,
        album: &str,
        album_id: &str,
        track_number: u32,
        duration: u64,
    ) -> Self {
        Object::builder()
            .property("id", id)
            .property("title", title)
            .property("artists", artists)
            .property("album", album)
            .property("album-id", album_id)
            .property("track-number", track_number)
            .property("duration", duration)
            .build()
    }
}

impl From<&MusicDto> for SongData {
    fn from(dto: &MusicDto) -> Self {
        let artists = dto
            .artist_items
            .iter()
            .map(|artist| artist.name.clone())
            .collect();

        SongData::new(
            &dto.id,
            &dto.name,
            artists,
            &dto.album,
            &dto.album_id,
            dto.index_number,
            dto.run_time_ticks,
        )
    }
}

mod imp {
    use glib::Properties;
    use gtk::{glib, prelude::*, subclass::prelude::*};
    use std::cell::RefCell;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::SongData)]
    pub struct SongData {
        #[property(get, set)]
        id: RefCell<String>,

        #[property(get, set)]
        title: RefCell<String>,

        #[property(get, set)]
        artists: RefCell<Vec<String>>,

        #[property(get, set)]
        album: RefCell<String>,

        #[property(get, set)]
        album_id: RefCell<String>,

        #[property(get, set, name = "track-number")]
        track_number: RefCell<u32>,

        #[property(get, set)]
        duration: RefCell<u64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SongData {
        const NAME: &'static str = "GellySongData";
        type Type = super::SongData;
        type ParentType = glib::Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for SongData {}
}
