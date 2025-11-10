use crate::{
    async_utils::spawn_tokio,
    jellyfin::{JellyfinError, api::PlaylistItems},
    library_utils::tracks_for_ids,
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

    pub fn set_playlist_name(&self, name: &str) {
        self.imp().name_label.set_text(name);
    }

    pub fn set_child_count(&self, child_count: u64) {
        self.imp()
            .child_count_label
            .set_text(&child_count.to_string());
    }

    pub fn set_playlist_model(&self, playlist_model: &PlaylistModel) {
        self.set_playlist_name(&playlist_model.name());
        self.set_child_count(playlist_model.child_count());
        self.imp()
            .album_image
            .set_item_id(&playlist_model.id(), None);
        self.imp()
            .playlist_id
            .replace(playlist_model.id().to_string());
    }

    fn play_songs(&self, playlist_items: PlaylistItems) {
        let library = self.get_application().library().clone();
        let tracks = tracks_for_ids(playlist_items.item_ids, &library.borrow());
        let songs: Vec<SongModel> = tracks.iter().map(SongModel::from).collect();
        if let Some(audio_model) = self.get_application().audio_model() {
            audio_model.set_queue(songs, 0);
        } else {
            self.toast("Audio model not initialized, please restart", None);
            warn!("No audio model found");
        }
    }

    pub fn play_playlist(&self) {
        let playlist_id = self.imp().playlist_id.borrow().clone();
        let jellyfin = self.get_application().jellyfin();
        spawn_tokio(
            async move { jellyfin.get_playlist_items(&playlist_id).await },
            glib::clone!(
                #[weak(rename_to=playlist)]
                self,
                move |result: Result<PlaylistItems, JellyfinError>| {
                    match result {
                        Ok(playlist_items) => {
                            playlist.play_songs(playlist_items);
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
    use std::cell::RefCell;

    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib::{self},
        prelude::*,
    };

    use crate::ui::album_art::AlbumArt;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/playlist.ui")]
    pub struct Playlist {
        #[template_child]
        pub name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub child_count_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub album_image: TemplateChild<AlbumArt>,
        #[template_child]
        pub play_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub motion_controller: TemplateChild<gtk::EventControllerMotion>,
        #[template_child]
        pub overlay_play: TemplateChild<gtk::Button>,

        pub playlist_id: RefCell<String>,
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
    impl BoxImpl for Playlist {}
    impl ObjectImpl for Playlist {
        fn constructed(&self) {
            self.parent_constructed();
            self.setup_signals();
        }
    }
    impl WidgetImpl for Playlist {}

    impl Playlist {
        fn setup_signals(&self) {
            self.motion_controller.connect_enter(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |_, _x, _y| {
                    imp.play_revealer.set_reveal_child(true);
                }
            ));

            self.motion_controller.connect_leave(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |_| {
                    imp.play_revealer.set_reveal_child(false);
                }
            ));

            self.overlay_play.connect_clicked(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |_| {
                    imp.obj().play_playlist();
                }
            ));
        }
    }
}
