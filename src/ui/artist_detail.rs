use crate::{
    async_utils::spawn_tokio,
    jellyfin::api::ImageType,
    library_utils::albums_for_artist,
    models::{AlbumModel, ArtistModel},
    ui::{
        album_detail::AlbumDetail, image_utils::bytes_to_texture, page_traits::DetailPage,
        widget_ext::WidgetApplicationExt,
    },
};
use glib::Object;
use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use log::warn;

glib::wrapper! {
    pub struct ArtistDetail(ObjectSubclass<imp::ArtistDetail>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl DetailPage for ArtistDetail {
    type Model = ArtistModel;

    fn set_model(&self, model: &ArtistModel) {
        let imp = self.imp();
        imp.model.replace(Some(model.clone()));
        imp.artist_name.set_text(&model.name());
        self.load_banner_image();
        self.pull_albums();
    }

    fn get_model(&self) -> Option<Self::Model> {
        self.imp().model.borrow().clone()
    }
}

impl ArtistDetail {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn pull_albums(&self) {
        let library = self.get_application().library().clone();
        let albums: Vec<AlbumModel> = albums_for_artist(&self.id(), &library.borrow());
        self.imp().albums.replace(albums);
        while let Some(child) = self.imp().albums_box.first_child() {
            self.imp().albums_box.remove(&child);
        }
        for album in self.imp().albums.borrow().iter() {
            let album_widget = AlbumDetail::new();
            self.imp().albums_box.append(&album_widget);
            album_widget.set_model(album);
            album_widget.imp().artist_label.set_visible(false);
        }
    }

    pub fn load_banner_image(&self) {
        let Some(image_cache) = self.get_application().image_cache() else {
            // TODO: hide banner here
            return;
        };
        let Some(model) = self.imp().model.borrow().clone() else {
            // TODO: hide banner here
            return;
        };
        let jellyfin = self.get_application().jellyfin();
        let item_id = model.id();
        spawn_tokio(
            async move {
                image_cache
                    .get_image(&item_id, ImageType::Backdrop, &jellyfin)
                    .await
            },
            glib::clone!(
                #[weak(rename_to = artist_detail)]
                self,
                move |result| {
                    match result {
                        Ok(image_data) => {
                            artist_detail.set_image(&image_data);
                        }
                        Err(err) => {
                            warn!("Failed to load artist banner image: {}", err);
                        }
                    }
                }
            ),
        );
    }

    pub fn set_image(&self, image_data: &[u8]) {
        let image_data_copy = image_data.to_vec();
        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to=artist_detail)]
            self,
            async move {
                match bytes_to_texture(&image_data_copy).await {
                    Ok(texture) => {
                        artist_detail
                            .imp()
                            .banner_image
                            .set_paintable(Some(&texture));
                    }
                    Err(err) => {
                        warn!("Failed to load album image: {}", err);
                    }
                }
            }
        ));
    }
}

impl Default for ArtistDetail {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use std::cell::RefCell;

    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib::{self},
    };

    use crate::models::{AlbumModel, ArtistModel};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/artist_detail.ui")]
    pub struct ArtistDetail {
        #[template_child]
        pub artist_name: TemplateChild<gtk::Label>,
        #[template_child]
        pub banner_image: TemplateChild<gtk::Picture>,
        #[template_child]
        pub albums_box: TemplateChild<gtk::Box>,

        pub model: RefCell<Option<ArtistModel>>,
        pub albums: RefCell<Vec<AlbumModel>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ArtistDetail {
        const NAME: &'static str = "GellyArtistDetail";
        type Type = super::ArtistDetail;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl BoxImpl for ArtistDetail {}
    impl ObjectImpl for ArtistDetail {}
    impl WidgetImpl for ArtistDetail {}
}
