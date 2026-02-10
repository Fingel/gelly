use glib::Object;
use gtk::{self, gio, glib, prelude::*, subclass::prelude::*};

use log::warn;
use num_enum::TryFromPrimitive;

use crate::audio::model::AudioModel;

#[derive(Debug, TryFromPrimitive)]
#[repr(u32)]
pub enum PlaybackMode {
    Normal = 0,
    Shuffle = 1,
    Repeat = 2,
    RepeatOne = 3,
}

impl PlaybackMode {
    fn icon_name(&self) -> &'static str {
        match self {
            PlaybackMode::Normal => "media-playlist-consecutive-symbolic",
            PlaybackMode::Shuffle => "media-playlist-shuffle-symbolic",
            PlaybackMode::Repeat => "media-playlist-repeat-symbolic",
            PlaybackMode::RepeatOne => "media-playlist-repeat-song-symbolic",
        }
    }

    fn label(&self) -> &'static str {
        match self {
            PlaybackMode::Normal => "No Shuffle/Repeat",
            PlaybackMode::Shuffle => "Shuffle",
            PlaybackMode::Repeat => "Repeat",
            PlaybackMode::RepeatOne => "Repeat One",
        }
    }
}

glib::wrapper! {
    pub struct PlaybackModeMenu(ObjectSubclass<imp::PlaybackModeMenu>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl PlaybackModeMenu {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn bind_to_audio_model(&self, audio_model: &AudioModel) {
        let imp = self.imp();
        if let Err(err) = imp.audio_model.set(audio_model.clone()) {
            warn!("Failed to set audio model in PlaybackModeMenu: {:?}", err);
            return;
        }

        let Some(action) = imp.action.get() else {
            warn!("PlaybackModeMenu action not initialized");
            return;
        };

        let initial_mode = audio_model.playback_mode();
        action.set_state(&initial_mode.to_variant());

        // Update icon
        if let Ok(mode) = PlaybackMode::try_from(initial_mode) {
            imp.menu_button.set_icon_name(mode.icon_name());
        }

        audio_model
            .bind_property("playback-mode", &*imp.menu_button, "icon-name")
            .transform_to(|_, value: u32| {
                PlaybackMode::try_from(value)
                    .ok()
                    .map(|mode| mode.icon_name().to_value())
            })
            .sync_create()
            .build();

        audio_model
            .bind_property("playback-mode", action, "state")
            .transform_to(|_, value: u32| Some(value.to_variant().to_value()))
            .sync_create()
            .build();
    }
}

impl Default for PlaybackModeMenu {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use std::cell::OnceCell;

    use adw::subclass::prelude::*;
    use gtk::{
        CompositeTemplate, TemplateChild, gio,
        glib::{self, subclass::InitializingObject},
        prelude::*,
    };

    use crate::{audio::model::AudioModel, ui::playback_mode::PlaybackMode};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/playback_mode.ui")]
    pub struct PlaybackModeMenu {
        #[template_child]
        pub menu_button: TemplateChild<gtk::MenuButton>,
        pub audio_model: OnceCell<AudioModel>,
        pub action: OnceCell<gio::SimpleAction>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlaybackModeMenu {
        const NAME: &'static str = "GellyPlaybackModeMenu";
        type Type = super::PlaybackModeMenu;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl PlaybackModeMenu {
        fn initialize_menu(&self) {
            let menu = gio::Menu::new();
            for mode in [
                super::PlaybackMode::Normal,
                super::PlaybackMode::Shuffle,
                super::PlaybackMode::Repeat,
                super::PlaybackMode::RepeatOne,
            ] {
                let item = gio::MenuItem::new(Some(mode.label()), None);
                item.set_action_and_target_value(
                    Some("playbackmode.mode"),
                    Some(&(mode as u32).to_variant()),
                );
                menu.append_item(&item);
            }
            self.menu_button.set_menu_model(Some(&menu));
        }

        fn create_actiongroup(&self) -> gio::SimpleActionGroup {
            let action_group = gio::SimpleActionGroup::new();
            let initial_state = (PlaybackMode::Normal as u32).to_variant();
            let action = gio::SimpleAction::new_stateful(
                "mode",
                Some(&u32::static_variant_type()),
                &initial_state,
            );

            // Store action so we can bind to it later
            let _ = self.action.set(action.clone());

            action.connect_activate(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |action, param| {
                    if let Some(new_state) = param {
                        action.set_state(new_state);
                        if let Some(mode_value) = new_state.get::<u32>()
                            && let Some(audio_model) = imp.audio_model.get()
                        {
                            audio_model.set_playback_mode(mode_value);
                        }
                    }
                }
            ));
            action_group.add_action(&action);
            action_group
        }
    }

    impl ObjectImpl for PlaybackModeMenu {
        fn constructed(&self) {
            self.parent_constructed();
            let action_group = self.create_actiongroup();
            self.obj()
                .insert_action_group("playbackmode", Some(&action_group));
            self.initialize_menu();
        }
    }
    impl WidgetImpl for PlaybackModeMenu {}
    impl BoxImpl for PlaybackModeMenu {}
}
