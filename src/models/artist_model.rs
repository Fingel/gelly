use glib::Object;
use gtk::{glib, subclass::prelude::*};

use crate::{jellyfin::api::ArtistItemsDto, models::model_traits::ItemModel};

glib::wrapper! {
    pub struct ArtistModel(ObjectSubclass<imp::ArtistModel>);
}

impl ItemModel for ArtistModel {
    fn display_name(&self) -> String {
        self.name()
    }

    fn item_id(&self) -> String {
        self.id()
    }
}

impl ArtistModel {
    pub fn new(name: &str, id: &str) -> Self {
        Object::builder()
            .property("name", name)
            .property("id", id)
            .property("play-count", 0u64)
            .build()
    }

    pub fn set_image_data(&self, image_data: Vec<u8>) {
        self.imp().image_data.replace(image_data);
    }

    pub fn image_data(&self) -> Vec<u8> {
        self.imp().image_data.borrow().clone()
    }
}

impl From<&ArtistItemsDto> for ArtistModel {
    fn from(dto: &ArtistItemsDto) -> Self {
        ArtistModel::new(&dto.name, &dto.id)
    }
}

mod imp {
    use glib::Properties;
    use gtk::{glib, prelude::*, subclass::prelude::*};
    use std::cell::{Cell, RefCell};

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::ArtistModel)]
    pub struct ArtistModel {
        #[property(get, set)]
        pub name: RefCell<String>,
        #[property(get, set)]
        pub id: RefCell<String>,
        #[property(get, set, name = "image-loading")]
        pub image_loading: Cell<bool>,
        #[property(get, set, name = "image-loaded")]
        pub image_loaded: Cell<bool>,
        #[property(get, set)]
        pub play_count: Cell<u64>,
        pub image_data: RefCell<Vec<u8>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ArtistModel {
        const NAME: &'static str = "GellyArtistModel";
        type Type = super::ArtistModel;
        type ParentType = glib::Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ArtistModel {}
}
