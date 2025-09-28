use gtk::{
    glib::{self, Object, Properties},
    prelude::*,
    subclass::prelude::*,
};
use std::cell::{Cell, RefCell};
use std::sync::OnceLock;

use crate::{
    audio::player::{AudioPlayer, PlayerEvent, PlayerState},
    jellyfin::{Jellyfin, api::MusicDto},
};

glib::wrapper! {
    pub struct AudioModel(ObjectSubclass<imp::AudioModel>);
}

impl AudioModel {
    pub fn new() -> Self {
        let obj: Self = Object::builder().build();
        obj.initialize_player();
        obj
    }

    fn initialize_player(&self) {
        let (player, event_reciever) = AudioPlayer::new();
        self.imp().player.replace(Some(player));

        let obj_weak = self.downgrade();
        glib::spawn_future_local(async move {
            while let Ok(event) = event_reciever.recv().await {
                let Some(obj) = obj_weak.upgrade() else { break };

                match event {
                    PlayerEvent::StateChanged(state) => {
                        let playing = matches!(state, PlayerState::Playing);
                        let paused = matches!(state, PlayerState::Paused);

                        obj.set_property("playing", playing);
                        obj.set_property("paused", paused);
                        obj.set_property("loading", false);

                        match state {
                            PlayerState::Playing => obj.emit_by_name::<()>("play", &[]),
                            PlayerState::Paused => obj.emit_by_name::<()>("pause", &[]),
                            PlayerState::Stopped => obj.emit_by_name::<()>("stop", &[]),
                        }
                    }
                    PlayerEvent::PositionChanged(pos) => {
                        obj.set_property("position", pos as u32);
                    }
                    PlayerEvent::DurationChanged(dur) => {
                        obj.set_property("duration", dur as u32);
                    }
                    PlayerEvent::EndOfStream => {
                        obj.emit_by_name::<()>("song-finished", &[]);
                    }
                    PlayerEvent::Error(err) => {
                        obj.set_property("loading", false);
                        obj.emit_by_name::<()>("error", &[&err]);
                    }
                }
            }
        });
    }

    pub fn play_track(&self, track: &MusicDto, jellyfin: &Jellyfin) {
        if let Some(player) = self.imp().player.borrow().as_ref() {
            let stream_url = jellyfin.get_stream_uri(&track.id);
            self.set_property("current-song-title", &track.name);
            self.set_property(
                "current-song-artist",
                track
                    .artist_items
                    .first()
                    .map(|a| a.name.as_str())
                    .unwrap_or("Unknown"),
            );
            self.set_property("current-song-album", &track.album);

            // Reset position/duration
            self.set_property("position", 0u32);
            self.set_property("duration", 0u32);
            self.set_property("loading", true);

            player.set_uri(&stream_url);
            player.play();
        }
    }

    pub fn play(&self) {
        if let Some(player) = self.imp().player.borrow().as_ref() {
            player.play();
        }
    }

    pub fn pause(&self) {
        if let Some(player) = self.imp().player.borrow().as_ref() {
            player.pause();
        }
    }

    pub fn stop(&self) {
        if let Some(player) = self.imp().player.borrow().as_ref() {
            player.stop();
            self.set_property("current-song-title", "");
            self.set_property("current-song-artist", "");
            self.set_property("current-song-album", "");
            self.set_property("position", 0u32);
            self.set_property("duration", 0u32);
        }
    }

    pub fn seek(&self, position: u32) {
        if let Some(player) = self.imp().player.borrow().as_ref() {
            player.seek(position as u64);
            self.set_property("position", position);
        }
    }

    pub fn toggle_play_pause(&self) {
        if self.playing() {
            self.pause();
        } else {
            self.play();
        }
    }
}

impl Default for AudioModel {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use crate::audio::player::AudioPlayer;

    use super::*;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::AudioModel)]
    pub struct AudioModel {
        #[property(get, set)]
        pub playing: Cell<bool>,

        #[property(get, set)]
        pub loading: Cell<bool>,

        #[property(get, set)]
        pub position: Cell<u32>,

        #[property(get, set)]
        pub duration: Cell<u32>,

        #[property(get, set)]
        pub volume: Cell<f64>,

        #[property(get, set)]
        pub muted: Cell<bool>,

        #[property(get, set, name = "current-song-title")]
        pub current_song_title: RefCell<String>,

        #[property(get, set, name = "current-song-artist")]
        pub current_song_artist: RefCell<String>,

        #[property(get, set, name = "current-song-album")]
        pub current_song_album: RefCell<String>,

        pub player: RefCell<Option<AudioPlayer>>,
    }
    #[glib::object_subclass]
    impl ObjectSubclass for AudioModel {
        const NAME: &'static str = "GellyAudioModel";
        type Type = super::AudioModel;
        type ParentType = Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for AudioModel {
        fn signals() -> &'static [glib::subclass::Signal] {
            static SIGNALS: OnceLock<Vec<glib::subclass::Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    glib::subclass::Signal::builder("play").build(),
                    glib::subclass::Signal::builder("pause").build(),
                    glib::subclass::Signal::builder("stop").build(),
                    glib::subclass::Signal::builder("song-finished").build(),
                    glib::subclass::Signal::builder("error")
                        .param_types([String::static_type()])
                        .build(),
                ]
            })
        }
    }

    impl AudioModel {
        pub fn set_volume(&self, volume: f64) {
            let clamped_volume = volume.clamp(0.0, 1.0);
            self.volume.set(clamped_volume);

            if let Some(player) = self.player.borrow().as_ref() {
                player.set_volume(clamped_volume);
            }
        }

        pub fn set_muted(&self, muted: bool) {
            self.muted.set(muted);

            if let Some(player) = self.player.borrow().as_ref() {
                player.set_mute(muted);
            }
        }
    }
}
