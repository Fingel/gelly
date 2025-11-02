use crate::jellyfin::api::MusicDto;
use glib::Object;
use gtk::{glib, subclass::prelude::*};

glib::wrapper! {
    pub struct AlbumModel(ObjectSubclass<imp::AlbumData>);
}

/// Simple GObject to provide album data, and to convert from the API response from jellyfin.
impl AlbumModel {
    pub fn new(
        name: &str,
        id: &str,
        artists: Vec<String>,
        date_created: &str,
        year: &Option<u32>,
    ) -> Self {
        let artists_string = artists.join(", ");

        Object::builder()
            .property("name", name)
            .property("id", id)
            .property("artists", artists)
            .property("date-created", date_created)
            .property("image-loading", false)
            .property("image-loaded", false)
            .property("year", year.unwrap_or(0))
            .property("artists-string", artists_string)
            .build()
    }

    /// Get the primary artist (first in the list) or empty string
    pub fn primary_artist(&self) -> String {
        let artists = self.artists();
        artists.first().cloned().unwrap_or_default()
    }

    pub fn set_image_data(&self, image_data: Vec<u8>) {
        self.imp().image_data.replace(image_data);
    }

    pub fn image_data(&self) -> Vec<u8> {
        self.imp().image_data.borrow().clone()
    }
}

impl From<&MusicDto> for AlbumModel {
    fn from(dto: &MusicDto) -> Self {
        let artists = dto
            .artist_items
            .iter()
            .map(|artist| artist.name.clone())
            .collect();

        AlbumModel::new(
            &dto.album,
            &dto.album_id,
            artists,
            &dto.date_created,
            &dto.production_year,
        )
    }
}

mod imp {
    use glib::Properties;
    use gtk::{glib, prelude::*, subclass::prelude::*};
    use std::cell::RefCell;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::AlbumModel)]
    pub struct AlbumData {
        #[property(get, set)]
        pub name: RefCell<String>,

        #[property(get, set)]
        pub id: RefCell<String>,

        #[property(get, set)]
        pub artists: RefCell<Vec<String>>,

        #[property(get, set, name = "date-created")]
        pub date_created: RefCell<String>,

        #[property(get, set)]
        pub year: RefCell<u32>,

        #[property(get, set, name = "image-loading")]
        pub image_loading: RefCell<bool>,

        #[property(get, set, name = "image-loaded")]
        pub image_loaded: RefCell<bool>,

        #[property(get, set, name = "artists-string")]
        pub artists_string: RefCell<String>,

        pub image_data: RefCell<Vec<u8>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AlbumData {
        const NAME: &'static str = "GellyAlbumData";
        type Type = super::AlbumModel;
        type ParentType = glib::Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for AlbumData {}
}
