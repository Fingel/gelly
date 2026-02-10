use gtk::{
    gio,
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
    config,
    models::SongModel,
    reporting::{PlaybackEvent, ReportingManager},
    ui::playback_mode::PlaybackMode,
};

glib::wrapper! {
    pub struct AudioModel(ObjectSubclass<imp::AudioModel>);
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

                        obj.report_event(PlaybackEvent::StateChanged {
                            playing,
                            paused,
                            position: obj.position() as u64,
                            can_play: !obj.queue().is_empty(),
                            can_pause: obj.playing(),
                        });

                        match state {
                            PlayerState::Playing => obj.emit_by_name::<()>("play", &[]),
                            PlayerState::Paused => obj.emit_by_name::<()>("pause", &[]),
                            PlayerState::Stopped => obj.emit_by_name::<()>("stop", &[]),
                        }
                    }
                    PlayerEvent::PositionChanged(position) => {
                        obj.set_property("position", position as u32);
                        obj.report_event(PlaybackEvent::PositionChanged { position });
                    }
                    PlayerEvent::DurationChanged(dur) => {
                        obj.set_property("duration", dur as u32);
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

    pub fn initialize_reporting(&self) {
        let reporting_manager = ReportingManager::new(self);
        self.imp()
            .reporting_manager
            .set(reporting_manager)
            .expect("Reporting manager should only be set once");
    }

    fn report_event(&self, event: PlaybackEvent) {
        let reporting_manager = self
            .imp()
            .reporting_manager
            .get()
            .expect("Reporting manager should be initialized");
        reporting_manager.report_event(event);
    }

    pub fn application(&self) -> Option<crate::application::Application> {
        gio::Application::default()
            .and_then(|app| app.downcast::<crate::application::Application>().ok())
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

    fn apply_volume(&self) {
        let normalize_enabled = config::get_normalize_audio_enabled();
        let user_volume = self.volume();
        let user_linear = user_volume.powi(3).clamp(0.0, 1.0);
        let multiplier = if normalize_enabled && let Some(song) = self.current_song() {
            10f64.powf(song.normalization_gain() / 20.0)
        } else {
            1.0
        };
        let volume = (user_linear * multiplier).clamp(0.0, 10.0); // upper bound above one in case replaygain boosts volume

        self.player().set_volume(volume);
    }

    pub fn queue(&self) -> Vec<SongModel> {
        let queue = &self.imp().queue;
        (0..queue.n_items())
            .filter_map(|i| queue.item(i).and_downcast::<SongModel>())
            .collect()
    }

    pub fn queue_store(&self) -> gio::ListStore {
        self.imp().queue.clone()
    }

    pub fn queue_len(&self) -> i32 {
        self.imp().queue.n_items() as i32
    }

    pub fn set_queue(&self, songs: Vec<SongModel>, start_index: usize) {
        let song_len = songs.len();
        let queue = &self.imp().queue;
        queue.remove_all();
        queue.extend_from_slice(&songs);
        self.report_event(PlaybackEvent::NavigationChanged {
            can_go_next: song_len > 0,
            can_go_previous: start_index > 0,
            can_play: song_len > 0,
        });
        self.new_shuffle_cycle();
        if song_len > 0 {
            self.load_song(start_index as i32);
            self.play();
        } else {
            self.stop();
        }
    }

    pub fn replace_queue(&self, songs: Vec<SongModel>) {
        let queue = &self.imp().queue;
        queue.remove_all();
        queue.extend_from_slice(&songs);
        self.new_shuffle_cycle();
    }

    pub fn append_to_queue(&self, songs: Vec<SongModel>) {
        let songs_len = songs.len();
        self.imp().queue.extend_from_slice(&songs);
        let current_index = self.queue_index();
        self.report_event(PlaybackEvent::NavigationChanged {
            can_go_next: songs_len > 0,
            can_go_previous: current_index > 0,
            can_play: true,
        });
        self.new_shuffle_cycle();
    }

    pub fn prepend_to_queue(&self, songs: Vec<SongModel>) {
        let current_index = self.imp().queue_index.get();
        let index = if current_index < 1 && !self.player().is_playing() {
            0
        } else {
            current_index + 1
        } as usize;
        let queue = &self.imp().queue;
        for (i, song) in songs.into_iter().enumerate() {
            queue.insert((index + i) as u32, &song);
        }
        self.report_event(PlaybackEvent::NavigationChanged {
            can_go_next: index < self.queue_len() as usize,
            can_go_previous: current_index > 0,
            can_play: true,
        });
        self.new_shuffle_cycle();
    }

    pub fn clear_queue(&self) {
        self.imp().queue.remove_all();
        self.set_queue_index(-1);
        self.report_event(PlaybackEvent::NavigationChanged {
            can_go_next: false,
            can_go_previous: false,
            can_play: false,
        });
    }

    pub fn play_song(&self, index: usize) {
        self.load_song(index as i32);
        self.play();
    }

    fn load_song(&self, index: i32) {
        if let Some(song) = self
            .imp()
            .queue
            .item(index as u32)
            .and_downcast::<SongModel>()
        {
            let stream_uri = self.stream_uri(&song.id());
            let player = self.player();
            player.stop();
            self.set_property("position", 0u32);
            self.set_property("duration", 0u32);
            self.set_property("loading", true);
            self.set_queue_index(index);
            player.set_uri(&stream_uri);
            self.imp().uri.replace(Some(stream_uri));
            self.emit_by_name::<()>("song-changed", &[&song.id()]);
            self.apply_volume();
            let queue_len = self.queue().len() as i32;
            self.report_event(PlaybackEvent::TrackChanged {
                song: Some(song),
                position: 0,
                can_go_next: index >= 0 && (index + 1) < queue_len,
                can_go_previous: index > 0,
            })
        } else {
            self.stop();
            warn!("Failed to load song at index {}", index);
        }
    }

    pub fn next(&self) {
        if let Some(next_index) = self.next_index() {
            self.load_song(next_index);
            self.play();
        } else {
            self.stop();
            self.emit_by_name::<()>("queue-finished", &[]);
        }
    }

    pub fn prev(&self) {
        if let Some(prev_index) = self.prev_index() {
            self.load_song(prev_index);
            self.play()
        } else {
            self.stop();
        }
    }

    pub fn next_index(&self) -> Option<i32> {
        let mode = PlaybackMode::try_from(self.playback_mode()).unwrap_or(PlaybackMode::Normal);
        match mode {
            PlaybackMode::Normal => {
                let next_index = self.queue_index() + 1;
                if next_index < self.imp().queue.n_items() as i32 {
                    Some(next_index)
                } else {
                    None
                }
            }
            PlaybackMode::Shuffle => {
                let shuffle_order = self.get_shuffle_order();
                let current_pos = self.imp().shuffle_index.get();
                if current_pos < shuffle_order.len() {
                    shuffle_order.get(current_pos).map(|&song_index| {
                        self.imp().shuffle_index.set(current_pos + 1);
                        song_index as i32
                    })
                } else {
                    self.new_shuffle_cycle();
                    None
                }
            }
            PlaybackMode::Repeat => {
                let next_index = self.queue_index() + 1;
                if next_index < self.imp().queue.n_items() as i32 {
                    Some(next_index)
                } else {
                    Some(0)
                }
            }
            PlaybackMode::RepeatOne => Some(self.queue_index()),
        }
    }

    pub fn prev_index(&self) -> Option<i32> {
        let mode = PlaybackMode::try_from(self.playback_mode()).unwrap_or(PlaybackMode::Normal);
        match mode {
            PlaybackMode::Normal => {
                let index = if self.get_position() > 3 {
                    self.queue_index()
                } else {
                    (self.queue_index() - 1).max(0)
                };
                Some(index)
            }
            PlaybackMode::Shuffle => {
                let shuffle_order = self.get_shuffle_order();
                let current_pos = self.imp().shuffle_index.get();
                if self.get_position() > 3 {
                    // Restart current song if less then 3 seconds have elapsed
                    shuffle_order.get(current_pos).map(|idx| *idx as i32)
                } else if current_pos > 0 {
                    // Go to previous song
                    let prev_pos = current_pos - 1;
                    self.imp().shuffle_index.set(prev_pos);
                    shuffle_order.get(prev_pos).map(|idx| *idx as i32)
                } else {
                    // At start of shuffle - just restart first song
                    shuffle_order.first().map(|idx| *idx as i32)
                }
            }
            PlaybackMode::Repeat => {
                let index = if self.get_position() > 3 {
                    self.queue_index()
                } else {
                    let prev_index = self.queue_index() - 1;
                    if prev_index >= 0 {
                        prev_index
                    } else {
                        (self.imp().queue.n_items() as i32) - 1
                    }
                };
                Some(index)
            }
            PlaybackMode::RepeatOne => Some(self.queue_index()),
        }
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
        self.report_event(PlaybackEvent::Seeked {
            position: position.into(),
        });
    }

    pub fn get_uri(&self) -> Option<String> {
        self.imp().uri.borrow().clone()
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
            self.imp()
                .queue
                .item(index as u32)
                .and_downcast::<SongModel>()
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

    #[derive(Properties)]
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

        #[property(get, set)]
        pub playback_mode: Cell<u32>,

        pub player: OnceCell<AudioPlayer>,
        pub queue: gio::ListStore,
        pub mpris_server: OnceCell<LocalServer<super::AudioModel>>,
        pub reporting_manager: OnceCell<ReportingManager>,
        pub shuffle_index: Cell<usize>,
        pub shuffle_seed: Cell<u64>,
        pub uri: RefCell<Option<String>>,
    }

    impl Default for AudioModel {
        fn default() -> Self {
            Self {
                queue_index: Cell::new(0),
                playing: Cell::new(false),
                paused: Cell::new(false),
                loading: Cell::new(false),
                position: Cell::new(0),
                duration: Cell::new(0),
                volume: Cell::new(1.0),
                muted: Cell::new(false),
                playback_mode: Cell::new(0),
                player: OnceCell::new(),
                queue: gio::ListStore::new::<SongModel>(),
                mpris_server: OnceCell::new(),
                reporting_manager: OnceCell::new(),
                shuffle_index: Cell::new(0),
                shuffle_seed: Cell::new(0),
                uri: RefCell::new(None),
            }
        }
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
                ]
            })
        }

        fn constructed(&self) {
            self.parent_constructed();
            self.obj().initialize_reporting();
        }
    }

    impl AudioModel {
        pub fn set_shuffle_enabled(&self, enabled: bool) {
            self.playback_mode.set(if enabled {
                PlaybackMode::Shuffle as u32
            } else {
                PlaybackMode::Normal as u32
            });
            if enabled {
                self.obj().new_shuffle_cycle();
            }
        }

        // TODO change these to gobject setters
        pub fn set_volume(&self, volume: f64) {
            let clamped_volume = volume.clamp(0.0, 1.0);
            self.volume.set(clamped_volume);

            // Replaygain and normalization
            self.obj().apply_volume();

            self.obj()
                .report_event(PlaybackEvent::VolumeChanged { volume });
        }

        pub fn set_muted(&self, muted: bool) {
            self.muted.set(muted);

            if let Some(player) = self.player.get() {
                player.set_mute(muted);
            }
        }
    }
}
