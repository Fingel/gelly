use adw::prelude::*;
use gtk::{
    self, gio,
    glib::{self, Object},
    subclass::prelude::*,
};

use crate::{
    async_utils::spawn_tokio,
    config::{self, TranscodingProfile},
    i18n::tr,
    ui::widget_ext::WidgetApplicationExt,
};

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

        // Gapless Playback
        settings
            .bind("gapless-playback", &*imp.gapless_playback_row, "active")
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

    fn confirm_clear(&self, heading: &str, body: &str, on_confirm: impl Fn() + 'static) {
        let dialog = adw::AlertDialog::new(Some(heading), Some(body));
        dialog.add_responses(&[("cancel", &tr("Cancel")), ("clear", &tr("Clear"))]);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        dialog.set_response_appearance("clear", adw::ResponseAppearance::Destructive);
        dialog.connect_response(Some("clear"), move |_, _| {
            on_confirm();
        });
        dialog.present(Some(self));
    }

    fn calculate_media_size(&self) {
        if let Some(cache) = self.get_application().media_cache() {
            spawn_tokio(
                async move { cache.total_size().await },
                glib::clone!(
                    #[weak(rename_to=pref)]
                    self,
                    move |result| {
                        if let Ok(size) = result {
                            let label = glib::format_size(size);
                            pref.imp().media_usage_row.set_subtitle(&label);
                            pref.imp().clear_media_button.set_sensitive(true);
                        } else {
                            pref.imp().media_usage_row.set_subtitle(&tr("Unknown"));
                        }
                    }
                ),
            );
        } else {
            self.imp().media_usage_row.set_subtitle(&tr("Unknown"));
        }
    }

    fn confirm_clear_media(&self) {
        self.confirm_clear(
            &tr("Clear downloaded music?"),
            &tr("This will remove all offline media."),
            glib::clone!(
                #[weak(rename_to=pref)]
                self,
                move || {
                    pref.clear_media();
                }
            ),
        )
    }

    fn clear_media(&self) {
        self.imp().clear_media_button.set_sensitive(false);
        self.imp().media_usage_row.set_subtitle(&tr("Clearing…"));
        self.get_application().clear_media_cache(glib::clone!(
            #[weak(rename_to = pref)]
            self,
            move |result| {
                if let Err(error) = result {
                    log::warn!("Failed to clear downloaded music: {error}");
                    pref.toast(&tr("Failed to clear downloaded music."), None);
                }
                pref.calculate_media_size();
                pref.imp().clear_media_button.set_sensitive(true);
            }
        ));
    }

    fn calculate_image_size(&self) {
        if let Some(cache) = self.get_application().image_cache() {
            spawn_tokio(
                async move { cache.total_size().await },
                glib::clone!(
                    #[weak(rename_to=pref)]
                    self,
                    move |result| {
                        if let Ok(size) = result {
                            let label = glib::format_size(size);
                            pref.imp().image_usage_row.set_subtitle(&label);
                            pref.imp().clear_images_button.set_sensitive(true);
                        } else {
                            pref.imp().image_usage_row.set_subtitle(&tr("Unknown"));
                        }
                    }
                ),
            );
        } else {
            self.imp().image_usage_row.set_subtitle(&tr("Unknown"));
        }
    }

    fn confirm_clear_images(&self) {
        self.confirm_clear(
            &tr("Clear image cache?"),
            &tr("This will remove all cached album and artist art."),
            glib::clone!(
                #[weak(rename_to=pref)]
                self,
                move || {
                    pref.clear_images();
                }
            ),
        )
    }

    fn clear_images(&self) {
        self.imp().clear_images_button.set_sensitive(false);
        self.imp().image_usage_row.set_subtitle(&tr("Clearing…"));
        self.get_application().clear_image_cache(glib::clone!(
            #[weak(rename_to = pref)]
            self,
            move |result| {
                if let Err(error) = result {
                    log::warn!("Failed to clear cached images: {error}");
                    pref.toast(&tr("Failed to clear cached images."), None);
                }
                pref.calculate_image_size();
                pref.imp().clear_images_button.set_sensitive(true);
            }
        ));
    }
}

impl Default for Preferences {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use gtk::CompositeTemplate;
    use gtk::glib::{self, subclass::InitializingObject};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/preferences.ui")]
    pub struct Preferences {
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
        pub gapless_playback_row: TemplateChild<adw::SwitchRow>,
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
        #[template_child]
        pub media_usage_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub clear_media_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub image_usage_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub clear_images_button: TemplateChild<gtk::Button>,
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
            self.setup_signals();
        }
    }

    impl WidgetImpl for Preferences {}

    impl AdwDialogImpl for Preferences {}

    impl PreferencesDialogImpl for Preferences {}

    impl Preferences {
        fn setup_signals(&self) {
            self.obj().connect_map(glib::clone!(
                #[weak(rename_to = preferences)]
                self.obj(),
                move |_| {
                    preferences.calculate_media_size();
                    preferences.calculate_image_size();
                }
            ));

            self.clear_media_button.connect_clicked(glib::clone!(
                #[weak(rename_to = preferences)]
                self.obj(),
                move |_| {
                    preferences.confirm_clear_media();
                }
            ));
            self.clear_images_button.connect_clicked(glib::clone!(
                #[weak(rename_to = preferences)]
                self.obj(),
                move |_| {
                    preferences.confirm_clear_images();
                }
            ));
        }
    }
}
