use gtk::glib;
use log::warn;
use mpris_server::zbus::{self, fdo};
use mpris_server::{LocalServer, Metadata, PlaybackStatus, Property, Signal, Time};
use thiserror::Error;

use crate::audio::model::AudioModel;
use crate::cache::ImageCache;
use crate::config::APP_ID;
use crate::models::SongModel;

use super::PlaybackEvent;

#[derive(Error, Debug)]
pub enum MprisError {
    #[error("Zbus error: {0}")]
    Zbus(#[from] zbus::Error),

    #[error("FDO error: {0}")]
    Fdo(#[from] fdo::Error),
}

type Result<T> = std::result::Result<T, MprisError>;

#[derive(Debug)]
pub struct MprisReporter {
    server: Option<GellyMprisServer>,
}

type GellyMprisServer = LocalServer<AudioModel>;

impl MprisReporter {
    pub fn new(audio_model: &AudioModel) -> Self {
        let mut reporter = Self { server: None };

        // Initialize immediately since we have the AudioModel reference here
        if let Err(e) =
            glib::MainContext::default().block_on(reporter.initialize_with_model(audio_model))
        {
            warn!("Failed to initialize MPRIS server: {}", e);
        }

        reporter
    }

    async fn initialize_with_model(&mut self, audio_model: &AudioModel) -> Result<()> {
        let server: GellyMprisServer = LocalServer::new(APP_ID, audio_model.clone()).await?;
        glib::spawn_future_local(server.run());
        self.server = Some(server);

        Ok(())
    }

    async fn emit_properties_changed(
        &self,
        properties: impl IntoIterator<Item = Property>,
    ) -> Result<()> {
        if let Some(server) = &self.server {
            server.properties_changed(properties).await?;
        }

        Ok(())
    }

    async fn emit_seeked_signal(&self, position: u64) {
        if let Some(server) = &self.server {
            let signal = Signal::Seeked {
                position: Time::from_secs(position as i64),
            };
            if let Err(err) = server.emit(signal).await {
                warn!("Failed to emit MPRIS seeked signal: {}", err);
            }
        }
    }

    fn metadata(&self, song: Option<SongModel>) -> Metadata {
        if let Some(song) = song {
            let mut metadata = Metadata::builder()
                .artist(song.artists())
                .album(song.album())
                .title(song.title())
                .length(Time::from_secs(song.duration() as i64))
                .build();
            if let Ok(cache_dir) = ImageCache::new() {
                let art_path = cache_dir.get_cache_file_path(&song.id());
                if art_path.exists() {
                    let art_url = format!("file://{}", art_path.to_string_lossy());
                    metadata.set_art_url(Some(art_url));
                }
            }
            metadata
        } else {
            Metadata::new()
        }
    }

    pub async fn handle_event(&mut self, event: PlaybackEvent) -> Result<()> {
        match event {
            PlaybackEvent::StateChanged {
                playing,
                paused,
                can_play,
                can_pause,
                ..
            } => {
                let status = if playing {
                    PlaybackStatus::Playing
                } else if paused {
                    PlaybackStatus::Paused
                } else {
                    PlaybackStatus::Stopped
                };

                self.emit_properties_changed([
                    Property::PlaybackStatus(status),
                    Property::CanPause(can_pause),
                    Property::CanPlay(can_play),
                ])
                .await?;
            }

            PlaybackEvent::TrackChanged {
                song,
                can_go_next,
                can_go_previous,
                ..
            } => {
                let metadata = self.metadata(song);

                self.emit_properties_changed([
                    Property::Metadata(metadata),
                    Property::CanGoNext(can_go_next),
                    Property::CanGoPrevious(can_go_previous),
                ])
                .await?;
            }

            PlaybackEvent::MetadataChanged { song } => {
                let metadata = self.metadata(song);

                self.emit_properties_changed([Property::Metadata(metadata)])
                    .await?;
            }

            PlaybackEvent::VolumeChanged { volume } => {
                self.emit_properties_changed([Property::Volume(volume)])
                    .await?;
            }

            PlaybackEvent::NavigationChanged {
                can_go_next,
                can_go_previous,
                can_play,
            } => {
                self.emit_properties_changed([
                    Property::CanGoNext(can_go_next),
                    Property::CanGoPrevious(can_go_previous),
                    Property::CanPlay(can_play),
                ])
                .await?;
            }

            PlaybackEvent::Seeked { position } => {
                self.emit_seeked_signal(position).await;
            }
            PlaybackEvent::PositionChanged { .. } => {
                // MPRIS doesn't need position change notifications
            }
        }

        Ok(())
    }
}
