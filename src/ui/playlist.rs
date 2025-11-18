use crate::{
    async_utils::spawn_tokio,
    jellyfin::{JellyfinError, api::MusicDto},
    models::{PlaylistModel, SongModel},
    ui::widget_ext::WidgetApplicationExt,
};
use glib::Object;
use gtk::{self, gio, glib, subclass::prelude::*};
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
        let card = &self.imp().media_card;
        card.set_primary_text(&playlist_model.name());
        card.set_secondary_text(&format!("{} songs", playlist_model.child_count()));
        card.set_image_id(&playlist_model.id());
        self.imp()
            .playlist_model
            .replace(Some(playlist_model.clone()));
    }

    fn get_playlist_model(&self) -> Option<PlaylistModel> {
        self.imp().playlist_model.borrow().clone()
    }

    fn play_songs(&self, songs: Vec<SongModel>) {
        if let Some(audio_model) = self.get_application().audio_model() {
            audio_model.set_queue(songs, 0);
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

        let playlist_id = playlist_model.id().to_string();
        let playlist_type = playlist_model.playlist_type();
        let library_data = self.get_application().library().borrow().clone();
        let jellyfin = self.get_application().jellyfin();

        spawn_tokio(
            async move {
                playlist_type
                    .load_song_data(&playlist_id, &jellyfin, &library_data)
                    .await
            },
            glib::clone!(
                #[weak(rename_to=playlist)]
                self,
                move |result: Result<Vec<MusicDto>, JellyfinError>| {
                    match result {
                        Ok(music_data) => {
                            let songs: Vec<SongModel> =
                                music_data.iter().map(SongModel::from).collect();
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

        // Store the PlaylistModel instead of just the ID
        // TODO why is this an optional
        pub playlist_model: RefCell<Option<PlaylistModel>>,
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
        }
    }

    impl WidgetImpl for Playlist {}
    impl BoxImpl for Playlist {}
}
