use adw::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use gtk::gio::prelude::SettingsExt;
use gtk::prelude::ObjectExt;
use gtk::{gio, glib};

use crate::async_utils::spawn_tokio;
use crate::audio::model::AudioModel;
use crate::cache::{ImageCache, LibraryCache};
use crate::config::{self, retrieve_jellyfin_api_token, settings};
use crate::jellyfin::api::{MusicDto, MusicDtoList, PlaylistDto, PlaylistDtoList};
use crate::jellyfin::{Jellyfin, JellyfinError};
use log::{debug, error, warn};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::Ordering;

glib::wrapper! {
    pub struct Application(ObjectSubclass<imp::Application>)
    @extends gio::Application, gtk::Application, adw::Application,
    @implements gio::ActionGroup, gio::ActionMap;
}

impl Application {
    pub fn new() -> Self {
        let app: Self = Object::builder()
            .property("application-id", config::APP_ID)
            .property("flags", gio::ApplicationFlags::FLAGS_NONE)
            .build();
        app.load_settings();
        app.initialize_jellyfin();
        app.initialize_library_cache();
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

    pub fn initialize_library_cache(&self) {
        match LibraryCache::new() {
            Ok(cache) => {
                self.imp().library_cache.replace(Some(cache));
            }
            Err(err) => {
                // App can technically still function
                self.imp().library_cache.replace(None);
                error!("Failed to initialize library cache: {}", err);
            }
        }
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

    pub fn playlists(&self) -> Rc<RefCell<Vec<PlaylistDto>>> {
        self.imp().playlists.clone()
    }

    pub fn library_cache(&self) -> Option<LibraryCache> {
        self.imp().library_cache.borrow().clone()
    }

    pub fn image_cache(&self) -> Option<ImageCache> {
        self.imp().image_cache.borrow().clone()
    }

    pub fn audio_model(&self) -> Option<AudioModel> {
        self.imp().audio_model.borrow().clone()
    }

    fn handle_jellyfin_error(&self, error: JellyfinError, operation: &str) {
        match error {
            JellyfinError::AuthenticationFailed { message } => {
                log::error!("Authentication failed during {}: {}", operation, message);
                self.emit_by_name::<()>("force-logout", &[]);
            }
            _ => {
                log::error!("Failed to {}: {}", operation, error);
                self.emit_by_name::<()>("global-error", &[&format!("Failed to {}", operation)])
            }
        }
    }

    pub fn refresh_library(&self, refresh_cache: bool) {
        if !refresh_cache && let Some(cache) = self.library_cache() {
            match cache.load::<MusicDtoList>() {
                Ok(library) => {
                    let library_cnt = library.items.len() as u64;
                    self.cache_library(&library);
                    self.imp().library.replace(library.items);
                    self.emit_by_name::<()>("library-refreshed", &[&library_cnt]);
                    debug!("Loaded library from cache");
                    return;
                }
                Err(error) => {
                    log::error!("Failed to load library cache: {}. Refreshing.", error);
                }
            }
        }
        let library_id = self.imp().library_id.borrow().clone();
        let jellyfin = self.jellyfin();
        self.http_with_loading(
            async move { jellyfin.get_library(&library_id).await },
            glib::clone!(
                #[weak(rename_to=app)]
                self,
                move |result: Result<MusicDtoList, JellyfinError>| {
                    match result {
                        Ok(library) => {
                            let library_cnt = library.items.len() as u64;
                            app.cache_library(&library);
                            app.imp().library.replace(library.items);
                            app.emit_by_name::<()>("library-refreshed", &[&library_cnt]);
                        }
                        Err(err) => app.handle_jellyfin_error(err, "refresh_library"),
                    }
                },
            ),
        );
    }

    fn cache_library(&self, library: &MusicDtoList) {
        if let Some(cache) = self.library_cache()
            && let Err(e) = cache.save(library)
        {
            warn!("Failed to save library to cache: {}", e);
        }
    }

    pub fn refresh_playlists(&self, refresh_cache: bool) {
        if !refresh_cache && let Some(cache) = self.library_cache() {
            match cache.load::<PlaylistDtoList>() {
                Ok(playlists) => {
                    let playlist_cnt = playlists.items.len() as u64;
                    self.cache_playlists(&playlists);
                    self.imp().playlists.replace(playlists.items);
                    self.emit_by_name::<()>("playlists-refreshed", &[&playlist_cnt]);
                    debug!("Loaded playlists from cache");
                    return;
                }
                Err(error) => {
                    log::error!("Failed to load playlist cache: {}. Refreshing.", error);
                }
            }
        }
        let jellyfin = self.jellyfin();
        self.http_with_loading(
            async move { jellyfin.get_playlists().await },
            glib::clone!(
                #[weak(rename_to=app)]
                self,
                move |result: Result<PlaylistDtoList, JellyfinError>| {
                    match result {
                        Ok(playlists) => {
                            let playlist_cnt = playlists.items.len() as u64;
                            app.cache_playlists(&playlists);
                            app.imp().playlists.replace(playlists.items);
                            app.emit_by_name::<()>("playlists-refreshed", &[&playlist_cnt]);
                        }
                        Err(err) => app.handle_jellyfin_error(err, "refresh_playlists"),
                    }
                }
            ),
        )
    }

    fn cache_playlists(&self, playlists: &PlaylistDtoList) {
        if let Some(cache) = self.library_cache()
            && let Err(e) = cache.save(playlists)
        {
            warn!("Failed to save playlists to cache: {}", e);
        }
    }

    pub fn clear_cache(&self) {
        if let Some(cache) = self.library_cache()
            && let Err(e) = cache.clear()
        {
            warn!("Failed to clear cache: {}", e);
        }
    }

    pub fn refresh_all(&self, refresh_cache: bool) {
        self.refresh_library(refresh_cache);
        self.refresh_playlists(refresh_cache);
    }

    pub fn request_library_rescan(&self) {
        let library_id = self.imp().library_id.borrow().clone();
        let jellyfin = self.jellyfin();
        spawn_tokio(
            async move { jellyfin.request_library_rescan(&library_id).await },
            glib::clone!(
                #[weak(rename_to=app)]
                self,
                move |result: Result<(), JellyfinError>| {
                    match result {
                        Ok(()) => app.emit_by_name::<()>("library-rescan-requested", &[]),
                        Err(err) => app.handle_jellyfin_error(err, "request_library_rescan"),
                    }
                }
            ),
        )
    }

    pub fn logout(&self) {
        let jellyfin = Jellyfin::default();
        self.clear_cache();
        self.imp().jellyfin.replace(jellyfin);
        self.imp().library.replace(Vec::new());
        self.imp().library_id.replace(String::new());
        config::logout();
    }

    /// Emit signals when HTTP requests start, and when all are complete.
    /// Mainly used in window.rs to display the loading pulse.
    pub fn http_with_loading<F, T, E>(
        &self,
        operation: F,
        callback: impl Fn(Result<T, E>) + 'static,
    ) where
        F: Future<Output = Result<T, E>> + Send + 'static,
        T: Send + 'static,
        E: Send + 'static,
    {
        let previous_count = self
            .imp()
            .http_request_count
            .fetch_add(1, Ordering::Relaxed);
        if previous_count == 0 {
            // Only emit signal on first concurrent request
            self.emit_by_name::<()>("http-request-start", &[]);
        }
        spawn_tokio(
            operation,
            glib::clone!(
                #[weak (rename_to = app)]
                self,
                move |result| {
                    let previous_count =
                        app.imp().http_request_count.fetch_sub(1, Ordering::Relaxed);
                    if previous_count == 1 {
                        // Only emit signal when all requests are complete
                        app.emit_by_name::<()>("http-request-end", &[]);
                    }
                    callback(result);
                }
            ),
        );
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
    use std::sync::atomic::AtomicU32;

    use crate::audio::model::AudioModel;
    use crate::cache::{ImageCache, LibraryCache};
    use crate::jellyfin::Jellyfin;
    use crate::jellyfin::api::{MusicDto, PlaylistDto};

    #[derive(Default)]
    pub struct Application {
        pub jellyfin: RefCell<Jellyfin>,
        pub library: Rc<RefCell<Vec<MusicDto>>>,
        pub playlists: Rc<RefCell<Vec<PlaylistDto>>>,
        pub library_id: RefCell<String>,
        pub library_cache: RefCell<Option<LibraryCache>>,
        pub image_cache: RefCell<Option<ImageCache>>,
        pub audio_model: RefCell<Option<AudioModel>>,
        pub http_request_count: AtomicU32,
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
                    Signal::builder("library-refreshed")
                        .param_types([u64::static_type()])
                        .build(),
                    Signal::builder("playlists-refreshed")
                        .param_types([u64::static_type()])
                        .build(),
                    Signal::builder("library-rescan-requested").build(),
                    Signal::builder("force-logout").build(),
                    Signal::builder("global-error")
                        .param_types([String::static_type()])
                        .build(),
                    Signal::builder("http-request-start").build(),
                    Signal::builder("http-request-end").build(),
                ]
            })
        }
    }
    impl ApplicationImpl for Application {}
    impl GtkApplicationImpl for Application {}
    impl AdwApplicationImpl for Application {}
}
