use crate::jellyfin::api::MusicDto;
use glib::Object;
use gtk::glib;

glib::wrapper! {
    pub struct SongModel(ObjectSubclass<imp::SongData>);
}

impl SongModel {
    pub fn new(dto: &MusicDto, favorite: bool) -> Self {
        let artists: Vec<String> = dto
            .album_artists
            .iter()
            .map(|artist| artist.name.clone())
            .collect();
        let artists_string = artists.join(", ");
        let date_created = dto.date_created.clone().unwrap_or("".to_string());
        Object::builder()
            .property("id", &dto.id)
            .property("title", &dto.name)
            .property("artists", artists)
            .property("artists-string", artists_string)
            .property("album", dto.album.as_deref().unwrap_or("Unknown Album"))
            .property("album-id", dto.effective_album_id())
            .property("track-number", dto.index_number.unwrap_or(0))
            .property("parent-track-number", dto.parent_index_number.unwrap_or(0))
            .property("duration", dto.run_time_ticks)
            .property("has-lyrics", dto.has_lyrics)
            .property("normalization-gain", dto.normalization_gain.unwrap_or(0.0))
            .property("date-created", date_created)
            .property("favorite", favorite)
            .build()
    }

    pub fn duration_seconds(&self) -> u64 {
        self.duration() / 10_000_000 // Jellyfin ticks
    }
}

mod imp {
    use glib::Properties;
    use gtk::{glib, prelude::*, subclass::prelude::*};
    use std::cell::{Cell, RefCell};

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::SongModel)]
    pub struct SongData {
        #[property(get, set)]
        id: RefCell<String>,

        #[property(get, set)]
        title: RefCell<String>,

        #[property(get, set)]
        artists: RefCell<Vec<String>>,

        #[property(get, set, name = "artists-string")]
        artists_string: RefCell<String>,

        #[property(get, set)]
        album: RefCell<String>,

        #[property(get, set)]
        album_id: RefCell<String>,

        #[property(get, set, name = "track-number")]
        track_number: RefCell<u32>,

        #[property(get, set, name = "parent-track-number")]
        parent_track_number: RefCell<u32>,

        #[property(get, set)]
        duration: RefCell<u64>,

        #[property(get, set, name = "has-lyrics")]
        has_lyrics: Cell<bool>,

        #[property(get, set)]
        normalization_gain: Cell<f64>,

        #[property(get, set, name = "date-created")]
        pub date_created: RefCell<String>,

        #[property(get, set)]
        pub favorite: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SongData {
        const NAME: &'static str = "GellySongData";
        type Type = super::SongModel;
        type ParentType = glib::Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for SongData {}
}
