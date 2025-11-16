use crate::models::ArtistModel;
use glib::Object;
use gtk::{gio, glib, subclass::prelude::*};

glib::wrapper! {
    pub struct Artist(ObjectSubclass<imp::Artist>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
impl Artist {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn set_artist_model(&self, artist_model: &ArtistModel) {
        let card = &self.imp().media_card;
        card.set_primary_text(&artist_model.name());
        card.set_image_id(&artist_model.id());
    }
}

impl Default for Artist {
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

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/artist.ui")]
    pub struct Artist {
        #[template_child]
        pub media_card: TemplateChild<MediaCard>,
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
