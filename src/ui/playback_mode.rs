use glib::Object;
use gtk::{self, gio, glib};

use num_enum::TryFromPrimitive;

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
}

impl Default for PlaybackModeMenu {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use adw::subclass::prelude::*;
    use gtk::{
        CompositeTemplate, TemplateChild, gio,
        glib::{self, subclass::InitializingObject},
        prelude::*,
    };

    use crate::ui::playback_mode::PlaybackMode;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/playback_mode.ui")]
    pub struct PlaybackModeMenu {
        #[template_child]
        pub menu_button: TemplateChild<gtk::MenuButton>,
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

        pub fn create_actiongroup(&self) -> gio::SimpleActionGroup {
            let action_group = gio::SimpleActionGroup::new();
            let initial_state = (PlaybackMode::Normal as u32).to_variant();
            let action = gio::SimpleAction::new_stateful(
                "mode",
                Some(&u32::static_variant_type()),
                &initial_state,
            );
            action.connect_activate(move |action, param| {
                if let Some(new_state) = param {
                    println!("Action activated with new state: {:?}", new_state);
                    action.set_state(new_state);

                    if let Some(mode_value) = new_state.get::<u32>()
                        && let Ok(mode) = super::PlaybackMode::try_from(mode_value)
                    {
                        println!("Selected playback mode: {:?}", mode);
                    }
                }
            });
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
