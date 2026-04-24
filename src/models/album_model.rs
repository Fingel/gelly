use crate::{
    application::Application, async_utils::spawn_tokio, jellyfin::api::MusicDto,
    models::model_traits::ItemModel,
};
use glib::Object;
use gtk::{glib, subclass::prelude::*};
use log::warn;

glib::wrapper! {
    pub struct AlbumModel(ObjectSubclass<imp::AlbumData>);
}

impl ItemModel for AlbumModel {
    fn display_name(&self) -> String {
        self.name()
    }

    fn item_id(&self) -> String {
        self.id()
    }
}

/// Simple GObject to provide album data, and to convert from the API response from jellyfin.
impl AlbumModel {
    pub fn new(dto: &MusicDto, favorite: bool, play_count: u64) -> Self {
        let artists: Vec<String> = dto
            .album_artists
            .iter()
            .map(|artist| artist.name.clone())
            .collect();
        let artists_string = artists.join(", ");

        let date_created = dto.date_created.clone().unwrap_or("".to_string());

        Object::builder()
            .property("name", dto.album.as_deref().unwrap_or("Unknown Album"))
            .property("id", dto.effective_album_id())
            .property("artists", artists)
            .property("date-created", date_created)
            .property("image-loading", false)
            .property("image-loaded", false)
            .property("year", dto.production_year.unwrap_or(0))
            .property("artists-string", artists_string)
            .property("play-count", play_count)
            .property("favorite", favorite)
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

    pub fn toggle_favorite(&self, is_favorite: bool, app: &Application) {
        self.set_favorite(is_favorite);
        let backend = app.jellyfin();
        let item_id = self.id();
        let app = app.clone();
        spawn_tokio(
            async move {
                backend
                    .set_favorite(
                        &item_id,
                        &crate::jellyfin::api::ItemType::MusicAlbum,
                        is_favorite,
                    )
                    .await
            },
            glib::clone!(
                #[weak(rename_to = album)]
                self,
                #[weak]
                app,
                move |result| {
                    match result {
                        Ok(()) => app.refresh_favorites(true),
                        Err(err) => {
                            warn!("Failed to set favorite: {err}");
                            album.set_favorite(!is_favorite);
                        }
                    }
                }
            ),
        );
    }
}

mod imp {
    use glib::Properties;
    use gtk::{glib, prelude::*, subclass::prelude::*};
    use std::cell::{Cell, RefCell};

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
        pub year: Cell<u32>,

        #[property(get, set, name = "image-loading")]
        pub image_loading: Cell<bool>,

        #[property(get, set, name = "image-loaded")]
        pub image_loaded: Cell<bool>,

        #[property(get, set, name = "artists-string")]
        pub artists_string: RefCell<String>,

        #[property(get, set)]
        pub play_count: Cell<u64>,

        #[property(get, set)]
        pub favorite: Cell<bool>,

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
