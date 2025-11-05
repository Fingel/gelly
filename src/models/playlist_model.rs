use crate::jellyfin::api::PlaylistDto;
use glib::Object;
use gtk::glib;

glib::wrapper! {
    pub struct PlaylistModel(ObjectSubclass<imp::PlaylistModel>);
}

impl PlaylistModel {
    pub fn new(id: &str, name: &str, child_count: u64) -> Self {
        Object::builder()
            .property("id", id)
            .property("name", name)
            .property("child_count", child_count)
            .build()
    }
}

impl From<&PlaylistDto> for PlaylistModel {
    fn from(dto: &PlaylistDto) -> Self {
        Self::new(&dto.id, &dto.name, dto.child_count)
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
