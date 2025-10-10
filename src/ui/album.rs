use crate::models::AlbumModel;
use glib::Object;
use gtk::{gio, glib, subclass::prelude::*};

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
    }
}

impl Default for Album {
    fn default() -> Self {
        Self::new()
    }
}
mod imp {
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib::{self},
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
    impl ObjectImpl for Album {}
    impl WidgetImpl for Album {}
}
