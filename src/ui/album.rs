use crate::{
    async_utils::spawn_tokio, library_utils::play_album, models::AlbumModel,
    ui::widget_ext::WidgetApplicationExt,
};
use glib::Object;
use gtk::{self, gio, glib, prelude::*, subclass::prelude::*};
use log::warn;

glib::wrapper! {
    pub struct Album(ObjectSubclass<imp::Album>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
impl Album {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn set_album_model(&self, album_model: &AlbumModel) {
        let imp = self.imp();
        let card = &imp.media_card;
        card.set_primary_text(&album_model.name());
        card.set_secondary_text(&album_model.artists_string());
        card.set_image_id(&album_model.id());
        imp.album_model.replace(Some(album_model.clone()));

        card.set_favorite(album_model.favorite());
        let binding = album_model
            .bind_property("favorite", &card.get(), "favorite")
            .build();
        imp.favorite_binding.replace(Some(binding));
    }

    pub fn play(&self) {
        if let Some(album_model) = self.imp().album_model.borrow().as_ref() {
            play_album(&album_model.id(), &self.get_application());
        }
    }

    pub fn toggle_favorite(&self, is_favorite: bool) {
        let item_id = if let Some(album_model) = self.imp().album_model.borrow().as_ref() {
            album_model.set_favorite(is_favorite);
            album_model.id()
        } else {
            return;
        };
        let app = self.get_application();
        let backend = app.jellyfin();
        spawn_tokio(
            async move {
                backend
                    .set_favorite(
                        &item_id,
                        &crate::jellyfin::api::ItemType::MusicAlbum,
                        is_favorite,
                    )
                    .await
            },
            glib::clone!(
                #[weak(rename_to = album)]
                self,
                move |result| {
                    if let Err(err) = result {
                        warn!("Failed to set favorite: {err}");
                        if let Some(model) = album.imp().album_model.borrow().as_ref() {
                            model.set_favorite(!is_favorite);
                        }
                    } else {
                        if let Some(model) = album.imp().album_model.borrow().as_ref() {
                            model.set_favorite(is_favorite);
                        }
                        album.get_application().refresh_favorites(true);
                    }
                }
            ),
        );
    }
}

impl Default for Album {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use crate::{models::AlbumModel, ui::media_card::MediaCard};
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib::{self},
    };
    use std::cell::RefCell;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/album.ui")]
    pub struct Album {
        #[template_child]
        pub media_card: TemplateChild<MediaCard>,

        pub album_model: RefCell<Option<AlbumModel>>,
        pub favorite_binding: RefCell<Option<glib::Binding>>,
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

    impl ObjectImpl for Album {
        fn constructed(&self) {
            self.parent_constructed();
            self.media_card.connect_play_clicked(glib::clone!(
                #[weak(rename_to = album)]
                self.obj(),
                move || {
                    album.play();
                }
            ));
            self.media_card.connect_star_toggled(glib::clone!(
                #[weak(rename_to = album)]
                self.obj(),
                move |is_favorite| {
                    album.toggle_favorite(is_favorite);
                }
            ));
        }
    }

    impl WidgetImpl for Album {}
    impl BoxImpl for Album {}
}
