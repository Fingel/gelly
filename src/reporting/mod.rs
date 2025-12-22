use futures::lock::Mutex;
use gtk::glib;
use log::warn;
use std::rc::Rc;

use crate::{
    audio::model::AudioModel,
    models::SongModel,
    reporting::{jellyfin::JellyfinReporter, mpris::MprisReporter},
};

pub mod jellyfin;
pub mod mpris;

#[derive(Debug, Clone)]
pub enum PlaybackEvent {
    StateChanged {
        playing: bool,
        paused: bool,
        position: u64,
        can_play: bool,
        can_pause: bool,
    },
    TrackChanged {
        song: Option<SongModel>,
        position: u64,
        can_go_next: bool,
        can_go_previous: bool,
    },
    PositionChanged {
        position: u64,
    },
    MetadataChanged {
        song: Option<SongModel>,
    },
    VolumeChanged {
        volume: f64,
    },
    NavigationChanged {
        can_go_next: bool,
        can_go_previous: bool,
        can_play: bool,
    },
    Seeked {
        position: u64,
    },
}

#[derive(Debug)]
pub struct ReportingManager {
    mpris_reporter: Rc<Mutex<MprisReporter>>,
    jellyfin_reporter: Option<Rc<Mutex<JellyfinReporter>>>,
}

impl ReportingManager {
    pub fn new(audio_model: &AudioModel) -> Self {
        let mpris_reporter = Rc::new(Mutex::new(MprisReporter::new(audio_model)));
        let jellyfin_reporter = if let Some(app) = audio_model.application() {
            Some(Rc::new(Mutex::new(JellyfinReporter::new(&app))))
        } else {
            warn!("Could not instantiate Jellyfin reporter");
            None
        };
        Self {
            mpris_reporter,
            jellyfin_reporter,
        }
    }

    pub fn report_event(&self, event: PlaybackEvent) {
        let mpris_reporter = self.mpris_reporter.clone();
        let jellyfin_reporter = self.jellyfin_reporter.clone();

        glib::spawn_future_local(async move {
            if let Err(e) = mpris_reporter
                .lock()
                .await
                .handle_event(event.clone())
                .await
            {
                warn!("MPRIS reporter failed to handle event: {}", e);
            }

            if let Some(jellyfin_reporter) = jellyfin_reporter
                && let Err(_) = jellyfin_reporter
                    .lock()
                    .await
                    .handle_event(event.clone())
                    .await
            {
                warn!("Jellyfin reporter failed to handle event");
            }
        });
    }
}
