use crate::models::{PlaylistType, model_traits::ItemModel};
use glib::Object;
use gtk::glib;

glib::wrapper! {
    pub struct PlaylistModel(ObjectSubclass<imp::PlaylistModel>);
}

impl ItemModel for PlaylistModel {
    fn display_name(&self) -> String {
        self.name()
    }

    fn item_id(&self) -> String {
        self.id()
    }
}

impl PlaylistModel {
    pub fn new(playlist_type: PlaylistType) -> Self {
        Object::builder()
            .property("id", playlist_type.to_id())
            .property("name", playlist_type.display_name())
            .property("child_count", playlist_type.estimated_count())
            .property("favorite", playlist_type.favorite())
            .build()
    }

    pub fn is_smart(&self) -> bool {
        self.id().starts_with("smart:")
    }

    pub fn playlist_type(&self) -> PlaylistType {
        // For smart playlists, parse from the synthetic ID
        if self.is_smart()
            && let Some(smart_type) = PlaylistType::smart_from_id(&self.id())
        {
            return smart_type;
        }

        // For regular playlists, reconstruct from the model's properties
        PlaylistType::new_regular(self.id(), self.name(), self.child_count(), self.favorite())
    }
}

mod imp {
    use glib::Properties;
    use gtk::{glib, prelude::*, subclass::prelude::*};
    use std::cell::{Cell, RefCell};

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::PlaylistModel)]
    pub struct PlaylistModel {
        #[property(get, set)]
        id: RefCell<String>,
        #[property(get, set)]
        name: RefCell<String>,
        #[property(get, set)]
        child_count: Cell<u64>,
        #[property(get, set)]
        favorite: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlaylistModel {
        const NAME: &'static str = "GellyPlaylistModel";
        type Type = super::PlaylistModel;
        type ParentType = glib::Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for PlaylistModel {}
}
