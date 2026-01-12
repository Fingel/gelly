use std::time::{Duration, Instant};

use adw::subclass::prelude::ObjectSubclassIsExt;
use gtk::{glib, prelude::*};
use mpris_server::zbus::fdo;
use mpris_server::{
    LocalPlayerInterface, LocalRootInterface, LoopStatus, Metadata, PlaybackStatus, Time, TrackId,
    Volume,
};

use crate::audio::model::AudioModel;
use crate::cache::ImageCache;
use crate::config;
use crate::models::SongModel;

pub async fn build_metadata(song: &SongModel) -> Metadata {
    let mut metadata = Metadata::builder()
        .artist(song.artists())
        .album(song.album())
        .title(song.title())
        .length(Time::from_secs(song.duration_seconds() as i64))
        .build();
    if let Ok(cache_dir) = ImageCache::new() {
        let art_path = cache_dir.get_cache_file_path(&song.id());

        // Poll for the file for 2 seconds because elsewhere the album art should be being fetched.
        let start_time = Instant::now();
        let timeout = Duration::from_secs(2);
        loop {
            if art_path.exists() {
                let art_url = format!("file://{}", art_path.to_string_lossy());
                metadata.set_art_url(Some(art_url));
                break;
            }
            if start_time.elapsed() >= timeout {
                break;
            }
            glib::timeout_future(Duration::from_millis(100)).await;
        }
    }
    metadata
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
        Ok(self.imp().shuffle_enabled.get())
    }

    async fn set_shuffle(&self, shuffle: bool) -> mpris_server::zbus::Result<()> {
        self.set_shuffle_enabled(shuffle);
        Ok(())
    }

    async fn metadata(&self) -> fdo::Result<Metadata> {
        Ok(if let Some(song) = self.current_song() {
            build_metadata(&song).await
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
