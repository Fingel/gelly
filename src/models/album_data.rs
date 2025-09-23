use crate::jellyfin::api::MusicDto;
use glib::Object;
use gtk::{glib, subclass::prelude::*};

glib::wrapper! {
    pub struct AlbumData(ObjectSubclass<imp::AlbumData>);
}

/// Simple GObject to provide album data, and to convert from the API response from jellyfin.
impl AlbumData {
    pub fn new(
        name: &str,
        id: &str,
        artists: Vec<String>,
        date_created: &str,
        image_tag: &str,
    ) -> Self {
        Object::builder()
            .property("name", name)
            .property("id", id)
            .property("artists", artists)
            .property("date-created", date_created)
            .property("image-tag", image_tag)
            .property("image-loading", false)
            .property("image-loaded", false)
            .build()
    }

    /// Get the primary artist (first in the list) or empty string
    pub fn primary_artist(&self) -> String {
        let artists = self.artists();
        artists.first().cloned().unwrap_or_default()
    }

    /// Get all artists joined by ", "
    pub fn artists_string(&self) -> String {
        self.artists().join(", ")
    }

    pub fn set_image_data(&self, image_data: Vec<u8>) {
        self.imp().image_data.replace(image_data);
    }

    pub fn image_data(&self) -> Vec<u8> {
        self.imp().image_data.borrow().clone()
    }
}

impl From<&MusicDto> for AlbumData {
    fn from(dto: &MusicDto) -> Self {
        let artists = dto
            .artist_items
            .iter()
            .map(|artist| artist.name.clone())
            .collect();

        AlbumData::new(
            &dto.album,
            &dto.album_id,
            artists,
            &dto.date_created,
            &dto.album_primary_image_tag,
        )
    }
}

mod imp {
    use glib::Properties;
    use gtk::{glib, prelude::*, subclass::prelude::*};
    use std::cell::RefCell;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::AlbumData)]
    pub struct AlbumData {
        #[property(get, set)]
        pub name: RefCell<String>,

        #[property(get, set)]
        pub id: RefCell<String>,

        #[property(get, set)]
        pub artists: RefCell<Vec<String>>,

        #[property(get, set, name = "date-created")]
        pub date_created: RefCell<String>,

        #[property(get, set, name = "image-tag")]
        pub image_tag: RefCell<String>,

        #[property(get, set, name = "image-loading")]
        pub image_loading: RefCell<bool>,

        #[property(get, set, name = "image-loaded")]
        pub image_loaded: RefCell<bool>,

        pub image_data: RefCell<Vec<u8>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AlbumData {
        const NAME: &'static str = "GellyAlbumData";
        type Type = super::AlbumData;
        type ParentType = glib::Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for AlbumData {}
}
