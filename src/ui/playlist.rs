use crate::{
    backend::BackendError,
    config::{self, BackendType},
    library_utils::songs_for_playlist,
    models::{PlaylistModel, SongModel},
    ui::widget_ext::WidgetApplicationExt,
};
use glib::Object;
use gtk::{self, gio, glib, prelude::*, subclass::prelude::*};
use log::warn;

glib::wrapper! {
    pub struct Playlist(ObjectSubclass<imp::Playlist>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
impl Playlist {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn set_playlist_model(&self, playlist_model: &PlaylistModel) {
        let imp = self.imp();
        let card = &imp.media_card;
        card.set_primary_text(&playlist_model.name());
        card.set_secondary_text(&format!("{} songs", playlist_model.child_count()));
        card.set_image_id(&playlist_model.id());
        card.set_has_star_button(config::get_backend_type() == BackendType::Jellyfin);
        if playlist_model.is_smart() {
            card.set_static_icon(playlist_model.playlist_type().icon_name());
            card.display_icon();
            card.set_has_star_button(false);
        }
        self.imp()
            .playlist_model
            .replace(Some(playlist_model.clone()));

        card.set_favorite(playlist_model.favorite());
        let binding = playlist_model
            .bind_property("favorite", &card.get(), "favorite")
            .build();
        imp.favorite_binding.replace(Some(binding));
    }

    fn get_playlist_model(&self) -> Option<PlaylistModel> {
        self.imp().playlist_model.borrow().clone()
    }

    fn play_songs(&self, songs: Vec<SongModel>) {
        if let Some(audio_model) = self.get_application().audio_model() {
            audio_model.set_queue(songs, 0, false);
        } else {
            self.toast("Audio model not initialized, please restart", None);
            warn!("No audio model found");
        }
    }

    fn play(&self) {
        let Some(playlist_model) = self.get_playlist_model() else {
            warn!("No playlist model set");
            return;
        };

        let app = self.get_application();
        songs_for_playlist(
            &playlist_model,
            &app,
            glib::clone!(
                #[weak(rename_to=playlist)]
                self,
                move |result: Result<Vec<SongModel>, BackendError>| {
                    match result {
                        Ok(songs) => {
                            playlist.play_songs(songs);
                        }
                        Err(error) => {
                            playlist.toast("Unable to load playlist", None);
                            warn!("Unable to load playlist: {error}");
                        }
                    }
                }
            ),
        );
    }

    pub fn toggle_favorite(&self, is_favorite: bool) {
        let Some(playlist_model) = self.imp().playlist_model.borrow().clone() else {
            return;
        };
        let app = self.get_application();
        playlist_model.toggle_favorite(is_favorite, &app);
        app.refresh_favorites(true);
    }
}

impl Default for Playlist {
    fn default() -> Self {
        Self::new()
    }
}
mod imp {
    use crate::{models::PlaylistModel, ui::media_card::MediaCard};
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib::{self},
    };
    use std::cell::RefCell;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/playlist.ui")]
    pub struct Playlist {
        #[template_child]
        pub media_card: TemplateChild<MediaCard>,

        pub playlist_model: RefCell<Option<PlaylistModel>>,
        pub favorite_binding: RefCell<Option<glib::Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Playlist {
        const NAME: &'static str = "GellyPlaylist";
        type Type = super::Playlist;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }
    impl ObjectImpl for Playlist {
        fn constructed(&self) {
            self.parent_constructed();
            self.media_card.connect_play_clicked(glib::clone!(
                #[weak(rename_to = playlist)]
                self.obj(),
                move || {
                    playlist.play();
                }
            ));

            self.media_card.connect_star_toggled(glib::clone!(
                #[weak(rename_to = playlist)]
                self.obj(),
                move |is_favorite| {
                    playlist.toggle_favorite(is_favorite);
                }
            ));
        }
    }

    impl WidgetImpl for Playlist {}
    impl BoxImpl for Playlist {}
}
