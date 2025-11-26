use crate::{
    library_utils::play_artist, models::ArtistModel, ui::widget_ext::WidgetApplicationExt,
};
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
        self.imp().artist_id.replace(artist_model.id().to_string());
    }

    pub fn play(&self) {
        play_artist(&self.imp().artist_id.borrow(), &self.get_application());
    }
}

impl Default for Artist {
    fn default() -> Self {
        Self::new()
    }
}
mod imp {
    use std::cell::RefCell;

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

        pub artist_id: RefCell<String>,
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
    impl ObjectImpl for Artist {
        fn constructed(&self) {
            self.parent_constructed();
            self.media_card.connect_play_clicked(glib::clone!(
                #[weak(rename_to = artist)]
                self.obj(),
                move || {
                    artist.play();
                }
            ));
        }
    }
    impl WidgetImpl for Artist {}
}
