use crate::{
    library_utils::albums_for_artist,
    models::{AlbumModel, ArtistModel},
    ui::{album_detail::AlbumDetail, widget_ext::WidgetApplicationExt},
};
use glib::Object;
use gtk::{gio, glib, prelude::*, subclass::prelude::*};

glib::wrapper! {
    pub struct ArtistDetail(ObjectSubclass<imp::ArtistDetail>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup;
}

impl ArtistDetail {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn set_artist_model(&self, artist_model: &ArtistModel) {
        let imp = self.imp();
        imp.artist_label.set_label(&artist_model.name());
        imp.artist_id.replace(artist_model.id());
        self.pull_albums();
    }

    pub fn pull_albums(&self) {
        let library = self.get_application().library().clone();
        let albums: Vec<AlbumModel> =
            albums_for_artist(&self.imp().artist_id.borrow(), &library.borrow());
        self.imp().albums.replace(albums);
        while let Some(child) = self.imp().albums_box.first_child() {
            self.imp().albums_box.remove(&child);
        }
        for album in self.imp().albums.borrow().iter() {
            let album_widget = AlbumDetail::new();
            self.imp().albums_box.append(&album_widget);
            album_widget.set_album_model(album);
            album_widget.imp().artist_label.set_label("");
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

    use crate::models::AlbumModel;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/artist_detail.ui")]
    pub struct ArtistDetail {
        #[template_child]
        pub artist_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub albums_box: TemplateChild<gtk::Box>,

        pub artist_id: RefCell<String>,
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
