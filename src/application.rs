use adw::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use gtk::gio::prelude::SettingsExt;
use gtk::prelude::ObjectExt;
use gtk::{gio, glib};

use crate::async_utils::spawn_tokio;
use crate::config::{self, retrieve_jellyfin_api_token, settings};
use crate::jellyfin::api::{MusicDto, MusicDtoList};
use crate::jellyfin::{Jellyfin, JellyfinError};
use std::cell::RefCell;
use std::rc::Rc;

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

    pub fn library(&self) -> Rc<RefCell<Vec<MusicDto>>> {
        self.imp().library.clone()
    }

    pub fn refresh_library(&self) {
        let library_id = self.imp().library_id.borrow().clone();
        let jellyfin = self.jellyfin();
        spawn_tokio(
            async move { jellyfin.get_library(&library_id).await },
            glib::clone!(
                #[weak(rename_to=app)]
                self,
                move |result: Result<MusicDtoList, JellyfinError>| {
                    match result {
                        Ok(library) => {
                            app.imp().library.replace(library.items);
                            app.emit_by_name::<()>("library-refreshed", &[]);
                        }
                        Err(err) => {
                            log::error!("Failed to refresh library: {}", err);
                            app.emit_by_name::<()>(
                                "global-error",
                                &[&String::from("Failed to refresh library")],
                            )
                        }
                    }
                },
            ),
        );
    }

    pub fn logout(&self) {
        self.imp().jellyfin.replace(Jellyfin::default());
        self.imp().library.replace(Vec::new());
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
    use gtk::glib::subclass::Signal;
    use gtk::glib::types::StaticType;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::OnceLock;

    use crate::jellyfin::Jellyfin;
    use crate::jellyfin::api::MusicDto;

    #[derive(Default)]
    pub struct Application {
        pub jellyfin: RefCell<Jellyfin>,
        pub library: Rc<RefCell<Vec<MusicDto>>>,
        pub library_id: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Application {
        const NAME: &'static str = "GellyApplication";
        type Type = super::Application;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for Application {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("library-refreshed").build(),
                    Signal::builder("global-error")
                        .param_types([String::static_type()])
                        .build(),
                ]
            })
        }
    }
    impl ApplicationImpl for Application {}
    impl GtkApplicationImpl for Application {}
    impl AdwApplicationImpl for Application {}
}
