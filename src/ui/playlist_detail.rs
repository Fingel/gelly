use crate::{
    async_utils::spawn_tokio,
    jellyfin::{JellyfinError, api::PlaylistItems, utils::format_duration},
    library_utils::tracks_for_ids,
    models::{PlaylistModel, SongModel},
    ui::{song::Song, widget_ext::WidgetApplicationExt},
};
use glib::Object;
use gtk::{gio, glib, subclass::prelude::*};
use log::warn;

glib::wrapper! {
    pub struct PlaylistDetail(ObjectSubclass<imp::PlaylistDetail>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl PlaylistDetail {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn set_playlist_model(&self, playlist_model: &PlaylistModel) {
        let imp = self.imp();
        imp.playlist_id.replace(playlist_model.id());
        imp.name_label.set_text(&playlist_model.name());
        imp.album_image.set_item_id(&playlist_model.id(), None);
        self.pull_tracks();
    }

    fn pull_tracks(&self) {
        let playlist_id = self.imp().playlist_id.borrow().clone();
        let jellyfin = self.get_application().jellyfin();
        spawn_tokio(
            async move { jellyfin.get_playlist_items(&playlist_id).await },
            glib::clone!(
                #[weak(rename_to=playlist_detail)]
                self,
                move |result: Result<PlaylistItems, JellyfinError>| {
                    match result {
                        Ok(playlist_items) => {
                            playlist_detail.populate_tracks(playlist_items);
                        }
                        Err(error) => {
                            playlist_detail
                                .toast("Could not load playlist, please try again.", None);
                            warn!("Unable to load playlist: {error}");
                        }
                    }
                }
            ),
        );
    }

    fn populate_tracks(&self, playlist_items: PlaylistItems) {
        let library = self.get_application().library().clone();
        let tracks = tracks_for_ids(playlist_items.item_ids, &library.borrow());
        let songs: Vec<SongModel> = tracks.iter().map(SongModel::from).collect();
        let track_list = &self.imp().track_list;
        track_list.remove_all();
        for (i, song) in songs.iter().enumerate() {
            let song_widget = Song::new();
            // we don't want the track number here, we want the playlist index
            song.set_track_number(i as u32 + 1);
            song_widget.set_song_data(song);
            // Check if currently playing song is in the playlist
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
            audio_model.set_playlist(songs, index);
        } else {
            self.toast("Audio model not initialized, please restart", None);
            warn!("No audio model found");
        }
    }

    fn play_playlist(&self) {
        let songs = self.imp().songs.borrow().clone();
        if let Some(audio_model) = self.get_application().audio_model() {
            audio_model.set_playlist(songs, 0);
        } else {
            self.toast("Audio model not initialized, please restart", None);
            warn!("No audio model found");
        }
    }

    fn enqueue_playlist(&self) {
        let songs = self.imp().songs.borrow().clone();
        if let Some(audio_model) = self.get_application().audio_model() {
            let song_cnt = songs.len();
            audio_model.append_to_playlist(songs);
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
            .playlist_duration
            .set_text(&format_duration(duration));
    }
}

impl Default for PlaylistDetail {
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

    use crate::{models::SongModel, ui::album_art::AlbumArt};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/playlist_detail.ui")]
    pub struct PlaylistDetail {
        #[template_child]
        pub album_image: TemplateChild<AlbumArt>,
        #[template_child]
        pub name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub track_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub track_count: TemplateChild<gtk::Label>,
        #[template_child]
        pub playlist_duration: TemplateChild<gtk::Label>,
        #[template_child]
        pub play_all: TemplateChild<gtk::Button>,
        #[template_child]
        pub enqueue: TemplateChild<gtk::Button>,

        pub playlist_id: RefCell<String>,
        pub songs: RefCell<Vec<SongModel>>,
        pub song_change_signal_connected: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlaylistDetail {
        const NAME: &'static str = "GellyPlaylistDetail";
        type Type = super::PlaylistDetail;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }
    impl BoxImpl for PlaylistDetail {}
    impl ObjectImpl for PlaylistDetail {
        fn constructed(&self) {
            self.parent_constructed();
            self.setup_signals();
        }
    }
    impl WidgetImpl for PlaylistDetail {}

    impl PlaylistDetail {
        fn setup_signals(&self) {
            self.track_list.connect_row_activated(glib::clone!(
                #[weak(rename_to=imp)]
                self,
                move |_track_list, row| {
                    let index = row.index();
                    imp.obj().song_selected(index as usize);
                }
            ));

            self.play_all.connect_clicked(glib::clone!(
                #[weak(rename_to=imp)]
                self,
                move |_| {
                    imp.obj().play_playlist();
                }
            ));

            self.enqueue.connect_clicked(glib::clone!(
                #[weak(rename_to=imp)]
                self,
                move |_| {
                    imp.obj().enqueue_playlist();
                }
            ));
        }
    }
}
