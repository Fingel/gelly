use crate::{
    library_utils::tracks_for_album,
    models::{AlbumModel, SongModel},
    ui::widget_ext::WidgetApplicationExt,
};
use glib::Object;
use gtk::{gio, glib, subclass::prelude::*};
use log::warn;

glib::wrapper! {
    pub struct Album(ObjectSubclass<imp::Album>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup;
}
impl Album {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn set_album_name(&self, name: &str) {
        self.imp().name_label.set_text(name);
    }

    pub fn set_artist_name(&self, artist: &str) {
        self.imp().artist_label.set_text(artist);
    }

    pub fn set_album_model(&self, album_model: &AlbumModel) {
        self.set_album_name(&album_model.name());
        self.set_artist_name(&album_model.artists_string());
        self.imp().album_image.set_item_id(&album_model.id(), None);
        self.imp().album_id.replace(album_model.id().to_string());
    }

    pub fn play_album(&self) {
        let library = self.get_application().library().clone();
        let tracks = tracks_for_album(&self.imp().album_id.borrow(), &library.borrow());
        let songs: Vec<SongModel> = tracks.iter().map(SongModel::from).collect();
        if let Some(audio_model) = self.get_application().audio_model() {
            audio_model.set_playlist(songs, 0);
        } else {
            self.toast("Audio model not initialized, please restart", None);
            warn!("No audio model found");
        }
    }
}

impl Default for Album {
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
    #[template(resource = "/io/m51/Gelly/ui/album.ui")]
    pub struct Album {
        #[template_child]
        pub name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub artist_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub album_image: TemplateChild<AlbumArt>,
        #[template_child]
        pub play_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub motion_controller: TemplateChild<gtk::EventControllerMotion>,
        #[template_child]
        pub overlay_play: TemplateChild<gtk::Button>,

        pub album_id: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Album {
        const NAME: &'static str = "GellyAlbum";
        type Type = super::Album;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }
    impl BoxImpl for Album {}
    impl ObjectImpl for Album {
        fn constructed(&self) {
            self.parent_constructed();
            self.setup_signals();
        }
    }
    impl WidgetImpl for Album {}

    impl Album {
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
                    imp.obj().play_album();
                }
            ));
        }
    }
}
