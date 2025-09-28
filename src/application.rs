use adw::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use gtk::gio::prelude::SettingsExt;
use gtk::prelude::ObjectExt;
use gtk::{gio, glib};

use crate::async_utils::spawn_tokio;
use crate::audio::model::AudioModel;
use crate::cache::ImageCache;
use crate::config::{self, retrieve_jellyfin_api_token, settings};
use crate::jellyfin::api::{MusicDto, MusicDtoList};
use crate::jellyfin::{Jellyfin, JellyfinError};
use log::error;
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
        app.initialize_image_cache();
        app.initialize_audio_model();
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

    pub fn initialize_image_cache(&self) {
        match ImageCache::new() {
            Ok(cache) => {
                self.imp().image_cache.replace(Some(cache));
            }
            Err(err) => {
                // App can technically still function
                self.imp().image_cache.replace(None);
                error!("Failed to initialize image cache: {}", err);
            }
        }
    }

    pub fn initialize_audio_model(&self) {
        let audio_model = AudioModel::new();

        audio_model.connect_closure(
            "request-stream-uri",
            false,
            glib::closure_local!(
                #[weak(rename_to = app)]
                self,
                move |_audio_model: AudioModel, song_id: &str| -> String {
                    app.jellyfin().get_stream_uri(song_id)
                }
            ),
        );

        audio_model.connect_closure(
            "error",
            false,
            glib::closure_local!(
                #[weak(rename_to = app)]
                self,
                move |_audio_model: AudioModel, error: String| {
                    log::error!("Audio error: {}", error);
                    app.emit_by_name::<()>(
                        "global-error",
                        &[&format!("Playback error: {}", error)],
                    );
                }
            ),
        );

        self.imp().audio_model.replace(Some(audio_model));
    }

    pub fn jellyfin(&self) -> Jellyfin {
        self.imp().jellyfin.borrow().clone()
    }

    pub fn library(&self) -> Rc<RefCell<Vec<MusicDto>>> {
        self.imp().library.clone()
    }

    pub fn image_cache(&self) -> Option<ImageCache> {
        self.imp().image_cache.borrow().clone()
    }

    pub fn audio_model(&self) -> Option<AudioModel> {
        self.imp().audio_model.borrow().clone()
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

    use crate::audio::model::AudioModel;
    use crate::cache::ImageCache;
    use crate::jellyfin::Jellyfin;
    use crate::jellyfin::api::MusicDto;

    #[derive(Default)]
    pub struct Application {
        pub jellyfin: RefCell<Jellyfin>,
        pub library: Rc<RefCell<Vec<MusicDto>>>,
        pub library_id: RefCell<String>,
        pub image_cache: RefCell<Option<ImageCache>>,
        pub audio_model: RefCell<Option<AudioModel>>,
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
