use crate::{models::album_data::AlbumData, ui::image_utils::bytes_to_texture};
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

    pub fn set_album_data(&self, album_data: &AlbumData) {
        let imp = self.imp();
        imp.name_label.set_text(&album_data.name());
        imp.artist_label.set_text(&album_data.artists_string());
        if !album_data.image_data().is_empty() {
            match bytes_to_texture(&album_data.image_data()) {
                Ok(texture) => {
                    self.imp().album_image.set_paintable(Some(&texture));
                }
                Err(err) => {
                    warn!("Failed to load album image: {}", err);
                }
            }
        }
    }
}

impl Default for AlbumDetail {
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
    #[template(resource = "/io/m51/Gelly/ui/album_detail.ui")]
    pub struct AlbumDetail {
        #[template_child]
        pub album_image: TemplateChild<gtk::Picture>,
        #[template_child]
        pub name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub artist_label: TemplateChild<gtk::Label>,
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
    impl ObjectImpl for AlbumDetail {}
    impl WidgetImpl for AlbumDetail {}
}
