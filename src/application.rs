use adw::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use gtk::gio::prelude::SettingsExt;
use gtk::{gio, glib};

use crate::config::{self, settings};
use crate::jellyfin::Jellyfin;

glib::wrapper! {
    pub struct Application(ObjectSubclass<imp::Application>)
    @extends gio::Application, gtk::Application, adw::Application,
    @implements gio::ActionGroup, gio::ActionMap;
}

impl Application {
    pub fn new() -> Self {
        Object::builder()
            .property("application-id", config::APP_ID)
            .build()
    }

    pub fn jellyfin(&self) -> Jellyfin {
        let mut jellyfin_ref = self.imp().jellyfin.borrow_mut();

        if jellyfin_ref.is_none() {
            let host = settings().string("hostname");
            let user_id = settings().string("user-id");
            let token = "";
            let jellyfin = Jellyfin::new(host.as_str(), token, user_id.as_str());
            *jellyfin_ref = Some(jellyfin);
        }

        jellyfin_ref.as_ref().unwrap().clone()
    }
}

impl Default for Application {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use adw::subclass::prelude::*;
    use gtk::glib;
    use std::cell::RefCell;

    use crate::jellyfin::Jellyfin;

    #[derive(Default)]
    pub struct Application {
        pub jellyfin: RefCell<Option<Jellyfin>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Application {
        const NAME: &'static str = "GellyApplication";
        type Type = super::Application;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for Application {}
    impl ApplicationImpl for Application {}
    impl GtkApplicationImpl for Application {}
    impl AdwApplicationImpl for Application {}
}
