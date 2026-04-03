use crate::{audio::model::AudioModel, ui::player_bar::common::PlayerImp};
use adw::prelude::*;
use glib::Object;
use gtk::{gio, glib, subclass::prelude::*};
use log::debug;

glib::wrapper! {
    pub struct BigPlayer(ObjectSubclass<imp::BigPlayer>)
    @extends gtk::Widget, adw::Bin,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl BigPlayer {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn bind_to_audio_model(&self, audio_model: &AudioModel) {
        let imp = self.imp();

        if let Err(e) = imp.audio_model.set(audio_model.clone()) {
            debug!("Audio model already set: {:?}", e);
            return;
        };

        imp.playback_mode_menu.bind_to_audio_model(audio_model);

        imp.volume_control
            .bind_property("value", audio_model, "volume")
            .bidirectional()
            .sync_create()
            .build();

        audio_model.connect_closure(
            "play",
            false,
            glib::closure_local!(
                #[weak(rename_to = player)]
                self,
                move |_audio_model: AudioModel| {
                    player.imp().update_play_pause_button(true);
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
                    player.imp().update_play_pause_button(false);
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
                    player.imp().update_play_pause_button(false);
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
                move |_audio_model, _| {
                    player.imp().update_song_info();
                }
            ),
        );

        // Initial update
        self.imp().update_song_info();
        self.imp().update_play_pause_button(audio_model.playing());
    }
}

impl Default for BigPlayer {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use std::cell::{OnceCell, RefCell};

    use crate::{
        audio::model::AudioModel,
        ui::{album_art::AlbumArt, playback_mode::PlaybackModeMenu, player_bar::common::PlayerImp},
    };
    use adw::{prelude::*, subclass::prelude::*};
    use glib::{Properties, WeakRef};
    use gtk::{
        CompositeTemplate, TemplateChild,
        glib::{self},
    };

    #[derive(CompositeTemplate, Default, Properties)]
    #[template(resource = "/io/m51/Gelly/ui/player_bar/big_player.ui")]
    #[properties(wrapper_type = super::BigPlayer)]
    pub struct BigPlayer {
        #[template_child]
        pub album_art: TemplateChild<AlbumArt>,
        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub artist_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub artist_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub album_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub album_label: TemplateChild<gtk::Label>,
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
        pub volume_control: TemplateChild<gtk::ScaleButton>,
        #[template_child]
        pub info_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub lyrics_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub playback_mode_menu: TemplateChild<PlaybackModeMenu>,

        pub audio_model: OnceCell<AudioModel>,
        pub lyrics_window: RefCell<Option<WeakRef<adw::Window>>>,
        pub seek_debounce_id: RefCell<Option<glib::SourceId>>,

        #[property(get, set)]
        pub position: RefCell<u32>,

        #[property(get, set)]
        pub duration: RefCell<u32>,

        #[property(nullable, get, set)]
        pub album_art_paintable: RefCell<Option<gtk::gdk::Paintable>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BigPlayer {
        const NAME: &'static str = "GellyBigPlayer";
        type Type = super::BigPlayer;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for BigPlayer {
        fn constructed(&self) {
            self.parent_constructed();
            self.setup_common_signals();
            self.setup_clickable_labels();
            self.setup_volume_icons();

            self.album_art.connect_closure(
                "album-art-changed",
                true,
                glib::closure_local!(
                    #[weak(rename_to = this)]
                    self,
                    move |_: AlbumArt, has_album_art: bool| {
                        this.update_album_art_paintable(has_album_art);
                    }
                ),
            );
        }
    }

    impl BinImpl for BigPlayer {}
    impl WidgetImpl for BigPlayer {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            self.snapshot_background(snapshot);
            self.parent_snapshot(snapshot);
        }
    }

    impl PlayerImp for BigPlayer {
        fn audio_model(&self) -> &AudioModel {
            self.audio_model.get().expect("AudioModel not initialized")
        }
        fn lyrics_window(&self) -> &RefCell<Option<WeakRef<adw::Window>>> {
            &self.lyrics_window
        }
        fn seek_debounce_id(&self) -> &RefCell<Option<glib::SourceId>> {
            &self.seek_debounce_id
        }
        fn position_storage(&self) -> &RefCell<u32> {
            &self.position
        }
        fn duration_storage(&self) -> &RefCell<u32> {
            &self.duration
        }
        fn play_pause_button(&self) -> &gtk::Button {
            &self.play_pause_button
        }
        fn next_button(&self) -> &gtk::Button {
            &self.next_button
        }
        fn prev_button(&self) -> &gtk::Button {
            &self.prev_button
        }
        fn volume_control(&self) -> &gtk::ScaleButton {
            &self.volume_control
        }
        fn info_button(&self) -> &gtk::Button {
            &self.info_button
        }
        fn position_scale(&self) -> &gtk::Scale {
            &self.position_scale
        }
        fn position_label(&self) -> &gtk::Label {
            &self.position_label
        }
        fn duration_label(&self) -> &gtk::Label {
            &self.duration_label
        }
        fn lyrics_button(&self) -> &gtk::Button {
            &self.lyrics_button
        }
        fn artist_button(&self) -> &gtk::Button {
            &self.artist_button
        }
        fn album_button(&self) -> &gtk::Button {
            &self.album_button
        }
        fn title_label(&self) -> &gtk::Label {
            &self.title_label
        }
        fn artist_label(&self) -> &gtk::Label {
            &self.artist_label
        }
        fn album_label(&self) -> &gtk::Label {
            &self.album_label
        }
        fn album_art(&self) -> &AlbumArt {
            &self.album_art
        }
    }

    impl BigPlayer {
        pub fn update_album_art_paintable(&self, has_album_art: bool) {
            let paintable = if has_album_art {
                self.album_art.imp().album_image.paintable()
            } else {
                None
            };
            self.obj().set_album_art_paintable(paintable);
        }
    }
}
