use adw::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use gtk::gio::prelude::*;
use gtk::prelude::{GtkApplicationExt, ObjectExt};
use gtk::{gio, glib};

use crate::async_utils::spawn_tokio;
use crate::audio::model::AudioModel;
use crate::backend::Backend;
use crate::backend::BackendError;
use crate::cache::{CacheError, Cacheable, ImageCache, LibraryCache, MediaCache};
use crate::cli::add_cli_options;
use crate::config::{
    self, BackendType, retrieve_jellyfin_api_token, retrieve_subsonic_password, settings,
};
use crate::i18n::tr;
use crate::jellyfin::Jellyfin;
use crate::jellyfin::api::{FavoriteDtoList, MusicDto, MusicDtoList, Playlist};
use crate::library::Library;
use crate::subsonic::Subsonic;
use log::{debug, error, warn};
use std::sync::Arc;
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
            .property("flags", gio::ApplicationFlags::HANDLES_COMMAND_LINE)
            .build();
        app.load_settings();
        app.initialize_backend();
        app.initialize_library_cache();
        app.initialize_image_cache();
        app.initialize_media_cache();
        app.initialize_audio_model();
        app.register_playback_actions();
        app.initialize_cli();
        app
    }

    pub fn load_settings(&self) {
        let library_id = settings().string("library-id");
        self.imp().library_id.replace(library_id.into());
    }

    pub fn setup_complete(&self) -> bool {
        let backend = self.imp().backend.borrow();
        backend.is_authenticated() && !self.imp().library_id.borrow().is_empty()
    }

    pub fn initialize_backend(&self) {
        let host = settings().string("hostname");

        let backend = match config::get_backend_type() {
            BackendType::Jellyfin => {
                let user_id = settings().string("user-id");
                let token = retrieve_jellyfin_api_token(host.as_str(), user_id.as_str())
                    .unwrap_or_default();
                Backend::Jellyfin(Jellyfin::new(host.as_str(), &token, user_id.as_str()))
            }
            BackendType::Subsonic => {
                let username = settings().string("subsonic-username");
                let password = retrieve_subsonic_password(host.as_str(), username.as_str())
                    .unwrap_or_default();
                Backend::Subsonic(Subsonic::new(host.as_str(), username.as_str(), &password))
            }
        };

        self.imp().backend.replace(backend);
    }

    // TODO: simplify like media cache
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

    // TODO: simplify like media cache
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

    pub fn initialize_media_cache(&self) {
        let Some(cache) = MediaCache::new() else {
            return;
        };
        let cache = Arc::new(cache);
        self.imp().media_cache.replace(Some(Arc::clone(&cache)));
        spawn_tokio(
            async move { cache.ensure_initialized().await },
            glib::clone!(
                #[weak(rename_to = app)]
                self,
                move |result: Result<(), CacheError>| {
                    match result {
                        Ok(()) => {
                            app.emit_by_name::<()>("media-cache-initialized", &[]);
                        }
                        Err(err) => {
                            app.emit_by_name::<()>("global-error", &[&err.to_string()]);
                        }
                    }
                }
            ),
        );
        config::settings().connect_changed(
            Some("offline"),
            glib::clone!(
                #[weak(rename_to = app)]
                self,
                move |_, _| {
                    app.offline_changed();
                }
            ),
        );
    }

    fn offline_changed(&self) {
        self.refresh_all(false);
    }

    pub fn initialize_cli(&self) {
        add_cli_options(self);
    }

    pub fn initialize_audio_model(&self) {
        let audio_model = AudioModel::new();

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
                        &[&tr("Playback error: {}").replace("{}", &error)],
                    );
                }
            ),
        );

        audio_model.connect_closure(
            "play",
            false,
            glib::closure_local!(
                #[weak(rename_to = app)]
                self,
                move |_audio_model: AudioModel| {
                    app.update_inhibit(true);
                }
            ),
        );

        audio_model.connect_closure(
            "pause",
            false,
            glib::closure_local!(
                #[weak(rename_to = app)]
                self,
                move |_audio_model: AudioModel| {
                    app.update_inhibit(false);
                }
            ),
        );

        audio_model.connect_closure(
            "stop",
            false,
            glib::closure_local!(
                #[weak(rename_to = app)]
                self,
                move |_audio_model: AudioModel| {
                    app.update_inhibit(false);
                }
            ),
        );

        self.imp().audio_model.replace(Some(audio_model));
    }

    fn register_playback_actions(&self) {
        type AudioModelAction = (&'static str, fn(&AudioModel));
        let actions: [AudioModelAction; 5] = [
            ("play-pause", AudioModel::toggle_play_pause),
            ("previous", AudioModel::prev),
            ("next", AudioModel::next),
            ("seek-backward", AudioModel::seek_backward),
            ("seek-forward", AudioModel::seek_forward),
        ];
        for (name, callback) in actions {
            let action = gio::SimpleAction::new(name, None);
            action.connect_activate(glib::clone!(
                #[weak(rename_to = app)]
                self,
                move |_, _| {
                    if let Some(audio_model) = app.audio_model() {
                        callback(&audio_model);
                    }
                }
            ));
            self.add_action(&action);
        }
    }

    pub fn backend(&self) -> Backend {
        self.imp().backend.borrow().clone()
    }

    pub fn library(&self) -> Library {
        self.imp().library.clone()
    }

    pub fn library_cache(&self) -> Option<LibraryCache> {
        self.imp().library_cache.borrow().clone()
    }

    pub fn image_cache(&self) -> Option<ImageCache> {
        self.imp().image_cache.borrow().clone()
    }

    pub fn media_cache(&self) -> Option<Arc<MediaCache>> {
        self.imp().media_cache.borrow().clone()
    }

    pub fn set_image_cache_scale(&self, scale: f32) {
        if let Some(image_cache) = self.image_cache() {
            image_cache.set_scale(scale);
        }
    }

    pub fn audio_model(&self) -> Option<AudioModel> {
        self.imp().audio_model.borrow().clone()
    }

    pub fn playback_uri(&self, song_id: &str) -> String {
        if let Some(cache) = self.media_cache()
            && let Some(path) = cache.media_path(song_id)
        {
            debug!("Locating {song_id} from media cache");
            gio::File::for_path(path).uri().into()
        } else {
            debug!("Locating {song_id} via backend");
            self.backend().get_stream_uri(song_id)
        }
    }

    fn handle_backend_error(&self, error: BackendError, operation: &str) {
        match error {
            BackendError::AuthenticationFailed { message } => {
                log::error!("Authentication failed during {}: {}", operation, message);
                self.emit_by_name::<()>("force-logout", &[]);
            }
            _ => {
                log::error!("Failed to {}: {}", operation, error);
                self.emit_by_name::<()>("global-error", &[&format!("Failed to {}", operation)])
            }
        }
    }

    fn refresh_library_offline(&self) {
        let library_cache = self.library_cache();
        let media_cache = self.media_cache();

        spawn_tokio(
            async move {
                let library_cache = library_cache.ok_or(CacheError::Uninitialized)?;
                let media_cache = media_cache.ok_or(CacheError::Uninitialized)?;
                let cached_ids = media_cache.cached_item_ids().await?;
                let library = library_cache.load::<MusicDtoList>()?;

                Ok::<_, CacheError>(
                    library
                        .items
                        .into_iter()
                        .filter(|item| cached_ids.contains(&item.id))
                        .collect::<Vec<_>>(),
                )
            },
            glib::clone!(
                #[weak(rename_to = app)]
                self,
                move |result| {
                    // in case we've gone online in the meantime
                    if !config::get_offline() {
                        return;
                    }

                    match result {
                        Ok(items) => {
                            app.apply_library(items);
                            debug!("Loaded offline library via cache");
                        }
                        Err(err) => {
                            warn!("Failed to load library or media cache: {}", err);
                            app.emit_by_name::<()>(
                                "global-error",
                                &[&format!("Failed to load library: {}", err)],
                            );
                        }
                    }
                }
            ),
        );
    }

    fn refresh_library_online(&self, refresh_cache: bool) {
        if !refresh_cache {
            match self.load_cached_library() {
                Ok(library) => {
                    self.apply_library(library.items);
                    debug!("Loaded library from cache");
                    return;
                }
                Err(error) => {
                    warn!("Failed to load library cache: {}. Refreshing.", error);
                }
            }
        }
        let backend = self.backend();
        if !backend.is_authenticated() {
            debug!("Not authenticated, skipping library refresh");
            return;
        }
        let library_id = self.imp().library_id.borrow().clone();
        self.http_with_loading(
            async move { backend.get_library(&library_id).await },
            glib::clone!(
                #[weak(rename_to=app)]
                self,
                move |result: Result<MusicDtoList, BackendError>| match result {
                    Ok(library) => {
                        app.cache_collection(&library);
                        app.apply_library(library.items);
                    }
                    Err(err) => app.handle_backend_error(err, "refresh_library"),
                },
            ),
        );
    }

    fn load_cached_library(&self) -> Result<MusicDtoList, CacheError> {
        let cache = self.library_cache().ok_or(CacheError::Uninitialized)?;
        // TODO: get this off the main thread, large libraries may lock up
        cache.load::<MusicDtoList>()
    }

    fn apply_library(&self, items: Vec<MusicDto>) {
        let count = items.len() as u64;
        self.imp().library.update_songs(items);
        self.emit_by_name::<()>("library-refreshed", &[&count]);
    }

    pub fn refresh_library(&self, refresh_cache: bool) {
        self.emit_by_name::<()>("library-refresh-requested", &[]);
        if config::get_offline() {
            self.refresh_library_offline();
        } else {
            self.refresh_library_online(refresh_cache);
        }
    }

    pub fn refresh_favorites(&self, refresh_cache: bool) {
        if !refresh_cache && let Some(cache) = self.library_cache() {
            match cache.load::<FavoriteDtoList>() {
                Ok(favorites) => {
                    self.imp().library.update_favorites(&favorites.items);
                    self.emit_by_name::<()>("favorites-updated", &[]);
                    debug!("Loaded favorites from cache");
                }
                Err(error) => warn!("Failed to load favorites cache: {}. Refreshing.", error),
            }
        }
        let backend = self.backend();
        if !backend.is_authenticated() {
            debug!("Not authenticated, skipping favorites refresh");
            return;
        }
        self.http_with_loading(
            async move { backend.get_favorites().await },
            glib::clone!(
                #[weak(rename_to=app)]
                self,
                move |result: Result<FavoriteDtoList, BackendError>| {
                    match result {
                        Ok(favorites) => {
                            app.cache_collection(&favorites);
                            app.imp().library.update_favorites(&favorites.items);
                            app.emit_by_name::<()>("favorites-updated", &[]);
                        }
                        Err(err) => app.handle_backend_error(err, "refresh_favorites"),
                    }
                },
            ),
        );
    }

    fn apply_playlists(&self, playlists: Vec<Playlist>) {
        let playlist_cnt = playlists.len() as u64;
        self.library().update_playlists(playlists);
        self.emit_by_name::<()>("playlists-refreshed", &[&playlist_cnt]);
    }

    fn load_cached_playlists(&self) -> Result<Vec<Playlist>, CacheError> {
        let cache = self.library_cache().ok_or(CacheError::Uninitialized)?;
        cache.load::<Vec<Playlist>>()
    }

    fn refresh_playlists_offline(&self) {
        match self.load_cached_playlists() {
            Ok(playlists) => {
                // This is where we prune the playlists down to what is available offline
                self.apply_playlists(playlists);
                debug!("Loaded offline playlists from cache");
            }
            Err(err) => {
                warn!("Failed to load cached playlists: {}", err);
                self.emit_by_name::<()>(
                    "global-error",
                    &[&format!("Failed to load cached playlists: {err}")],
                )
            }
        }
    }

    fn refresh_playlists_online(&self, refresh_cache: bool) {
        if !refresh_cache {
            match self.load_cached_playlists() {
                Ok(playlists) => {
                    self.apply_playlists(playlists);
                    debug!("Loaded playlists from cache");
                    return;
                }
                Err(error) => {
                    warn!("Failed to load playlist cache: {}. Refreshing", error);
                }
            }
        }
        let backend = self.backend();
        if !backend.is_authenticated() {
            debug!("Not authenticated, skipping playlist refresh");
            return;
        }
        self.http_with_loading(
            async move { backend.get_playlists().await },
            glib::clone!(
                #[weak(rename_to=app)]
                self,
                move |result: Result<Vec<Playlist>, BackendError>| {
                    match result {
                        Ok(playlists) => {
                            app.cache_collection(&playlists);
                            app.apply_playlists(playlists);
                        }
                        Err(err) => app.handle_backend_error(err, "refresh_playlists"),
                    }
                }
            ),
        );
    }

    pub fn refresh_playlists(&self, refresh_cache: bool) {
        if config::get_offline() {
            self.refresh_playlists_offline();
        } else {
            self.refresh_playlists_online(refresh_cache);
        }
    }

    pub fn download_media(&self, item_id: &str) {
        let Some(cache) = self.media_cache() else {
            log::error!("Failed to get media cache");
            return;
        };
        let backend = self.backend();
        let item_id = item_id.to_string();
        self.http_with_loading(
            async move {
                cache.download_item(&item_id, &backend).await?;
                Ok::<String, CacheError>(item_id)
            },
            glib::clone!(
                #[weak(rename_to=app)]
                self,
                move |result: Result<String, CacheError>| {
                    match result {
                        Ok(item_id) => app.emit_by_name::<()>("media-downloaded", &[&item_id]),
                        Err(err) => {
                            log::error!("Failed to download media: {}", err);
                            app.emit_by_name::<()>(
                                "global-error",
                                &[&tr("Failed to download media: {}")
                                    .replace("{}", &err.to_string())],
                            )
                        }
                    }
                }
            ),
        );
    }

    pub fn refresh_playlist(&self, id: &str, notify_views: bool) {
        let backend = self.backend();
        if !backend.is_authenticated() {
            debug!("Not authenticated, skipping refresh");
            return;
        }
        let playlist_id = id.to_string();
        self.http_with_loading(
            async move { backend.get_playlist(&playlist_id).await },
            glib::clone!(
                #[weak(rename_to=app)]
                self,
                move |result: Result<Playlist, BackendError>| {
                    match result {
                        Ok(playlist) => {
                            app.library().update_playlist(playlist);
                            app.cache_collection(&app.library().playlists());
                            if notify_views {
                                let playlist_count = app.library().playlist_count();
                                app.emit_by_name::<()>("playlists-refreshed", &[&playlist_count]);
                            }
                        }
                        Err(error) => {
                            app.handle_backend_error(error, "refresh_playlist");
                        }
                    }
                }
            ),
        )
    }

    fn cache_collection<T: Cacheable>(&self, collection: &T) {
        if let Some(cache) = self.library_cache()
            && let Err(e) = cache.save(collection)
        {
            warn!("Failed to save collection to cache: {}", e);
        }
    }

    pub fn clear_cache(&self) {
        if let Some(cache) = self.library_cache()
            && let Err(e) = cache.clear()
        {
            warn!("Failed to clear cache: {}", e);
        }
    }

    pub fn set_library_id(&self, library_id: &str) {
        self.imp().library_id.replace(library_id.to_string());
        self.refresh_all(true);
        settings()
            .set_string("library-id", library_id)
            .expect("Failed to save library id");
    }

    pub fn refresh_all(&self, refresh_cache: bool) {
        self.refresh_favorites(refresh_cache);
        self.refresh_library(refresh_cache);
        self.refresh_playlists(refresh_cache);
    }

    pub fn request_library_rescan(&self) {
        let library_id = self.imp().library_id.borrow().clone();
        let backend = self.backend();
        spawn_tokio(
            async move { backend.request_library_rescan(&library_id).await },
            glib::clone!(
                #[weak(rename_to=app)]
                self,
                move |result: Result<(), BackendError>| {
                    match result {
                        Ok(()) => app.emit_by_name::<()>("library-rescan-requested", &[]),
                        Err(err) => app.handle_backend_error(err, "request_library_rescan"),
                    }
                }
            ),
        )
    }

    fn update_inhibit(&self, playing: bool) {
        if playing && config::get_inhibit_suspend_enabled() {
            if self.imp().inhibit_cookie.get() == 0 {
                let window = self.active_window();
                let cookie = self.inhibit(
                    window.as_ref(),
                    gtk::ApplicationInhibitFlags::SUSPEND,
                    Some("Playing media"),
                );
                self.imp().inhibit_cookie.set(cookie);
            }
        } else {
            let cookie = self.imp().inhibit_cookie.get();
            if cookie != 0 {
                self.uninhibit(cookie);
                self.imp().inhibit_cookie.set(0);
            }
        }
    }

    pub fn logout(&self) {
        let backend = Backend::default();
        self.clear_cache();
        self.imp().backend.replace(backend);
        self.imp().library.update_songs(Vec::new());
        self.imp().library_id.replace(String::new());
        config::logout().unwrap_or_else(|e| warn!("Failed to clear config on logout: {}", e));
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
    use std::cell::Cell;
    use std::cell::RefCell;
    use std::sync::atomic::AtomicU32;
    use std::sync::{Arc, OnceLock};

    use crate::audio::model::AudioModel;
    use crate::backend::Backend;
    use crate::cache::MediaCache;
    use crate::cache::{ImageCache, LibraryCache};
    use crate::library::Library;

    #[derive(Default)]
    pub struct Application {
        pub backend: RefCell<Backend>,
        pub library: Library,
        pub library_id: RefCell<String>,
        pub library_cache: RefCell<Option<LibraryCache>>, // TODO: remove these Option<> types
        pub image_cache: RefCell<Option<ImageCache>>,
        pub media_cache: RefCell<Option<Arc<MediaCache>>>,
        pub audio_model: RefCell<Option<AudioModel>>,
        pub http_request_count: AtomicU32,
        pub inhibit_cookie: Cell<u32>,
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
                    Signal::builder("library-refresh-requested").build(),
                    Signal::builder("library-refreshed")
                        .param_types([u64::static_type()])
                        .build(),
                    Signal::builder("playlists-refreshed")
                        .param_types([u64::static_type()])
                        .build(),
                    Signal::builder("favorites-updated").build(),
                    Signal::builder("library-rescan-requested").build(),
                    Signal::builder("force-logout").build(),
                    Signal::builder("global-error")
                        .param_types([String::static_type()])
                        .build(),
                    Signal::builder("http-request-start").build(),
                    Signal::builder("http-request-end").build(),
                    Signal::builder("big-player-requested").build(),
                    Signal::builder("media-downloaded")
                        .param_types([String::static_type()])
                        .build(),
                    Signal::builder("media-cache-initialized").build(),
                ]
            })
        }
    }
    impl ApplicationImpl for Application {}
    impl GtkApplicationImpl for Application {}
    impl AdwApplicationImpl for Application {}
}
