use crate::audio::model::AudioModel;
use glib::Object;
use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use log::debug;

glib::wrapper! {
    pub struct PlayerBar(ObjectSubclass<imp::PlayerBar>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl PlayerBar {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn bind_to_audio_model(&self, audio_model: &AudioModel) {
        let imp = self.imp();

        match imp.audio_model.set(audio_model.clone()) {
            Ok(_) => (),
            Err(e) => debug!("Audio model already set: {:?}", e),
        };

        audio_model.connect_closure(
            "play",
            false,
            glib::closure_local!(
                #[weak(rename_to = player)]
                self,
                move |_audio_model: AudioModel| {
                    player.update_play_pause_button(true);
                    player.reveal();
                }
            ),
        );

        audio_model.connect_closure(
            "pause",
            false,
            glib::closure_local!(
                #[weak(rename_to = player)]
                self,
                move |_audio_model: AudioModel| {
                    player.update_play_pause_button(false);
                }
            ),
        );

        audio_model.connect_closure(
            "queue-finished",
            false,
            glib::closure_local!(
                #[weak(rename_to = player)]
                self,
                move |_audio_model: AudioModel| {
                    player.update_play_pause_button(false);
                    player.hide();
                }
            ),
        );

        audio_model
            .bind_property("position", self, "position")
            .sync_create()
            .build();

        audio_model
            .bind_property("duration", self, "duration")
            .sync_create()
            .build();

        audio_model.connect_notify_local(
            Some("queue-index"),
            glib::clone!(
                #[weak(rename_to = player)]
                self,
                move |audio_model, _| {
                    player.update_song_info(audio_model);
                    // Show player bar when a song is loaded (queue-index >= 0)
                    if audio_model.queue_index() >= 0 {
                        player.reveal();
                    }
                }
            ),
        );

        audio_model.connect_notify_local(
            Some("volume"),
            glib::clone!(
                #[weak(rename_to = player)]
                self,
                move |audio_model, _| {
                    let volume = audio_model.volume();
                    player.imp().volume_scale.set_value(volume);
                    player.imp().update_volume_icon(volume);
                }
            ),
        );

        // Initial update
        self.update_song_info(audio_model);
        self.update_play_pause_button(audio_model.playing());

        // Show player bar if there's already a song loaded
        if audio_model.queue_index() >= 0 {
            self.reveal();
        }
    }

    fn update_play_pause_button(&self, playing: bool) {
        let imp = self.imp();
        if playing {
            imp.play_pause_button
                .set_icon_name("media-playback-pause-symbolic");
            imp.play_pause_button.set_tooltip_text(Some("Pause"));
        } else {
            imp.play_pause_button
                .set_icon_name("media-playback-start-symbolic");
            imp.play_pause_button.set_tooltip_text(Some("Play"));
        }
    }

    fn update_song_info(&self, audio_model: &AudioModel) {
        let imp = self.imp();

        // Update title and artist
        let title = audio_model.current_song_title();
        let artists = audio_model.current_song_artists();
        let artist_str = if artists.is_empty() {
            "Unknown Artist".to_string()
        } else {
            artists.join(", ")
        };

        imp.title_label.set_text(&title);
        imp.artist_label.set_text(&artist_str);

        // Load album art if available
        if let Some(song) = audio_model.current_song() {
            self.load_album_art(&song.album_id(), &song.id());
        }
    }

    fn load_album_art(&self, album_id: &str, song_id: &str) {
        self.imp().album_art.set_item_id(song_id, Some(album_id));
    }

    fn format_time(seconds: u32) -> String {
        let minutes = seconds / 60;
        let seconds = seconds % 60;
        format!("{}:{:02}", minutes, seconds)
    }

    fn reveal(&self) {
        self.imp().action_bar.set_revealed(true);
    }

    fn hide(&self) {
        self.imp().action_bar.set_revealed(false);
    }
}

impl Default for PlayerBar {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use std::cell::{OnceCell, RefCell};

    use crate::{audio::model::AudioModel, ui::album_art::AlbumArt};
    use adw::subclass::prelude::*;
    use glib::{Properties, subclass::InitializingObject};
    use gtk::{
        CompositeTemplate,
        glib::{self},
        prelude::*,
    };

    #[derive(CompositeTemplate, Default, Properties)]
    #[template(resource = "/io/m51/Gelly/ui/player_bar.ui")]
    #[properties(wrapper_type = super::PlayerBar)]
    pub struct PlayerBar {
        #[template_child]
        pub action_bar: TemplateChild<gtk::ActionBar>,
        #[template_child]
        pub album_art: TemplateChild<AlbumArt>,
        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub artist_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub play_pause_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub prev_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub next_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub position_scale: TemplateChild<gtk::Scale>,
        #[template_child]
        pub position_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub duration_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub mute_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub volume_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub volume_scale: TemplateChild<gtk::Scale>,
        #[template_child]
        pub play_mode: TemplateChild<gtk::Button>,

        pub audio_model: OnceCell<AudioModel>,

        #[property(get, set)]
        pub position: RefCell<u32>,

        #[property(get, set)]
        pub duration: RefCell<u32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlayerBar {
        const NAME: &'static str = "GellyPlayerBar";
        type Type = super::PlayerBar;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for PlayerBar {
        fn constructed(&self) {
            self.parent_constructed();
            self.setup_signals();
        }
    }

    impl BoxImpl for PlayerBar {}
    impl WidgetImpl for PlayerBar {}

    impl PlayerBar {
        fn audio_model(&self) -> &AudioModel {
            self.audio_model.get().expect("AudioModel not initialized")
        }

        pub fn update_volume_icon(&self, volume: f64) {
            let icon_name = if volume == 0.0 {
                "audio-volume-muted-symbolic"
            } else if volume < 0.33 {
                "audio-volume-low-symbolic"
            } else if volume < 0.66 {
                "audio-volume-medium-symbolic"
            } else {
                "audio-volume-high-symbolic"
            };
            self.volume_button.set_icon_name(icon_name);
        }

        fn toggle_play_mode(&self) {
            let audio_model = self.audio_model();
            let shuffled = !audio_model.imp().shuffle_enabled.get();
            audio_model.set_shuffle_enabled(shuffled);
            let icon_name = if shuffled {
                "media-playlist-shuffle-symbolic"
            } else {
                "media-playlist-consecutive-symbolic"
            };
            self.play_mode.set_icon_name(icon_name);
        }

        fn setup_signals(&self) {
            self.play_pause_button.connect_clicked(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |_| {
                    imp.audio_model().toggle_play_pause();
                }
            ));

            self.next_button.connect_clicked(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |_| {
                    imp.audio_model().next();
                }
            ));

            self.prev_button.connect_clicked(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |_| {
                    imp.audio_model().prev();
                }
            ));

            self.mute_button.connect_clicked(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |_| {
                    imp.volume_scale.set_value(0.0);
                    imp.update_volume_icon(0.0);
                }
            ));

            self.play_mode.connect_clicked(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |_| {
                    imp.toggle_play_mode();
                }
            ));

            self.position_scale.connect_change_value(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                #[upgrade_or]
                glib::Propagation::Proceed,
                move |_, _, value| {
                    imp.audio_model().seek(value as u32);
                    glib::Propagation::Proceed
                }
            ));

            self.volume_scale.connect_value_changed(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |scale| {
                    let volume = scale.value();
                    imp.audio_model().imp().set_volume(volume);
                    imp.update_volume_icon(volume);
                }
            ));

            self.obj().connect_notify_local(
                Some("position"),
                glib::clone!(
                    #[weak(rename_to = imp)]
                    self,
                    move |_, _| {
                        let position = imp.position.borrow();
                        let duration = imp.duration.borrow();

                        imp.position_label
                            .set_text(&super::PlayerBar::format_time(*position));

                        if *duration > 0 {
                            imp.position_scale.set_value(*position as f64);
                        }
                    }
                ),
            );

            self.obj().connect_notify_local(
                Some("duration"),
                glib::clone!(
                    #[weak(rename_to = imp)]
                    self,
                    move |_, _| {
                        let duration = imp.duration.borrow();
                        imp.duration_label
                            .set_text(&super::PlayerBar::format_time(*duration));

                        if *duration > 0 {
                            imp.position_scale.adjustment().set_upper(*duration as f64);
                        }
                    }
                ),
            );
        }
    }
}
