use adw::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use gtk::gio::prelude::SettingsExt;
use gtk::{gio, glib};

use crate::async_utils::spawn_tokio;
use crate::config::{self, retrieve_jellyfin_api_token, settings};
use crate::jellyfin::Jellyfin;
use crate::library::Library;

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
        app.initialize_library();
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
        let host = settings().string("hostname");
        let user_id = settings().string("user-id");
        let token =
            retrieve_jellyfin_api_token(host.as_str(), user_id.as_str()).unwrap_or_default();

        let jellyfin = Jellyfin::new(host.as_str(), &token, user_id.as_str());
        self.imp().jellyfin.replace(jellyfin);
    }

    pub fn jellyfin(&self) -> Jellyfin {
        self.imp().jellyfin.borrow().clone()
    }

    pub fn initialize_library(&self) {
        let library = Library::new(
            self.imp().jellyfin.borrow().clone(),
            self.imp().library_id.borrow().clone(),
        );
        self.imp().library.replace(Some(library));
    }

    pub fn library(&self) -> Option<Library> {
        self.imp().library.borrow().clone()
    }

    pub fn refresh_library<F>(&self, f: F)
    where
        F: FnOnce() + 'static,
    {
        let library = self.library().unwrap();
        spawn_tokio(
            async move {
                library.refresh().await;
            },
            |_| f(),
        );
    }

    pub fn logout(&self) {
        self.imp().jellyfin.replace(Jellyfin::default());
        self.imp().library.replace(None);
        self.imp().library_id.replace(String::new());
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

    use crate::jellyfin::Jellyfin;
    use crate::library::Library;

    #[derive(Default)]
    pub struct Application {
        pub jellyfin: RefCell<Jellyfin>,
        pub library: RefCell<Option<Library>>,
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
