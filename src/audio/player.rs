use async_channel::{Receiver, Sender};
use futures_util::StreamExt;
use gstreamer as gst;
use gstreamer::prelude::*;
use gtk::glib;
use log::warn;

#[derive(Debug, Clone)]
pub enum PlayerState {
    Stopped,
    Playing,
    Paused,
}

#[derive(Debug, Clone)]
pub enum PlayerEvent {
    StateChanged(PlayerState),
    EndOfStream,
    Error(String),
    PositionChanged(u64),
    DurationChanged(u64),
}

#[derive(Debug)]
pub struct AudioPlayer {
    pipeline: gst::Pipeline,
    playbin: gst::Element,
    event_sender: Sender<PlayerEvent>,
}

impl AudioPlayer {
    pub fn new() -> (Self, Receiver<PlayerEvent>) {
        gst::init().expect("Could not initialize gstreamer");
        let (event_sender, event_reciever) = async_channel::unbounded();

        let playbin = gst::ElementFactory::make("playbin")
            .build()
            .expect("Failed to create playbin element");

        let pipeline = gst::Pipeline::new();
        pipeline
            .add(&playbin)
            .expect("Failed to add playbin to pipeline");

        let player_instance = Self {
            pipeline,
            playbin,
            event_sender,
        };

        player_instance.setup_bus_handling();
        player_instance.setup_position_timer();

        (player_instance, event_reciever)
    }

    pub fn set_uri(&self, uri: &str) {
        self.playbin.set_property("uri", uri);
    }

    fn set_state(&self, state: gst::State) {
        match self.pipeline.set_state(state) {
            Ok(_) => (),
            Err(err) => {
                warn!("Failed to set player state: {}", err);
            }
        }
    }

    pub fn play(&self) {
        self.set_state(gst::State::Playing);
    }

    pub fn pause(&self) {
        self.set_state(gst::State::Paused);
    }

    pub fn stop(&self) {
        self.set_state(gst::State::Null);
    }

    pub fn seek(&self, position_s: u64) {
        let position = gst::ClockTime::from_seconds(position_s);
        match self
            .pipeline
            .seek_simple(gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT, position)
        {
            Ok(_) => (),
            Err(err) => {
                warn!("Failed to seek player: {}", err);
            }
        }
    }

    pub fn is_playing(&self) -> bool {
        let (state_result, current_state, _pending_state) =
            self.pipeline.state(gst::ClockTime::ZERO);
        state_result.is_ok() && current_state == gst::State::Playing
    }

    pub fn get_position(&self) -> Option<u64> {
        self.pipeline
            .query_position::<gst::ClockTime>()
            .map(|pos| pos.seconds())
    }

    pub fn get_duration(&self) -> Option<u64> {
        self.pipeline
            .query_duration::<gst::ClockTime>()
            .map(|dur| dur.seconds())
    }

    pub fn set_volume(&self, volume: f64) {
        self.playbin.set_property("volume", volume);
    }

    pub fn get_volume(&self) -> f64 {
        // linear to cubic
        let linear_volume = self.playbin.property::<f64>("volume");
        linear_volume.cbrt().clamp(0.0, 1.0)
    }

    pub fn set_mute(&self, muted: bool) {
        self.playbin.set_property("mute", muted);
    }

    pub fn is_muted(&self) -> bool {
        self.playbin.property::<bool>("mute")
    }

    fn setup_bus_handling(&self) {
        let bus = self.pipeline.bus().expect("Pipeline should have a bus");
        let sender = self.event_sender.clone();

        glib::spawn_future_local(async move {
            let mut messages = bus.stream();

            while let Some(msg) = messages.next().await {
                match msg.view() {
                    gst::MessageView::StateChanged(state_changed) => {
                        // Only handle pipeline-level state changes to avoid duplicate notifications
                        // Individual elements also emit state changes, but we only care about the overall pipeline state
                        if let Some(source) = state_changed.src()
                            && source.type_() == gst::Pipeline::static_type()
                        {
                            let new_state = state_changed.current();
                            let player_state = match new_state {
                                gst::State::Playing => PlayerState::Playing,
                                gst::State::Paused => PlayerState::Paused,
                                gst::State::Null | gst::State::Ready => PlayerState::Stopped,
                                _ => continue,
                            };
                            let _ = sender.send(PlayerEvent::StateChanged(player_state)).await;
                        }
                    }
                    gst::MessageView::Eos(_) => {
                        let _ = sender.send(PlayerEvent::EndOfStream).await;
                    }
                    gst::MessageView::Error(err) => {
                        let error_msg = format!(
                            "Gstreamer error from {:?}: {} ({})",
                            err.src().map(|s| s.path_string()),
                            err.error(),
                            err.debug().unwrap_or_else(|| "no debug info".into())
                        );
                        let _ = sender.send(PlayerEvent::Error(error_msg)).await;
                    }
                    gst::MessageView::DurationChanged(_) => {
                        // Get it via a timer instead
                    }
                    _ => {}
                }
            }
        });
    }

    fn setup_position_timer(&self) {
        let sender = self.event_sender.clone();
        let pipeline = self.pipeline.clone();

        // Update position every second while playing
        glib::timeout_add_seconds_local(1, move || {
            let (state_result, current_state, _pending_state) =
                pipeline.state(gst::ClockTime::ZERO);
            if state_result.is_ok() && current_state == gst::State::Playing {
                // Send position update
                if let Some(position) = pipeline.query_position::<gst::ClockTime>() {
                    let seconds = position.seconds();
                    glib::spawn_future_local(glib::clone!(
                        #[strong]
                        sender,
                        async move {
                            let _ = sender.send(PlayerEvent::PositionChanged(seconds)).await;
                        }
                    ));
                }

                // Send duration update if we don't have it yet
                if let Some(duration) = pipeline.query_duration::<gst::ClockTime>() {
                    let seconds = duration.seconds();
                    glib::spawn_future_local(glib::clone!(
                        #[strong]
                        sender,
                        async move {
                            let _ = sender.send(PlayerEvent::DurationChanged(seconds)).await;
                        }
                    ));
                }
            }
            glib::ControlFlow::Continue
        });
    }
}
