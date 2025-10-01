use crate::models::ArtistModel;
use crate::ui::image_utils::bytes_to_texture;
use glib::Object;
use gtk::{gio, glib, prelude::WidgetExt, subclass::prelude::*};
use log::warn;

glib::wrapper! {
    pub struct Artist(ObjectSubclass<imp::Artist>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup;
}
impl Artist {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn set_name(&self, name: &str) {
        self.imp().name_label.set_text(name);
    }

    pub fn set_image(&self, image_data: &[u8]) {
        match bytes_to_texture(image_data, None, None) {
            Ok(texture) => {
                self.imp().artist_image.set_paintable(Some(&texture));
            }
            Err(err) => {
                warn!("Failed to load image: {}", err);
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

    pub fn set_artist_model(&self, artist_model: &ArtistModel) {
        self.set_name(&artist_model.name());
        self.set_loading(artist_model.image_loading());
    }
}

impl Default for Artist {
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
    #[template(resource = "/io/m51/Gelly/ui/artist.ui")]
    pub struct Artist {
        #[template_child]
        pub name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub artist_image: TemplateChild<gtk::Picture>,
        #[template_child]
        pub spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub error_icon: TemplateChild<gtk::Image>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Artist {
        const NAME: &'static str = "GellyArtist";
        type Type = super::Artist;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }
    impl BoxImpl for Artist {}
    impl ObjectImpl for Artist {}
    impl WidgetImpl for Artist {}
}
