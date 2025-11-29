use gtk::{
    glib::{self, Object, Properties},
    prelude::*,
    subclass::prelude::*,
};
use log::warn;
use rand::Rng;
use std::cell::{Cell, RefCell};
use std::sync::OnceLock;

use crate::{
    audio::player::{AudioPlayer, PlayerEvent, PlayerState},
    models::SongModel,
};

glib::wrapper! {
    pub struct AudioModel(ObjectSubclass<imp::AudioModel>);
}

struct QueueStats {
    total: u32,
    unplayed: u32,
}

impl AudioModel {
    pub fn new() -> Self {
        let obj: Self = Object::builder().build();
        obj.initialize_player();
        obj.set_queue_index(-1);
        // TODO set these from settings
        obj.set_volume(1.0);
        obj.set_muted(false);
        obj
    }

    fn initialize_player(&self) {
        let (player, event_reciever) = AudioPlayer::new();
        self.imp()
            .player
            .set(player)
            .expect("Player should only be initialized once");

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

                        // Mpris notification
                        obj.notify_mpris_playback_status();

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
                        // Update MPRIS metadata
                        obj.notify_mpris_metadata();
                    }
                    PlayerEvent::EndOfStream => {
                        obj.next();
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

    fn player(&self) -> &AudioPlayer {
        self.imp()
            .player
            .get()
            .expect("Player should be initialized")
    }

    fn stream_uri(&self, song_id: &str) -> String {
        let uri: String = self.emit_by_name("request-stream-uri", &[&song_id]);
        if uri.is_empty() {
            self.emit_by_name::<()>("error", &[&"Failed to get stream URI".to_string()]);
            return String::new();
        }
        uri
    }

    pub fn queue(&self) -> Vec<SongModel> {
        self.imp().queue.borrow().clone()
    }

    pub fn set_queue(&self, songs: Vec<SongModel>, start_index: usize) {
        let len = songs.len() as u32;
        self.imp().queue.replace(songs);
        self.load_song(start_index as i32);
        self.play();
        self.notify_mpris_can_navigate(true, start_index > 0);
        if self.imp().shuffle_enabled.get() {
            self.new_shuffle_cycle();
        }
        self.emit_by_name("queue-changed", &[&len, &len])
    }

    fn queue_stats(&self) -> QueueStats {
        // Need to implement unplayed
        let total = self.queue().len() as u32;
        QueueStats {
            total,
            unplayed: total,
        }
    }

    pub fn append_to_queue(&self, songs: Vec<SongModel>) {
        self.imp().queue.borrow_mut().extend(songs);
        let current_index = self.queue_index();
        self.notify_mpris_can_navigate(true, current_index > 0);
        if self.imp().shuffle_enabled.get() {
            self.new_shuffle_cycle();
        }
        let stats = self.queue_stats();
        self.emit_by_name("queue-changed", &[&stats.total, &stats.unplayed])
    }

    pub fn clear_queue(&self) {
        self.imp().queue.replace(Vec::new());
        self.notify_mpris_can_navigate(false, false);
        self.emit_by_name("queue-changed", &[&0u32, &0u32])
    }

    pub fn play_song(&self, index: usize) {
        self.load_song(index as i32);
        self.play();
    }

    fn load_song(&self, index: i32) {
        if let Some(song) = self.imp().queue.borrow().get(index as usize).cloned() {
            let stream_uri = self.stream_uri(&song.id());
            let player = self.player();
            player.stop();
            self.set_property("position", 0u32);
            self.set_property("duration", 0u32);
            self.set_property("loading", true);
            self.set_queue_index(index);
            player.set_uri(&stream_uri);
            self.emit_by_name::<()>("song-changed", &[&song.id()]);
            // Notify MPRIS with metadata
            self.notify_mpris_track_changed();
        } else {
            self.stop();
            warn!("Failed to load song at index {}", index);
        }
    }

    pub fn next(&self) {
        if self.imp().shuffle_enabled.get() {
            self.next_shuffled();
        } else {
            self.next_linear();
        }
    }

    pub fn prev(&self) {
        if self.imp().shuffle_enabled.get() {
            self.prev_shuffled();
        } else {
            self.prev_linear();
        }
    }

    fn next_linear(&self) {
        let next_index = self.queue_index() + 1;
        if next_index < self.imp().queue.borrow().len() as i32 {
            self.load_song(next_index);
            self.play();
        } else {
            self.stop();
            self.emit_by_name::<()>("queue-finished", &[]);
        }
    }

    fn prev_linear(&self) {
        let prev_index = if self.get_position() > 3 {
            self.queue_index()
        } else {
            (self.queue_index() - 1).max(0)
        };

        self.load_song(prev_index);
        self.play();
    }

    pub fn play(&self) {
        self.player().play();
    }

    pub fn pause(&self) {
        self.player().pause();
    }

    pub fn stop(&self) {
        self.player().stop();
        self.set_property("position", 0u32);
        self.set_property("duration", 0u32);
    }

    pub fn seek(&self, position: u32) {
        self.player().seek(position as u64);
        self.set_property("position", position);
        // Some MRPIS clients care about this I guess
        self.notify_mpris_seeked(position);
    }

    pub fn get_position(&self) -> u64 {
        self.player().get_position().unwrap_or(0)
    }

    pub fn toggle_play_pause(&self) {
        if self.playing() {
            self.pause();
        } else {
            self.play();
        }
    }

    pub fn current_song(&self) -> Option<SongModel> {
        let index = self.imp().queue_index.get();
        if index >= 0 {
            self.imp().queue.borrow().get(index as usize).cloned()
        } else {
            None
        }
    }

    pub fn current_song_title(&self) -> String {
        self.current_song().map(|s| s.title()).unwrap_or_default()
    }

    pub fn current_song_artists(&self) -> Vec<String> {
        self.current_song().map(|s| s.artists()).unwrap_or_default()
    }

    pub fn current_song_album(&self) -> String {
        self.current_song().map(|s| s.album()).unwrap_or_default()
    }

    pub fn current_song_id(&self) -> String {
        self.current_song().map(|s| s.id()).unwrap_or_default()
    }

    pub fn set_shuffle_enabled(&self, enabled: bool) {
        self.imp().shuffle_enabled.set(enabled);
        if enabled {
            self.new_shuffle_cycle();
        }
    }

    fn new_shuffle_cycle(&self) {
        let new_seed = rand::rng().random::<u64>();
        self.imp().shuffle_seed.set(new_seed);
        self.imp().shuffle_index.set(0);
    }

    fn get_shuffle_order(&self) -> Vec<usize> {
        let queue_len = self.queue().len();
        if queue_len == 0 {
            return Vec::new();
        }
        let mut indicies: Vec<usize> = (0..queue_len).collect();
        use rand::{SeedableRng, seq::SliceRandom};
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.imp().shuffle_seed.get());
        indicies.shuffle(&mut rng);
        indicies
    }

    fn next_shuffled(&self) {
        let shuffle_order = self.get_shuffle_order();
        let current_pos = self.imp().shuffle_index.get();
        let next_pos = current_pos + 1;

        if next_pos < shuffle_order.len() {
            // Play next song in shuffle
            self.imp().shuffle_index.set(next_pos);
            let song_index = shuffle_order[next_pos];
            self.load_song(song_index as i32);
            self.play();
        } else {
            self.new_shuffle_cycle();
            self.stop();
            self.emit_by_name::<()>("queue-finished", &[]);
        }
    }

    fn prev_shuffled(&self) {
        let shuffle_order = self.get_shuffle_order();
        let current_pos = self.imp().shuffle_index.get();

        if self.get_position() > 3 {
            dbg!("current position");
            // Restart current song if less then 3 seconds have elapsed
            if let Some(&song_index) = shuffle_order.get(current_pos) {
                dbg!(&shuffle_order);
                dbg!(&current_pos);
                dbg!(&song_index);
                self.load_song(song_index as i32);
            }
        } else if current_pos > 0 {
            // Go to previous song
            let prev_pos = current_pos - 1;
            self.imp().shuffle_index.set(prev_pos);
            if let Some(&song_index) = shuffle_order.get(prev_pos) {
                self.load_song(song_index as i32);
            }
        } else {
            // At start of shuffle - just restart first song
            if let Some(&song_index) = shuffle_order.first() {
                self.load_song(song_index as i32);
            }
        }
        self.play();
    }
}

impl Default for AudioModel {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use std::cell::OnceCell;

    use mpris_server::LocalServer;

    use crate::{audio::player::AudioPlayer, models::SongModel};

    use super::*;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::AudioModel)]
    pub struct AudioModel {
        #[property(get, set)]
        pub queue_index: Cell<i32>,

        #[property(get, set)]
        pub playing: Cell<bool>,

        #[property(get, set)]
        pub paused: Cell<bool>,

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

        pub player: OnceCell<AudioPlayer>,
        pub queue: RefCell<Vec<SongModel>>,
        pub mpris_server: OnceCell<LocalServer<super::AudioModel>>,
        pub shuffle_enabled: Cell<bool>,
        pub shuffle_index: Cell<usize>,
        pub shuffle_seed: Cell<u64>,
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
                    glib::subclass::Signal::builder("song-changed")
                        .param_types([String::static_type()])
                        .build(),
                    glib::subclass::Signal::builder("queue-finished").build(),
                    glib::subclass::Signal::builder("error")
                        .param_types([String::static_type()])
                        .build(),
                    glib::subclass::Signal::builder("request-stream-uri")
                        .param_types([String::static_type()])
                        .return_type::<String>()
                        .build(),
                    glib::subclass::Signal::builder("queue-changed")
                        .param_types([u32::static_type(), u32::static_type()])
                        .build(),
                ]
            })
        }

        fn constructed(&self) {
            self.parent_constructed();
            glib::spawn_future_local(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                async move {
                    if let Err(e) = imp.obj().initialize_mpris().await {
                        warn!("Failed to initialize MPRIS: {}", e);
                    }
                }
            ));
        }
    }

    impl AudioModel {
        pub fn set_volume(&self, volume: f64) {
            let clamped_volume = volume.clamp(0.0, 1.0);
            self.volume.set(clamped_volume);

            if let Some(player) = self.player.get() {
                player.set_volume(clamped_volume);
            }

            self.obj().notify_mpris_volume();
        }

        pub fn set_muted(&self, muted: bool) {
            self.muted.set(muted);

            if let Some(player) = self.player.get() {
                player.set_mute(muted);
            }
        }
    }
}
