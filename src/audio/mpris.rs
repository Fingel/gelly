use adw::subclass::prelude::{ObjectSubclassExt, ObjectSubclassIsExt};
use gtk::{gio, glib, prelude::*};
use log::{error, info, warn};
use mpris_server::zbus::{self, fdo};
use mpris_server::{
    LocalPlayerInterface, LocalRootInterface, LocalServer, LoopStatus, Metadata, PlaybackStatus,
    Property, Time, TrackId, Volume,
};
use thiserror::Error;

use crate::audio::model::AudioModel;
use crate::cache::ImageCache;
use crate::config::{self, APP_ID};

#[derive(Error, Debug)]
pub enum MprisError {
    #[error("Zbus error: {0}")]
    Zbus(#[from] zbus::Error),

    #[error("FDO error: {0}")]
    Fdo(#[from] fdo::Error),

    #[error("Initialization error: {message}")]
    InitializationError { message: String },
}

type Result<T> = std::result::Result<T, MprisError>;

// Extend AudioModel with MPRIS functionality
impl AudioModel {
    pub async fn initialize_mpris(&self) -> Result<()> {
        let server = LocalServer::new(APP_ID, self.imp().obj().clone()).await?;
        glib::spawn_future_local(server.run());
        self.imp()
            .mpris_server
            .set(server)
            .map_err(|_| MprisError::InitializationError {
                message: "Failed to set MPRIS server".to_string(),
            })?;
        Ok(())
    }

    pub fn mpris_server(&self) -> Option<&LocalServer<AudioModel>> {
        self.imp().mpris_server.get()
    }

    pub fn mpris_properties_changed(&self, property: impl IntoIterator<Item = Property> + 'static) {
        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to=imp)]
            self,
            async move {
                match imp.mpris_server() {
                    Some(server) => {
                        if let Err(err) = server.properties_changed(property).await {
                            warn!("Failed to emit properties changed: {}", err);
                        }
                    }
                    None => {
                        info!("Failed to get MPRIS server.");
                    }
                }
            }
        ));
    }

    pub fn metadata(&self) -> Metadata {
        self.imp()
            .obj()
            .current_song()
            .map_or_else(Metadata::new, |song| {
                let cache_dir = ImageCache::new().expect("Failed to create image cache");
                let art_path = cache_dir.get_cache_file_path(&song.id());
                let art_url = format!("file://{}", art_path.to_string_lossy());
                Metadata::builder()
                    .artist(song.artists())
                    .album(song.album())
                    .title(song.title())
                    .art_url(art_url)
                    .length(Time::from_secs(song.duration() as i64))
                    .build()
            })
    }

    fn application(&self) -> Option<crate::application::Application> {
        gio::Application::default()
            .and_then(|app| app.downcast::<crate::application::Application>().ok())
    }

    pub fn notify_mpris_playback_status(&self) {
        let status = if self.playing() {
            PlaybackStatus::Playing
        } else if self.paused() {
            PlaybackStatus::Paused
        } else {
            PlaybackStatus::Stopped
        };

        self.mpris_properties_changed([
            Property::PlaybackStatus(status),
            Property::CanPause(self.playing()),
            Property::CanPlay(!self.queue().is_empty()),
        ]);
    }

    pub fn notify_mpris_track_changed(&self) {
        self.mpris_properties_changed([
            Property::Metadata(self.metadata()),
            Property::CanGoNext({
                let queue = self.queue();
                let current_index = self.queue_index();
                current_index >= 0 && (current_index + 1) < queue.len() as i32
            }),
            Property::CanGoPrevious(self.queue_index() > 0),
        ]);
    }

    pub fn notify_mpris_metadata(&self) {
        self.mpris_properties_changed([Property::Metadata(self.metadata())]);
    }

    pub fn notify_mpris_volume(&self) {
        self.mpris_properties_changed([Property::Volume(self.volume())]);
    }

    pub fn notify_mpris_seeked(&self, position_seconds: u32) {
        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to=imp)]
            self,
            async move {
                match imp.mpris_server() {
                    Some(server) => {
                        let signal = mpris_server::Signal::Seeked {
                            position: Time::from_secs(position_seconds as i64),
                        };
                        if let Err(err) = server.emit(signal).await {
                            warn!("failed to emit seeked signal: {}", err);
                        }
                    }
                    None => {
                        info!("Failed to get MPRIS server.");
                    }
                }
            }
        ));
    }

    pub fn notify_mpris_can_navigate(&self, next: bool, prev: bool) {
        self.mpris_properties_changed([Property::CanGoNext(next), Property::CanGoPrevious(prev)]);
    }
}

impl LocalRootInterface for AudioModel {
    async fn can_quit(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn quit(&self) -> fdo::Result<()> {
        if let Some(app) = self.application() {
            app.quit();
        }
        Ok(())
    }

    async fn can_raise(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn raise(&self) -> fdo::Result<()> {
        if let Some(app) = self.application()
            && let Some(window) = app.active_window()
        {
            window.present();
        }
        Ok(())
    }

    async fn can_set_fullscreen(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn fullscreen(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn set_fullscreen(&self, _fullscreen: bool) -> mpris_server::zbus::Result<()> {
        Ok(())
    }

    async fn has_track_list(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn identity(&self) -> fdo::Result<String> {
        Ok("Gelly".to_string())
    }

    async fn desktop_entry(&self) -> fdo::Result<String> {
        Ok(config::APP_ID.to_string())
    }

    async fn supported_uri_schemes(&self) -> fdo::Result<Vec<String>> {
        Ok(vec!["http".to_string(), "https".to_string()])
    }

    async fn supported_mime_types(&self) -> fdo::Result<Vec<String>> {
        Ok(vec![
            "audio/mpeg".to_string(),
            "audio/ogg".to_string(),
            "audio/flac".to_string(),
        ])
    }
}

impl LocalPlayerInterface for AudioModel {
    async fn next(&self) -> fdo::Result<()> {
        self.next();
        Ok(())
    }

    async fn previous(&self) -> fdo::Result<()> {
        self.prev();
        Ok(())
    }

    async fn pause(&self) -> fdo::Result<()> {
        self.pause();
        Ok(())
    }

    async fn play_pause(&self) -> fdo::Result<()> {
        self.toggle_play_pause();
        Ok(())
    }

    async fn stop(&self) -> fdo::Result<()> {
        self.stop();
        Ok(())
    }

    async fn play(&self) -> fdo::Result<()> {
        self.play();
        Ok(())
    }

    async fn seek(&self, offset: Time) -> fdo::Result<()> {
        let current_pos = self.position() as i64;
        let new_pos = (current_pos + offset.as_secs()).max(0) as u32;
        self.seek(new_pos);
        Ok(())
    }

    async fn set_position(&self, _track_id: TrackId, position: Time) -> fdo::Result<()> {
        self.seek(position.as_secs() as u32);
        Ok(())
    }

    async fn open_uri(&self, _uri: String) -> fdo::Result<()> {
        Err(fdo::Error::NotSupported("OpenUri not supported".into()))
    }

    async fn playback_status(&self) -> fdo::Result<PlaybackStatus> {
        Ok(if self.playing() {
            PlaybackStatus::Playing
        } else if self.paused() {
            PlaybackStatus::Paused
        } else {
            PlaybackStatus::Stopped
        })
    }

    async fn loop_status(&self) -> fdo::Result<LoopStatus> {
        Ok(LoopStatus::None) // TODO - could implement repeat modes later
    }

    async fn set_loop_status(&self, _loop_status: LoopStatus) -> mpris_server::zbus::Result<()> {
        // TODO Could be implemented to control repeat modes
        Ok(())
    }

    async fn rate(&self) -> fdo::Result<f64> {
        Ok(1.0)
    }

    async fn set_rate(&self, _rate: f64) -> mpris_server::zbus::Result<()> {
        Err(mpris_server::zbus::Error::from(fdo::Error::NotSupported(
            "SetRate not supported".into(),
        )))
    }

    async fn shuffle(&self) -> fdo::Result<bool> {
        Ok(false) // TODO - could implement shuffle later
    }

    async fn set_shuffle(&self, _shuffle: bool) -> mpris_server::zbus::Result<()> {
        Ok(())
    }

    async fn metadata(&self) -> fdo::Result<Metadata> {
        Ok(if let Some(song) = self.current_song() {
            Metadata::builder()
                .title(song.title())
                .artist(song.artists())
                .album(song.album())
                .length(Time::from_secs(self.duration() as i64))
                // Could add more fields like track_id, album_artist, etc.
                .build()
        } else {
            Metadata::new()
        })
    }

    async fn volume(&self) -> fdo::Result<Volume> {
        Ok(self.volume())
    }

    async fn set_volume(&self, volume: Volume) -> mpris_server::zbus::Result<()> {
        self.set_volume(volume);
        Ok(())
    }

    async fn position(&self) -> fdo::Result<Time> {
        Ok(Time::from_secs(self.position() as i64))
    }

    async fn minimum_rate(&self) -> fdo::Result<f64> {
        Ok(1.0)
    }

    async fn maximum_rate(&self) -> fdo::Result<f64> {
        Ok(1.0)
    }

    async fn can_go_next(&self) -> fdo::Result<bool> {
        let queue = self.queue();
        let current_index = self.queue_index();
        Ok(current_index >= 0 && (current_index + 1) < queue.len() as i32)
    }

    async fn can_go_previous(&self) -> fdo::Result<bool> {
        Ok(self.queue_index() > 0)
    }

    async fn can_play(&self) -> fdo::Result<bool> {
        Ok(!self.queue().is_empty())
    }

    async fn can_pause(&self) -> fdo::Result<bool> {
        Ok(self.playing())
    }

    async fn can_seek(&self) -> fdo::Result<bool> {
        Ok(self.duration() > 0)
    }

    async fn can_control(&self) -> fdo::Result<bool> {
        Ok(true)
    }
}
