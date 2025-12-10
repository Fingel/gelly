use crate::{
    jellyfin::utils::format_duration,
    library_utils::songs_for_album,
    models::AlbumModel,
    ui::{page_traits::DetailPage, song::Song, widget_ext::WidgetApplicationExt},
};
use glib::Object;
use gtk::{gio, glib, prelude::*, subclass::prelude::*};
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

    fn enqueue_album(&self) {
        let songs = self.imp().songs.borrow().clone();
        if let Some(audio_model) = self.get_application().audio_model() {
            let song_cnt = songs.len();
            audio_model.append_to_queue(songs);
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
        pub enqueue: TemplateChild<gtk::Button>,

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
            self.setup_signals();
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

            self.play_all.connect_clicked(glib::clone!(
                #[weak(rename_to=imp)]
                self,
                move |_| {
                    imp.obj().play_album();
                }
            ));

            self.enqueue.connect_clicked(glib::clone!(
                #[weak(rename_to=imp)]
                self,
                move |_| {
                    imp.obj().enqueue_album();
                }
            ));
        }
    }
}
