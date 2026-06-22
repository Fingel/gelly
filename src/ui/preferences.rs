use adw::prelude::ComboRowExt;
use gtk::{
    self, gio,
    glib::{self, Object},
    prelude::*,
    subclass::prelude::*,
};

use crate::config::{self, TranscodingProfile};

glib::wrapper! {
    pub struct Preferences(ObjectSubclass<imp::Preferences>)
    @extends gtk::Widget, adw::Dialog, adw::PreferencesDialog,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Preferences {
    pub fn new() -> Self {
        let obj: Self = Object::builder().build();
        obj.setup_bindings();
        obj
    }

    fn setup_bindings(&self) {
        let imp = self.imp();
        let settings = config::settings();

        // Close To Tray
        settings
            .bind("close-to-tray", &*imp.close_to_tray_row, "active")
            .build();

        // Max Bitrate
        settings
            .bind("max-bitrate", &*imp.maximum_bitrate_row, "value")
            .build();

        // Refresh on startup
        settings
            .bind("refresh-on-startup", &*imp.refresh_on_startup_row, "active")
            .build();

        settings
            .bind("compact-mode", &*imp.compact_mode_row, "active")
            .build();

        // Normalize Audio
        settings
            .bind("normalize-audio", &*imp.normalize_audio_row, "active")
            .build();

        settings
            .bind("inhibit-suspend", &*imp.inhibit_suspend_row, "active")
            .build();

        // Smart Playlists
        settings
            .bind(
                "playlist-favorites-enabled",
                &*imp.playlist_favorites_enabled_row,
                "active",
            )
            .build();
        settings
            .bind(
                "playlist-shuffle-enabled",
                &*imp.playlist_shuffle_enabled_row,
                "active",
            )
            .build();
        settings
            .bind(
                "playlist-most-played-enabled",
                &*imp.playlist_most_played_enabled_row,
                "active",
            )
            .build();
        settings
            .bind(
                "album-art-window-background",
                &*imp.album_art_background_row,
                "active",
            )
            .build();

        // Transcoding Profile
        imp.transcoding_profile_row
            .set_model(Some(&TranscodingProfile::as_string_list()));

        let current_profile = config::get_transcoding_profile();
        let initial_index = TranscodingProfile::PROFILES
            .iter()
            .position(|p| *p == current_profile)
            .unwrap_or(0) as u32;
        imp.transcoding_profile_row.set_selected(initial_index);

        imp.transcoding_profile_row
            .connect_selected_notify(move |row| {
                let selected_index = row.selected() as usize;
                if let Some(profile) = TranscodingProfile::PROFILES.get(selected_index) {
                    config::set_transcoding_profile(profile.clone());
                }
            });
    }
}

impl Default for Preferences {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use adw::subclass::prelude::*;
    use gtk::CompositeTemplate;
    use gtk::glib::{self, subclass::InitializingObject};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/preferences.ui")]
    pub struct Preferences {
        #[template_child]
        pub close_to_tray_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub transcoding_profile_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub maximum_bitrate_row: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub refresh_on_startup_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub compact_mode_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub normalize_audio_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub inhibit_suspend_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub playlist_favorites_enabled_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub playlist_shuffle_enabled_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub playlist_most_played_enabled_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub album_art_background_row: TemplateChild<adw::SwitchRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Preferences {
        const NAME: &'static str = "GellyPreferences";
        type Type = super::Preferences;
        type ParentType = adw::PreferencesDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Preferences {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for Preferences {}

    impl AdwDialogImpl for Preferences {}

    impl PreferencesDialogImpl for Preferences {}
}
