use crate::{
    audio::model::AudioModel,
    ui::player_bar::{compact_player_bar::CompactPlayerBar, full_player_bar::FullPlayerBar},
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

impl MiniPlayerBar {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn full_bar(&self) -> FullPlayerBar {
        self.imp().full_bar.clone()
    }

    pub fn compact_bar(&self) -> CompactPlayerBar {
        self.imp().compact_bar.clone()
    }

    pub fn bind_to_audio_model(&self, audio_model: &AudioModel, bottom_sheet: &adw::BottomSheet) {
        let imp = self.imp();
        if let Err(e) = imp.bottom_sheet.set(bottom_sheet.clone()) {
            debug!("Bottom Sheet already set: {:?}", e);
            return;
        }

        imp.full_bar.bind_to_audio_model(audio_model);
        imp.compact_bar.bind_to_audio_model(audio_model);

        audio_model.connect_closure(
            "play",
            false,
            glib::closure_local!(
                #[weak(rename_to = player)]
                self,
                move |_audio_model: AudioModel| {
                    player.reveal();
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
                    player.hide();
                }
            ),
        );

        audio_model.connect_notify_local(
            Some("queue-index"),
            glib::clone!(
                #[weak(rename_to = player)]
                self,
                move |audio_model, _| {
                    if audio_model.queue_index() >= 0 {
                        player.reveal();
                    }
                }
            ),
        );

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
    use std::cell::OnceCell;

    use crate::ui::player_bar::{
        compact_player_bar::CompactPlayerBar, full_player_bar::FullPlayerBar,
    };
    use adw::{prelude::*, subclass::prelude::*};
    use glib::subclass::InitializingObject;
    use gtk::{CompositeTemplate, TemplateChild, glib};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/player_bar/mini_player.ui")]
    pub struct MiniPlayerBar {
        #[template_child]
        pub full_bar: TemplateChild<FullPlayerBar>,
        #[template_child]
        pub compact_bar: TemplateChild<CompactPlayerBar>,

        pub bottom_sheet: OnceCell<adw::BottomSheet>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MiniPlayerBar {
        const NAME: &'static str = "GellyMiniPlayerBar";
        type Type = super::MiniPlayerBar;
        type ParentType = adw::BreakpointBin;

        fn class_init(klass: &mut Self::Class) {
            // pre-registration calls on these types ensure that they are registered before the template reneders
            FullPlayerBar::static_type();
            CompactPlayerBar::static_type();
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MiniPlayerBar {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl BreakpointBinImpl for MiniPlayerBar {}
    impl WidgetImpl for MiniPlayerBar {}
}
