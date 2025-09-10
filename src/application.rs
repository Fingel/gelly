use adw::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use gtk::{gio, glib};

use crate::jellyfin::Jellyfin;

glib::wrapper! {
    pub struct Application(ObjectSubclass<imp::Application>)
    @extends gio::Application, gtk::Application, adw::Application,
    @implements gio::ActionGroup, gio::ActionMap;
}

impl Application {
    pub fn new() -> Self {
        Object::builder()
            .property("application-id", "io.m51.Gelly")
            .build()
    }

    pub fn jellyfin(&self) -> Jellyfin {
        let mut jellyfin_ref = self.imp().jellyfin.borrow_mut();

        if jellyfin_ref.is_none() {
            let host = "";
            let password = "";
            let userid = "";
            let jellyfin = Jellyfin::new(host, password, userid);
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
