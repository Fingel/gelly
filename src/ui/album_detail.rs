use crate::{
    library_utils::tracks_for_album,
    models::{AlbumModel, SongModel},
    ui::{image_utils::bytes_to_texture, song::Song, widget_ext::WidgetApplicationExt},
};
use glib::Object;
use gtk::{gio, glib, subclass::prelude::*};
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
        } else {
            imp.year_label.set_text("");
        }
        if !album_model.image_data().is_empty() {
            match bytes_to_texture(&album_model.image_data(), None, None) {
                Ok(texture) => {
                    self.imp().album_image.set_paintable(Some(&texture));
                }
                Err(err) => {
                    warn!("Failed to load album image: {}", err);
                }
            }
        }
        self.pull_tracks();
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
}

impl Default for AlbumDetail {
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

    use crate::models::SongModel;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/album_detail.ui")]
    pub struct AlbumDetail {
        #[template_child]
        pub album_image: TemplateChild<gtk::Picture>,
        #[template_child]
        pub name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub artist_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub year_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub track_list: TemplateChild<gtk::ListBox>,

        pub album_id: RefCell<String>,
        pub songs: RefCell<Vec<SongModel>>,
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
        }
    }
}
