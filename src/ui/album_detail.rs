use crate::{
    audio::model::AudioModel,
    jellyfin::utils::format_duration,
    library_utils::tracks_for_album,
    models::{AlbumModel, SongModel},
    ui::{song::Song, widget_ext::WidgetApplicationExt},
};
use glib::Object;
use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use log::warn;

glib::wrapper! {
    pub struct AlbumDetail(ObjectSubclass<imp::AlbumDetail>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup;
}

impl AlbumDetail {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn set_album_model(&self, album_model: &AlbumModel) {
        let imp = self.imp();
        imp.album_id.replace(album_model.id());
        imp.name_label.set_text(&album_model.name());
        imp.artist_label.set_text(&album_model.artists_string());
        if album_model.year() > 0 {
            imp.year_label.set_text(&album_model.year().to_string());
            imp.year_label.set_visible(true);
        } else {
            imp.year_label.set_visible(false);
        }
        imp.album_image.set_item_id(&album_model.id());
        self.pull_tracks();
        if let Some(audio_model) = self.get_application().audio_model() {
            self.update_playing_status(&audio_model.current_song_id());
        }
    }

    pub fn pull_tracks(&self) {
        let library = self.get_application().library().clone();
        let tracks = tracks_for_album(&self.imp().album_id.borrow(), &library.borrow());
        let songs: Vec<SongModel> = tracks.iter().map(SongModel::from).collect();
        let track_list = &self.imp().track_list;
        track_list.remove_all();
        for song in &songs {
            let song_widget = Song::new();
            song_widget.set_song_data(song);
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

    fn listen_for_song_changes(&self) {
        if let Some(audio_model) = self.get_application().audio_model() {
            audio_model.connect_closure(
                "song-changed",
                false,
                glib::closure_local!(
                    #[weak(rename_to = album_detail)]
                    self,
                    move |_audio_model: AudioModel, song_id: &str| {
                        album_detail.update_playing_status(song_id);
                    }
                ),
            );
        }
    }

    fn update_playing_status(&self, current_song_id: &str) {
        // Iterate through all song widgets in the track list
        let track_list = &self.imp().track_list;
        let mut row_index = 0;
        while let Some(song_widget) = track_list.row_at_index(row_index).and_downcast::<Song>() {
            if let Some(id) = song_widget.imp().item_id.borrow().clone() {
                let is_current = id == current_song_id;
                song_widget.imp().playing_icon.set_visible(is_current);
            }
            row_index += 1;
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

    use crate::{models::SongModel, ui::album_art::AlbumArt};

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

        pub album_id: RefCell<String>,
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
            self.setup_signals();
            self.obj().connect_map(glib::clone!(
                #[weak(rename_to = album_detail)]
                self.obj(),
                move |_| {
                    if !album_detail.imp().song_change_signal_connected.get() {
                        album_detail.listen_for_song_changes();
                        album_detail.imp().song_change_signal_connected.set(true);
                    }
                }
            ));
        }
    }
    impl WidgetImpl for AlbumDetail {}

    impl AlbumDetail {
        fn setup_signals(&self) {
            self.track_list.connect_row_activated(glib::clone!(
                #[weak(rename_to=imp)]
                self,
                move |_track_list, row| {
                    let index = row.index();
                    imp.obj().song_selected(index as usize);
                }
            ));
        }
    }
}
