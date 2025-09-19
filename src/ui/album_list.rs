use crate::{
    library_utils::albums_from_library,
    models::album_data::AlbumData,
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
        @implements gio::ActionMap, gio::ActionGroup;
}

impl AlbumList {
    pub fn new() -> Self {
        Object::builder().build()
    }
    pub fn pull_albums(&self) {
        let library = self.get_application().library().clone();
        let albums = albums_from_library(&library.borrow());
        dbg!(albums);
    }

    fn setup_model(&self) {
        let imp = self.imp();
        let store = gio::ListStore::new::<AlbumData>();
        let dummy_albums = vec![
            AlbumData::new(
                "Album 1",
                "Album_id_1",
                vec![String::from("Artist 1")],
                "date_created_1",
                "image_1",
            ),
            AlbumData::new(
                "Album 2",
                "Album_id_2",
                vec![String::from("Artist 2")],
                "date_created_2",
                "image_2",
            ),
        ];
        for a in dummy_albums {
            store.append(&a);
        }

        let selection_model = gtk::SingleSelection::new(Some(store));
        let factory = gtk::SignalListItemFactory::new();

        factory.connect_setup(move |_, list_item| {
            let placeholder = Album::new();
            placeholder.set_album_name("Placeholder");
            placeholder.set_artist_name("Placeholder Artist");
            let item = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be a ListItem");
            item.set_child(Some(&placeholder))
        });

        factory.connect_bind(move |_, list_item| {
            let list_item = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be a ListItem");
            let album_data = list_item
                .item()
                .and_downcast::<AlbumData>()
                .expect("Item should be an AlbumData");
            let album_widget = list_item
                .child()
                .and_downcast::<Album>()
                .expect("Child has to be an Album");

            album_widget.set_album_name(&album_data.name());
            album_widget.set_artist_name(&album_data.primary_artist());
        });

        imp.grid_view.set_model(Some(&selection_model));
        imp.grid_view.set_factory(Some(&factory));
    }
}

impl Default for AlbumList {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {

    use crate::{application::Application, ui::widget_ext::WidgetApplicationExt};
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib::{self, object::ObjectExt},
        prelude::WidgetExt,
    };

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/album_list.ui")]
    pub struct AlbumList {
        #[template_child]
        pub grid_view: TemplateChild<gtk::GridView>,
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

            self.obj().connect_map(glib::clone!(
                #[weak(rename_to = album_list)]
                self.obj(),
                move |_| {
                    let app = album_list.get_application();
                    app.connect_closure(
                        "library-refreshed",
                        false,
                        glib::closure_local!(
                            #[weak]
                            album_list,
                            move |_app: Application| {
                                album_list.pull_albums();
                            }
                        ),
                    );
                }
            ));
        }
    }
    impl WidgetImpl for AlbumList {}
    impl BoxImpl for AlbumList {}
}
