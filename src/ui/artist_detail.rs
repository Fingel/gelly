use crate::{
    library_utils::albums_for_artist,
    models::{AlbumModel, ArtistModel},
    ui::{album_detail::AlbumDetail, page_traits::DetailPage, widget_ext::WidgetApplicationExt},
};
use glib::Object;
use gtk::{gio, glib, prelude::*, subclass::prelude::*};

glib::wrapper! {
    pub struct ArtistDetail(ObjectSubclass<imp::ArtistDetail>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl DetailPage for ArtistDetail {
    type Model = ArtistModel;

    fn title(&self) -> String {
        self.imp()
            .model
            .borrow()
            .as_ref()
            .map(|m| m.name())
            .unwrap_or_default()
    }

    fn id(&self) -> String {
        self.imp()
            .model
            .borrow()
            .as_ref()
            .map(|m| m.id())
            .unwrap_or_default()
    }

    fn set_model(&self, model: &ArtistModel) {
        self.imp().model.replace(Some(model.clone()));
        self.pull_albums();
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
