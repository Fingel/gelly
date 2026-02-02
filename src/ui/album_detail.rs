use crate::{
    async_utils::spawn_tokio,
    jellyfin::utils::format_duration,
    library_utils::songs_for_album,
    models::AlbumModel,
    ui::{
        music_context_menu::{ContextActions, construct_menu, create_actiongroup},
        page_traits::DetailPage,
        song::Song,
        song_utils::connect_song_navigation,
        widget_ext::WidgetApplicationExt,
    },
};
use glib::Object;
use gtk::{
    gio::{self, SimpleActionGroup},
    glib,
    prelude::*,
    subclass::prelude::*,
};
use log::warn;

glib::wrapper! {
    pub struct AlbumDetail(ObjectSubclass<imp::AlbumDetail>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl DetailPage for AlbumDetail {
    type Model = AlbumModel;

    fn set_model(&self, model: &AlbumModel) {
        let imp = self.imp();
        imp.model.replace(Some(model.clone()));
        imp.name_label.set_text(&model.name());
        imp.artist_label.set_text(&model.artists_string());
        if model.year() > 0 {
            imp.year_label.set_text(&model.year().to_string());
            imp.year_label.set_visible(true);
        } else {
            imp.year_label.set_visible(false);
        }
        imp.album_image.set_item_id(&model.id(), None);
        self.pull_tracks();
    }

    fn get_model(&self) -> Option<Self::Model> {
        self.imp().model.borrow().clone()
    }
}

impl AlbumDetail {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn pull_tracks(&self) {
        let library = self.get_application().library().clone();
        let songs = songs_for_album(&self.id(), &library.borrow());
        let track_list = &self.imp().track_list;
        track_list.remove_all();
        for song in &songs {
            let song_widget = Song::new();
            song_widget.set_song_data(song);
            song_widget.connect_closure(
                "song-activated",
                false,
                glib::closure_local!(
                    #[weak(rename_to = album_detail)]
                    self,
                    move |song_widget: Song| {
                        album_detail.song_selected(song_widget.index() as usize);
                    }
                ),
            );

            // Set up navigation signals
            connect_song_navigation(&song_widget, &self.get_root_window());
            // Check if currently playing song is in the album
            if let Some(audio_model) = self.get_application().audio_model()
                && audio_model
                    .current_song()
                    .is_some_and(|current_song| current_song.id() == song.id())
            {
                song_widget.set_playing(true);
            }

            track_list.append(&song_widget);
        }
        self.imp().songs.replace(songs);
        self.update_track_metadata();
    }

    pub fn song_selected(&self, index: usize) {
        let songs = self.imp().songs.borrow().clone();
        if let Some(audio_model) = self.get_application().audio_model() {
            audio_model.set_queue(songs, index);
        } else {
            self.toast("Audio model not initialized, please restart", None);
            warn!("No audio model found");
        }
    }

    fn play_album(&self) {
        let songs = self.imp().songs.borrow().clone();
        if let Some(audio_model) = self.get_application().audio_model() {
            audio_model.set_queue(songs, 0);
        } else {
            self.toast("Audio model not initialized, please restart", None);
            warn!("No audio model found");
        }
    }

    fn enqueue_album(&self, to_end: bool) {
        let songs = self.imp().songs.borrow().clone();
        if let Some(audio_model) = self.get_application().audio_model() {
            let song_cnt = songs.len();
            if to_end {
                audio_model.append_to_queue(songs);
            } else {
                audio_model.prepend_to_queue(songs);
            }
            self.toast(&format!("{} songs added to queue", song_cnt), None);
        } else {
            self.toast("Audio model not initialized, please restart", None);
            warn!("No audio model found");
        }
    }

    fn update_track_metadata(&self) {
        let songs = self.imp().songs.borrow();
        self.imp().track_count.set_text(&songs.len().to_string());
        let duration = songs.iter().map(|song| song.duration()).sum::<u64>();
        self.imp()
            .album_duration
            .set_text(&format_duration(duration));
    }

    fn setup_menu(&self) {
        let options = ContextActions {
            can_remove_from_playlist: false,
            in_queue: false,
            action_prefix: "album".to_string(),
        };
        let popover_menu = construct_menu(
            &options,
            glib::clone!(
                #[weak(rename_to = album)]
                self,
                #[upgrade_or_default]
                move || album.get_application().playlists().borrow().clone()
            ),
        );
        self.imp().action_menu.set_popover(Some(&popover_menu));
        let action_group = self.create_action_group();
        self.insert_action_group(&options.action_prefix, Some(&action_group));
    }

    fn create_action_group(&self) -> SimpleActionGroup {
        let on_add_to_playlist = glib::clone!(
            #[weak(rename_to = album)]
            self,
            move |playlist_id| {
                album.on_add_to_playlist(playlist_id);
            }
        );

        let on_queue_next = glib::clone!(
            #[weak(rename_to = album)]
            self,
            move || {
                album.enqueue_album(false);
            }
        );

        let on_queue_last = glib::clone!(
            #[weak(rename_to = album)]
            self,
            move || {
                album.enqueue_album(true);
            }
        );

        create_actiongroup(
            Some(on_add_to_playlist),
            None::<fn()>,
            Some(on_queue_next),
            Some(on_queue_last),
        )
    }

    fn on_add_to_playlist(&self, playlist_id: String) {
        let song_ids = self
            .imp()
            .songs
            .borrow()
            .iter()
            .map(|song| song.id())
            .collect::<Vec<_>>();

        let app = self.get_application();
        let jellyfin = app.jellyfin();
        let playlist_id = playlist_id.to_string();
        spawn_tokio(
            async move { jellyfin.add_playlist_items(&playlist_id, &song_ids).await },
            glib::clone!(
                #[weak(rename_to = album)]
                self,
                move |result| {
                    match result {
                        Ok(()) => {
                            album.toast("Added album to playlist", None);
                            app.refresh_playlists(true);
                        }
                        Err(e) => {
                            album.toast("Failed to add album to playlist", None);
                            warn!("Failed to add album to playlist: {}", e);
                        }
                    }
                }
            ),
        );
    }
}

impl Default for AlbumDetail {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use std::cell::{Cell, RefCell};

    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib::{self},
        prelude::*,
    };

    use crate::{
        models::{AlbumModel, SongModel},
        ui::album_art::AlbumArt,
    };

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/album_detail.ui")]
    pub struct AlbumDetail {
        #[template_child]
        pub album_image: TemplateChild<AlbumArt>,
        #[template_child]
        pub name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub artist_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub year_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub track_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub track_count: TemplateChild<gtk::Label>,
        #[template_child]
        pub album_duration: TemplateChild<gtk::Label>,
        #[template_child]
        pub play_all: TemplateChild<gtk::Button>,
        #[template_child]
        pub action_menu: TemplateChild<gtk::MenuButton>,

        pub model: RefCell<Option<AlbumModel>>,
        pub songs: RefCell<Vec<SongModel>>,
        pub song_change_signal_connected: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AlbumDetail {
        const NAME: &'static str = "GellyAlbumDetail";
        type Type = super::AlbumDetail;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }
    impl BoxImpl for AlbumDetail {}
    impl ObjectImpl for AlbumDetail {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_menu();
            self.setup_signals();
        }
    }
    impl WidgetImpl for AlbumDetail {}

    impl AlbumDetail {
        fn setup_signals(&self) {
            self.play_all.connect_clicked(glib::clone!(
                #[weak(rename_to=imp)]
                self,
                move |_| {
                    imp.obj().play_album();
                }
            ));
        }
    }
}
