use crate::{
    library_utils::songs_for_album, models::AlbumModel, ui::widget_ext::WidgetApplicationExt,
};
use glib::Object;
use gtk::{self, gio, glib, subclass::prelude::*};
use log::warn;

glib::wrapper! {
    pub struct Album(ObjectSubclass<imp::Album>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
impl Album {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn set_album_model(&self, album_model: &AlbumModel) {
        let card = &self.imp().media_card;
        card.set_primary_text(&album_model.name());
        card.set_secondary_text(&album_model.artists_string());
        card.set_image_id(&album_model.id());
        self.imp().album_id.replace(album_model.id().to_string());
    }

    fn play(&self) {
        let library = self.get_application().library().clone();
        let songs = songs_for_album(&self.imp().album_id.borrow(), &library.borrow());
        if let Some(audio_model) = self.get_application().audio_model() {
            audio_model.set_queue(songs, 0);
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
    use crate::ui::media_card::MediaCard;
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib::{self},
    };
    use std::cell::RefCell;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/album.ui")]
    pub struct Album {
        #[template_child]
        pub media_card: TemplateChild<MediaCard>,

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

    impl ObjectImpl for Album {
        fn constructed(&self) {
            self.parent_constructed();
            self.media_card.connect_play_clicked(glib::clone!(
                #[weak(rename_to = album)]
                self.obj(),
                move || {
                    album.play();
                }
            ));
        }
    }

    impl WidgetImpl for Album {}
    impl BoxImpl for Album {}
}
