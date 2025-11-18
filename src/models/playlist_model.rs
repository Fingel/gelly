use crate::{
    jellyfin::api::PlaylistDto,
    models::{PlaylistType, playlist_type::DEFAULT_SMART_COUNT},
};
use glib::Object;
use gtk::glib;

glib::wrapper! {
    pub struct PlaylistModel(ObjectSubclass<imp::PlaylistModel>);
}

impl PlaylistModel {
    pub fn new_regular(id: &str, name: &str, child_count: u64) -> Self {
        Object::builder()
            .property("id", id)
            .property("name", name)
            .property("child_count", child_count)
            .build()
    }

    pub fn new_smart(playlist_type: PlaylistType) -> Self {
        Object::builder()
            .property("id", playlist_type.to_id().unwrap_or_default())
            .property("name", playlist_type.display_name())
            .property(
                "child_count",
                playlist_type
                    .estimated_count()
                    .unwrap_or(DEFAULT_SMART_COUNT),
            )
            .build()
    }

    pub fn is_smart(&self) -> bool {
        !matches!(self.playlist_type(), PlaylistType::Regular)
    }

    pub fn playlist_type(&self) -> PlaylistType {
        PlaylistType::from_id(&self.id())
    }
}

impl From<&PlaylistDto> for PlaylistModel {
    fn from(dto: &PlaylistDto) -> Self {
        Self::new_regular(&dto.id, &dto.name, dto.child_count)
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
