use crate::{
    async_utils::spawn_tokio, library_utils::play_artist, models::ArtistModel,
    ui::widget_ext::WidgetApplicationExt,
};
use glib::Object;
use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use log::warn;

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
        let imp = self.imp();
        let card = &imp.media_card;
        card.set_primary_text(&artist_model.name());
        card.set_image_id(&artist_model.id());
        card.set_favorite(artist_model.favorite());
        imp.artist_model.replace(Some(artist_model.clone()));
        let binding = artist_model
            .bind_property("favorite", &card.get(), "favorite")
            .build();
        imp.favorite_binding.replace(Some(binding));
    }

    pub fn play(&self) {
        if let Some(artist_model) = self.imp().artist_model.borrow().as_ref() {
            play_artist(&artist_model.id(), &self.get_application());
        }
    }

    pub fn toggle_favorite(&self, is_favorite: bool) {
        let Some(artist_model) = self.imp().artist_model.borrow().clone() else {
            return;
        };
        let item_id = artist_model.id();
        artist_model.set_favorite(is_favorite);
        let app = self.get_application();
        let backend = app.jellyfin();
        spawn_tokio(
            async move {
                backend
                    .set_favorite(
                        &item_id,
                        &crate::jellyfin::api::ItemType::MusicArtist,
                        is_favorite,
                    )
                    .await
            },
            glib::clone!(
                #[weak(rename_to = artist)]
                self,
                #[weak]
                artist_model,
                move |result| {
                    if let Err(err) = result {
                        warn!("Failed to set favorite: {err}");
                        artist_model.set_favorite(!is_favorite);
                        artist.get_application().refresh_favorites(true);
                    }
                }
            ),
        );
    }
}

impl Default for Artist {
    fn default() -> Self {
        Self::new()
    }
}
mod imp {
    use std::cell::RefCell;

    use crate::{models::ArtistModel, ui::media_card::MediaCard};
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

        pub artist_model: RefCell<Option<ArtistModel>>,
        pub favorite_binding: RefCell<Option<glib::Binding>>,
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
            self.media_card.connect_star_toggled(glib::clone!(
                #[weak(rename_to = artist)]
                self.obj(),
                move |is_favorite| {
                    artist.toggle_favorite(is_favorite);
                }
            ));
        }
    }
    impl WidgetImpl for Artist {}
}
