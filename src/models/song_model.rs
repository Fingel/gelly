use crate::jellyfin::api::MusicDto;
use glib::Object;
use gtk::glib;

glib::wrapper! {
    pub struct SongModel(ObjectSubclass<imp::SongData>);
}

impl SongModel {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: &str,
        title: &str,
        artists: Vec<String>,
        album: &str,
        album_id: &str,
        track_number: u32,
        parent_track_number: u32,
        duration: u64,
        has_lyrics: bool,
        normalization_gain: f64,
        date_created: &str,
    ) -> Self {
        let artists_string = artists.join(", ");
        Object::builder()
            .property("id", id)
            .property("title", title)
            .property("artists", artists)
            .property("artists-string", artists_string)
            .property("album", album)
            .property("album-id", album_id)
            .property("track-number", track_number)
            .property("parent-track-number", parent_track_number)
            .property("duration", duration)
            .property("has-lyrics", has_lyrics)
            .property("normalization-gain", normalization_gain)
            .property("date-created", date_created)
            .build()
    }

    pub fn duration_seconds(&self) -> u64 {
        self.duration() / 10_000_000 // Jellyfin ticks
    }
}

impl From<&MusicDto> for SongModel {
    fn from(dto: &MusicDto) -> Self {
        let artists = dto
            .album_artists
            .iter()
            .map(|artist| artist.name.clone())
            .collect();
        let date_created = dto.date_created.clone().unwrap_or("".to_string());
        SongModel::new(
            &dto.id,
            &dto.name,
            artists,
            &dto.album,
            &dto.album_id,
            dto.index_number.unwrap_or(0),
            dto.parent_index_number.unwrap_or(0),
            dto.run_time_ticks,
            dto.has_lyrics,
            dto.normalization_gain.unwrap_or(0.0),
            &date_created,
        )
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
