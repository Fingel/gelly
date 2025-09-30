use crate::models::AlbumModel;
use crate::ui::image_utils::bytes_to_texture;
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
        match bytes_to_texture(image_data, None, None) {
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

    pub fn set_album_model(&self, album_model: &AlbumModel) {
        self.set_album_name(&album_model.name());
        self.set_artist_name(&album_model.artists_string());
        self.set_loading(album_model.image_loading());
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
