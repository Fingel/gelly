use crate::models::album_data::AlbumData;
use glib::Object;
use gtk::{
    gdk::Texture,
    gdk_pixbuf::{PixbufLoader, prelude::PixbufLoaderExt},
    gio, glib,
    prelude::WidgetExt,
    subclass::prelude::*,
};
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
        match self.bytes_to_texture(image_data) {
            Ok(texture) => {
                self.imp().album_image.set_paintable(Some(&texture));
            }
            Err(err) => {
                warn!("Failed to load album image: {}", err);
            }
        }
    }

    pub fn set_loading(&self, loading: bool) {
        self.imp().spinner.set_visible(loading);
        if loading {
            self.imp().spinner.start();
        } else {
            self.imp().spinner.stop();
        }
    }

    pub fn show_error(&self) {
        self.set_loading(false);
        self.imp().error_icon.set_visible(true);
    }

    pub fn set_album_data(&self, album_data: &AlbumData) {
        self.set_album_name(&album_data.name());
        self.set_artist_name(&album_data.primary_artist());
        self.set_loading(album_data.image_loading());
    }

    fn bytes_to_texture(&self, image_data: &[u8]) -> Result<Texture, glib::Error> {
        let loader = PixbufLoader::new();
        loader.write(image_data)?;
        loader.close()?;
        match loader.pixbuf() {
            Some(pixbuf) => Ok(Texture::for_pixbuf(&pixbuf)),
            None => Err(glib::Error::new(
                glib::FileError::Failed,
                "Failed to create pixbuf from image data",
            )),
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
        pub album_image: TemplateChild<gtk::Picture>,
        #[template_child]
        pub spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub error_icon: TemplateChild<gtk::Image>,
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
