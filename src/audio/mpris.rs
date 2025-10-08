use gtk::{gio, glib, prelude::*};
use log::error;
use mpris_server::zbus::fdo;
use mpris_server::{
    LocalPlayerInterface, LocalRootInterface, LocalServer, LoopStatus, Metadata, PlaybackStatus,
    Time, TrackId, Volume,
};

use crate::audio::model::AudioModel;
use crate::config;

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
        if let Some(app) = self.application() {
            app.activate();
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
        let playlist = self.playlist();
        let current_index = self.playlist_index();
        Ok(current_index >= 0 && (current_index + 1) < playlist.len() as i32)
    }

    async fn can_go_previous(&self) -> fdo::Result<bool> {
        Ok(self.playlist_index() > 0)
    }

    async fn can_play(&self) -> fdo::Result<bool> {
        Ok(!self.playlist().is_empty())
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

impl AudioModel {
    pub fn initialize_mpris(&self) {
        let audio_model = self.clone(); // Gobject clones are RC's by gtk so this is fine

        glib::spawn_future_local(async move {
            match LocalServer::new(config::APP_ID, audio_model).await {
                Ok(server) => {
                    let _handle = glib::spawn_future_local(async move {
                        server.run().await;
                    });
                }
                Err(e) => {
                    error!("Failed to create MPRIS server: {}", e);
                }
            }
        });
    }

    fn application(&self) -> Option<crate::application::Application> {
        gio::Application::default()
            .and_then(|app| app.downcast::<crate::application::Application>().ok())
    }
}
