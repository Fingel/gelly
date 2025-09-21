use crate::cache::ImageCache;
use crate::models::album_data::AlbumData;
use glib::Object;
use gtk::{gio, glib, prelude::WidgetExt, subclass::prelude::*};
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

    pub fn set_album_image(&self, image_data: &[u8]) {
        // TODO: move the bytes to pixbuf method somewhere else.
        match ImageCache::bytes_to_texture(image_data) {
            Ok(texture) => {
                self.imp().album_image.set_paintable(Some(&texture));
                self.imp().spinner.set_visible(false);
            }
            Err(err) => {
                warn!("Failed to load album image: {}", err);
                self.set_album_name("ERROR");
                self.imp().spinner.set_visible(false);
            }
        }
    }

    pub fn show_loading(&self) {
        self.imp().spinner.set_visible(true);
        self.imp().album_image.clear();
    }

    pub fn show_error(&self) {
        // Todo show an actual error icon here
        self.set_album_name("ERROR");
        self.imp().spinner.set_visible(false);
    }

    pub fn set_album_data(&self, album_data: &AlbumData) {
        self.set_album_name(&album_data.name());
        self.set_artist_name(&album_data.primary_artist());

        if album_data.image_loading() {
            self.show_loading();
        }
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

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/album.ui")]
    pub struct Album {
        #[template_child]
        pub name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub artist_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub album_image: TemplateChild<gtk::Image>,
        #[template_child]
        pub spinner: TemplateChild<gtk::Label>,
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
