use crate::models::ArtistModel;
use glib::Object;
use gtk::{gio, glib, subclass::prelude::*};

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

    pub fn set_artist_model(&self, artist_model: &ArtistModel) {
        self.set_name(&artist_model.name());
        self.imp().artist_image.set_item_id(&artist_model.id());
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

    use crate::ui::album_art::AlbumArt;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/artist.ui")]
    pub struct Artist {
        #[template_child]
        pub name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub artist_image: TemplateChild<AlbumArt>,
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
