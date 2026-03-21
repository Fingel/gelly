use crate::audio::model::AudioModel;
use adw::prelude::*;
use glib::Object;
use gtk::{gio, glib, subclass::prelude::*};
use log::debug;

glib::wrapper! {
    pub struct MiniPlayerBar(ObjectSubclass<imp::MiniPlayerBar>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl MiniPlayerBar {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn bind_to_audio_model(&self, audio_model: &AudioModel, bottom_sheet: &adw::BottomSheet) {
        let imp = self.imp();
        if let Err(e) = imp.bottom_sheet.set(bottom_sheet.clone()) {
            debug!("Bottom Sheet already set: {:?}", e);
        }

        if let Err(e) = imp.audio_model.set(audio_model.clone()) {
            debug!("Audio model already set: {:?}", e);
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

        // Update title
        let title = audio_model.current_song_title();
        imp.title_label.set_text(&title);

        // Load album art
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
        if let Some(w) = self.imp().bottom_sheet.get() {
            w.set_reveal_bottom_bar(true);
        }
    }

    fn hide(&self) {
        if let Some(w) = self.imp().bottom_sheet.get() {
            w.set_reveal_bottom_bar(false);
        }
    }
}

impl Default for MiniPlayerBar {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use std::cell::{OnceCell, RefCell};

    use crate::{audio::model::AudioModel, ui::album_art::AlbumArt};
    use adw::{prelude::*, subclass::prelude::*};
    use glib::{Properties, subclass::InitializingObject};
    use gtk::{
        CompositeTemplate,
        glib::{self},
    };

    #[derive(CompositeTemplate, Default, Properties)]
    #[template(resource = "/io/m51/Gelly/ui/mini_player_bar.ui")]
    #[properties(wrapper_type = super::MiniPlayerBar)]
    pub struct MiniPlayerBar {
        #[template_child]
        pub album_art: TemplateChild<AlbumArt>,
        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub play_pause_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub prev_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub next_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub position_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub duration_label: TemplateChild<gtk::Label>,

        pub audio_model: OnceCell<AudioModel>,
        pub bottom_sheet: OnceCell<adw::BottomSheet>,

        #[property(get, set)]
        pub position: RefCell<u32>,

        #[property(get, set)]
        pub duration: RefCell<u32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MiniPlayerBar {
        const NAME: &'static str = "GellyMiniPlayerBar";
        type Type = super::MiniPlayerBar;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for MiniPlayerBar {
        fn constructed(&self) {
            self.parent_constructed();
            self.setup_signals();
        }
    }

    impl BoxImpl for MiniPlayerBar {}
    impl WidgetImpl for MiniPlayerBar {}

    impl MiniPlayerBar {
        fn audio_model(&self) -> &AudioModel {
            self.audio_model.get().expect("AudioModel not initialized")
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

            self.obj().connect_notify_local(
                Some("position"),
                glib::clone!(
                    #[weak(rename_to = imp)]
                    self,
                    move |_, _| {
                        let position = imp.position.borrow();
                        imp.position_label
                            .set_text(&super::MiniPlayerBar::format_time(*position));
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
                            .set_text(&super::MiniPlayerBar::format_time(*duration));
                    }
                ),
            );
        }
    }
}
