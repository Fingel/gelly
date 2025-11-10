use crate::{
    application::Application,
    library_utils::albums_from_library,
    models::AlbumModel,
    ui::{album::Album, widget_ext::WidgetApplicationExt, window::Window},
};
use glib::Object;
use gtk::{
    FilterListModel, ListItem, PropertyExpression, StringFilter, gio,
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
        let grid_view = &self.imp().grid_view;
        let selection_model = grid_view
            .model()
            .expect("GridView should have a model")
            .downcast::<gtk::SingleSelection>()
            .expect("Model should be a SingleSelection");
        let current_model = selection_model
            .model()
            .expect("SelectionModel should have a model");
        let album_model = current_model
            .item(index)
            .expect("Item index invalid")
            .downcast_ref::<AlbumModel>()
            .expect("Item should be an AlbumModel")
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
                move |_app: Application, _total_record_count: u64| {
                    album_list.pull_albums();
                }
            ),
        );
    }

    pub fn search_changed(&self, query: &str) {
        let imp = self.imp();
        let store = imp.store.get().expect("Store should be initialized");

        if query.is_empty() {
            let selection_model = gtk::SingleSelection::new(Some(store.clone()));
            imp.grid_view.set_model(Some(&selection_model));
        } else {
            let name_filter = imp
                .name_filter
                .get()
                .expect("Name filter should be initialized");
            let artists_filter = imp
                .artists_filter
                .get()
                .expect("Name filter should be initialized");

            name_filter.set_search(Some(query));
            artists_filter.set_search(Some(query));

            let any_filter = gtk::AnyFilter::new();
            any_filter.append(name_filter.clone());
            any_filter.append(artists_filter.clone());

            let filter_model = FilterListModel::new(Some(store.clone()), Some(any_filter));
            let selection_model = gtk::SingleSelection::new(Some(filter_model));

            imp.grid_view.set_model(Some(&selection_model));
        }
    }

    pub fn setup_search_connection(&self) {
        let window = self.get_root_window();

        window.connect_closure(
            "search",
            false,
            glib::closure_local!(
                #[weak(rename_to = album_list)]
                self,
                move |_: Window| {
                    album_list.imp().search_bar.set_search_mode(true);
                }
            ),
        );
    }

    fn setup_model(&self) {
        let imp = self.imp();
        let store = gio::ListStore::new::<AlbumModel>();
        let name_expression =
            PropertyExpression::new(AlbumModel::static_type(), None::<&gtk::Expression>, "name");
        let name_filter = StringFilter::new(Some(name_expression));
        name_filter.set_ignore_case(true);
        name_filter.set_match_mode(gtk::StringFilterMatchMode::Substring);

        let artists_expression = PropertyExpression::new(
            AlbumModel::static_type(),
            None::<&gtk::Expression>,
            "artists-string",
        );
        let artists_filter = StringFilter::new(Some(artists_expression));
        artists_filter.set_ignore_case(true);
        artists_filter.set_match_mode(gtk::StringFilterMatchMode::Substring);

        imp.store
            .set(store.clone())
            .expect("AlbumList store should only be set once.");
        imp.name_filter
            .set(name_filter)
            .expect("Name filter should only be set once");
        imp.artists_filter
            .set(artists_filter)
            .expect("Artists filter should only be set once");

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
    use gtk::{CompositeTemplate, gio, glib, prelude::*};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/album_list.ui")]
    pub struct AlbumList {
        #[template_child]
        pub grid_view: TemplateChild<gtk::GridView>,
        #[template_child]
        pub empty: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub search_bar: TemplateChild<gtk::SearchBar>,
        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,

        pub store: OnceCell<gio::ListStore>,
        pub name_filter: OnceCell<gtk::StringFilter>,
        pub artists_filter: OnceCell<gtk::StringFilter>,
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

            self.search_entry.connect_search_changed(glib::clone!(
                #[weak(rename_to = album_list)]
                self.obj(),
                move |entry| {
                    album_list.search_changed(&entry.text());
                }
            ));

            self.obj().connect_realize(glib::clone!(
                #[weak (rename_to = album_list)]
                self.obj(),
                move |_| {
                    album_list.setup_search_connection();
                }
            ));
        }
    }
    impl WidgetImpl for AlbumList {}
    impl BoxImpl for AlbumList {}
}
