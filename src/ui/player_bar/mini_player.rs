use crate::{
    audio::model::AudioModel,
    ui::{album_art::AlbumArt, player_bar::common::PlayerControls},
};
use adw::prelude::*;
use glib::Object;
use gtk::{glib, subclass::prelude::*};
use log::debug;

glib::wrapper! {
    pub struct MiniPlayerBar(ObjectSubclass<imp::MiniPlayerBar>)
    @extends gtk::Widget, adw::BreakpointBin,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl PlayerControls for MiniPlayerBar {
    fn play_pause_btn(&self) -> &gtk::Button {
        &self.imp().play_pause_button
    }
    fn title_label(&self) -> &gtk::Label {
        &self.imp().title_label
    }
    fn artist_label(&self) -> &gtk::Label {
        &self.imp().artist_label
    }
    fn album_label(&self) -> &gtk::Label {
        &self.imp().album_label
    }
    fn lyrics_btn(&self) -> &gtk::Button {
        &self.imp().lyrics
    }
    fn album_art(&self) -> &AlbumArt {
        &self.imp().album_art
    }
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

        // Bind playback mode menu
        imp.playback_mode_menu.bind_to_audio_model(audio_model);

        imp.volume_button
            .scale()
            .adjustment()
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

    use crate::{
        audio::model::AudioModel,
        ui::{
            album_art::AlbumArt, playback_mode::PlaybackModeMenu, player_bar::common::PlayerImp,
            volume_button::VolumeButton,
        },
    };
    use adw::{prelude::*, subclass::prelude::*};
    use glib::{Properties, WeakRef, subclass::InitializingObject};
    use gtk::{
        CompositeTemplate, TemplateChild,
        glib::{self},
    };

    #[derive(CompositeTemplate, Default, Properties)]
    #[template(resource = "/io/m51/Gelly/ui/player_bar/mini_player.ui")]
    #[properties(wrapper_type = super::MiniPlayerBar)]
    pub struct MiniPlayerBar {
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
        pub scale_position_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub scale_duration_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub volume_button: TemplateChild<VolumeButton>,
        #[template_child]
        pub info_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub lyrics: TemplateChild<gtk::Button>,
        #[template_child]
        pub playback_mode_menu: TemplateChild<PlaybackModeMenu>,

        pub audio_model: OnceCell<AudioModel>,
        pub bottom_sheet: OnceCell<adw::BottomSheet>,
        pub lyrics_window: RefCell<Option<WeakRef<adw::Window>>>,
        pub seek_debounce_id: RefCell<Option<glib::SourceId>>,

        #[property(get, set)]
        pub position: RefCell<u32>,

        #[property(get, set)]
        pub duration: RefCell<u32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MiniPlayerBar {
        const NAME: &'static str = "GellyMiniPlayerBar";
        type Type = super::MiniPlayerBar;
        type ParentType = adw::BreakpointBin;

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
            self.setup_common_signals();
            self.setup_clickable_labels();
        }
    }

    impl adw::subclass::prelude::BreakpointBinImpl for MiniPlayerBar {}
    impl WidgetImpl for MiniPlayerBar {}

    impl PlayerImp for MiniPlayerBar {
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
        fn volume_control(&self) -> &VolumeButton {
            &self.volume_button
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
            &self.lyrics
        }
        fn artist_button(&self) -> &gtk::Button {
            &self.artist_button
        }
        fn album_button(&self) -> &gtk::Button {
            &self.album_button
        }

        fn extra_position_update(&self, position: u32) {
            self.scale_position_label
                .set_text(&crate::ui::player_bar::common::format_time(position));
        }

        fn extra_duration_update(&self, duration: u32) {
            self.scale_duration_label
                .set_text(&crate::ui::player_bar::common::format_time(duration));
        }
    }
}
