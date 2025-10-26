use crate::{
    application::Application,
    library_utils::albums_from_library,
    models::AlbumModel,
    ui::{album::Album, widget_ext::WidgetApplicationExt},
};
use glib::Object;
use gtk::{
    ListItem, gio,
    glib::{self, object::Cast},
    prelude::*,
    subclass::prelude::*,
};

glib::wrapper! {
    pub struct AlbumList(ObjectSubclass<imp::AlbumList>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl AlbumList {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn pull_albums(&self) {
        let library = self.get_application().library().clone();
        let albums = albums_from_library(&library.borrow());
        if albums.is_empty() {
            self.set_empty(true);
        } else {
            self.set_empty(false);
            let store = self
                .imp()
                .store
                .get()
                .expect("AlbumList store should be initialized.");
            store.remove_all();
            for album in albums {
                store.append(&album);
            }
        }
    }

    pub fn activate_album(&self, index: u32) {
        let store = self
            .imp()
            .store
            .get()
            .expect("AlbumList store should be initialized.");
        let album_model = store
            .item(index)
            .expect("Item index invalid")
            .downcast_ref::<AlbumModel>()
            .expect("Item should be an AlbumData")
            .clone();
        let window = self.get_root_window();
        window.show_album_detail(&album_model);
    }

    pub fn setup_library_connection(&self) {
        let app = self.get_application();
        app.connect_closure(
            "library-refreshed",
            false,
            glib::closure_local!(
                #[weak(rename_to = album_list)]
                self,
                move |_app: Application| {
                    album_list.pull_albums();
                }
            ),
        );
    }

    fn setup_model(&self) {
        let imp = self.imp();
        let store = gio::ListStore::new::<AlbumModel>();
        imp.store
            .set(store.clone())
            .expect("AlbumList store should only be set once.");

        let selection_model = gtk::SingleSelection::new(Some(store));
        let factory = gtk::SignalListItemFactory::new();

        factory.connect_setup(move |_, list_item| {
            let placeholder = Album::new();
            let item = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be a ListItem");
            item.set_child(Some(&placeholder))
        });

        factory.connect_bind(move |_, list_item| {
            let list_item = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be a ListItem");
            let album_model = list_item
                .item()
                .and_downcast::<AlbumModel>()
                .expect("Item should be an AlbumData");
            let album_widget = list_item
                .child()
                .and_downcast::<Album>()
                .expect("Child has to be an Album");

            album_widget.set_album_model(&album_model);
        });

        imp.grid_view.set_model(Some(&selection_model));
        imp.grid_view.set_factory(Some(&factory));
    }

    fn set_empty(&self, empty: bool) {
        self.imp().empty.set_visible(empty);
        self.imp().grid_view.set_visible(!empty);
    }
}

impl Default for AlbumList {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {

    use std::cell::OnceCell;

    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{CompositeTemplate, gio, glib};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/album_list.ui")]
    pub struct AlbumList {
        #[template_child]
        pub grid_view: TemplateChild<gtk::GridView>,
        #[template_child]
        pub empty: TemplateChild<adw::StatusPage>,

        pub store: OnceCell<gio::ListStore>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AlbumList {
        const NAME: &'static str = "GellyAlbumList";
        type Type = super::AlbumList;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AlbumList {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_model();

            self.grid_view.connect_activate(glib::clone!(
                #[weak(rename_to = album_list)]
                self.obj(),
                move |_, position| {
                    album_list.activate_album(position);
                }
            ));
        }
    }
    impl WidgetImpl for AlbumList {}
    impl BoxImpl for AlbumList {}
}
