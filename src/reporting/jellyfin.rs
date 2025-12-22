use std::cell::RefCell;

use crate::{
    application::Application,
    async_utils::spawn_tokio,
    jellyfin::api::{PlaybackReport, PlaybackReportStatus},
};

use super::PlaybackEvent;
use gtk::glib::{self, object::ObjectExt};
use log::warn;
use uuid::Uuid;

#[derive(Debug)]
pub struct JellyfinReporter {
    app: glib::WeakRef<Application>,
    session_id: String,
    playback_id: RefCell<String>,
    last_song_id: RefCell<Option<String>>,
}

impl JellyfinReporter {
    pub fn new(app: &Application) -> Self {
        Self {
            app: app.downgrade(),
            session_id: Uuid::new_v4().to_string(),
            playback_id: RefCell::new(Uuid::new_v4().to_string()),
            last_song_id: RefCell::new(None),
        }
    }

    pub async fn handle_event(&mut self, event: PlaybackEvent) -> Result<(), ()> {
        match event {
            PlaybackEvent::StateChanged {
                playing,
                paused,
                position,
                ..
            } => {
                // these events fire off a few times at track transitions, we only want those
                // that happen mid-listen here
                if position > 0 {
                    let item_id = self.last_song_id.borrow().clone().unwrap_or_default();
                    let position_ticks = position * 10_000_000;
                    if paused {
                        let pause_report =
                            self.new_report(item_id.clone(), false, true, position_ticks);
                        self.report(pause_report, PlaybackReportStatus::Stopped);
                    } else if playing {
                        let play_report =
                            self.new_report(item_id.clone(), true, false, position_ticks);
                        self.report(play_report, PlaybackReportStatus::Started);
                    }
                }
            }
            PlaybackEvent::TrackChanged {
                song: Some(song), ..
            } => {
                let song_id = song.id();
                let mut last_song_id = self.last_song_id.borrow_mut();
                if last_song_id.as_ref() != Some(&song_id) {
                    if let Some(prev_id) = last_song_id.as_ref() {
                        // New song now playing, send stopped report and change playback id
                        let stop_report = self.new_report(prev_id.clone(), false, false, 0);
                        self.report(stop_report, PlaybackReportStatus::Stopped);
                        self.playback_id.replace(Uuid::new_v4().to_string());
                    }
                    // Jellyfin start playing
                    *last_song_id = Some(song_id.clone());
                    let start_report = self.new_report(song_id, true, false, 0);
                    self.report(start_report, PlaybackReportStatus::Started);
                }
            }
            PlaybackEvent::PositionChanged { position } => {
                if position % 5 == 0 {
                    let position_ticks = position * 10_000_000;
                    let item_id = self.last_song_id.borrow().clone().unwrap_or_default();
                    let report = self.new_report(item_id, true, false, position_ticks);
                    self.report(report, PlaybackReportStatus::InProgress);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn new_report(
        &self,
        item_id: String,
        can_seek: bool,
        is_paused: bool,
        position_ticks: u64,
    ) -> PlaybackReport {
        PlaybackReport {
            item_id,
            session_id: self.session_id.clone(),
            play_session_id: self.playback_id.borrow().clone(),
            can_seek,
            is_paused,
            is_muted: false,
            position_ticks,
        }
    }

    fn report(&self, report: PlaybackReport, status: PlaybackReportStatus) {
        let Some(app) = self.app.upgrade() else {
            warn!("JellyfinReporter: Unable to access application instance");
            return;
        };
        let jellyfin = app.jellyfin();
        spawn_tokio(
            async move { jellyfin.playback_report(&report, &status).await },
            move |result| {
                if let Err(err) = result {
                    warn!("JellyfinReporter: Error reporting playback: {}", err);
                }
            },
        );
    }
}
