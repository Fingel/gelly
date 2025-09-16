use adw::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use gtk::gio::prelude::SettingsExt;
use gtk::{gio, glib};

use crate::config::{self, retrieve_jellyfin_api_token, settings};
use crate::jellyfin::Jellyfin;

glib::wrapper! {
    pub struct Application(ObjectSubclass<imp::Application>)
    @extends gio::Application, gtk::Application, adw::Application,
    @implements gio::ActionGroup, gio::ActionMap;
}

impl Application {
    pub fn new() -> Self {
        let app: Self = Object::builder()
            .property("application-id", config::APP_ID)
            .build();
        app.load_settings();
        app.initialize_jellyfin();
        app
    }

    pub fn load_settings(&self) {
        let library_id = settings().string("library-id");
        self.imp().library_id.replace(library_id.into());
    }

    pub fn setup_complete(&self) -> bool {
        let jellyfin = self.imp().jellyfin.borrow();
        jellyfin.is_authenticated() && !self.imp().library_id.borrow().is_empty()
    }

    pub fn initialize_jellyfin(&self) {
        let mut jellyfin = self.imp().jellyfin.borrow_mut();
        let host = settings().string("hostname");
        let user_id = settings().string("user-id");
        let token =
            retrieve_jellyfin_api_token(host.as_str(), user_id.as_str()).unwrap_or_default();

        *jellyfin = Jellyfin::new(host.as_str(), &token, user_id.as_str());
    }

    pub fn jellyfin(&self) -> Jellyfin {
        self.imp().jellyfin.borrow().clone()
    }

    pub fn logout(&self) {
        let mut jellyfin = self.imp().jellyfin.borrow_mut();
        *jellyfin = Jellyfin::default();
        settings()
            .set_string("library-id", "")
            .expect("Failed to clear library ID");
        config::logout();
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
    use std::rc::Rc;

    use crate::jellyfin::Jellyfin;

    #[derive(Default)]
    pub struct Application {
        pub jellyfin: Rc<RefCell<Jellyfin>>,
        pub library_id: RefCell<String>,
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
