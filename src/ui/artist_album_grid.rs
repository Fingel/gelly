use gtk::{
    gio,
    glib::{self, Object},
    prelude::*,
    subclass::prelude::*,
};

use crate::{
    models::AlbumModel,
    ui::{album::Album, widget_ext::WidgetApplicationExt},
};

glib::wrapper! {
    pub struct ArtistAlbumGrid(ObjectSubclass<imp::ArtistAlbumGrid>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ArtistAlbumGrid {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn set_albums(&self, albums: &[AlbumModel]) {
        let store = self
            .imp()
            .store
            .get()
            .expect("ArtistAlbumGrid store should be initialized");
        store.remove_all();
        if albums.len() > 1 {
            // looks bad with only 1 album
            store.extend_from_slice(albums);
        }
    }

    fn setup_model(&self) {
        let store = gio::ListStore::new::<AlbumModel>();
        self.imp().flow_box.bind_model(Some(&store), |object| {
            let model = object.downcast_ref::<AlbumModel>().unwrap();
            let album = Album::new();
            album.imp().media_card.set_compact_mode(true);
            album.set_album_model(model);
            album.upcast::<gtk::Widget>()
        });

        self.imp().store.set(store).unwrap();
    }

    fn activate_album(&self, child: &gtk::FlowBoxChild) {
        let Some(album) = child.child().and_downcast::<Album>() else {
            return;
        };
        let model = album.imp().album_model.borrow().clone();
        if let Some(model) = model {
            self.get_root_window().show_album_detail(&model);
        }
    }
}

impl Default for ArtistAlbumGrid {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use std::cell::OnceCell;

    use adw::subclass::prelude::*;
    use gtk::{
        CompositeTemplate, gio,
        glib::{self, subclass::InitializingObject},
    };

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/artist_album_grid.ui")]
    pub struct ArtistAlbumGrid {
        #[template_child]
        pub flow_box: TemplateChild<gtk::FlowBox>,

        pub store: OnceCell<gio::ListStore>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ArtistAlbumGrid {
        const NAME: &'static str = "GellyArtistAlbumGrid";
        type Type = super::ArtistAlbumGrid;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ArtistAlbumGrid {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_model();

            self.flow_box.connect_child_activated(glib::clone!(
                #[weak(rename_to = grid)]
                self.obj(),
                move |_, child| {
                    grid.activate_album(child);
                }
            ));
        }
    }

    impl WidgetImpl for ArtistAlbumGrid {}
    impl BoxImpl for ArtistAlbumGrid {}
}
