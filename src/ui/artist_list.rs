use crate::{
    application::Application,
    library_utils::artists_from_library,
    models::ArtistModel,
    ui::{artist::Artist, widget_ext::WidgetApplicationExt, window::Window},
};
use glib::Object;
use gtk::{
    FilterListModel, ListItem, PropertyExpression, StringFilter, gio,
    glib::{self, object::Cast},
    prelude::*,
    subclass::prelude::*,
};

glib::wrapper! {
    pub struct ArtistList(ObjectSubclass<imp::ArtistList>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ArtistList {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn pull_artists(&self) {
        let library = self.get_application().library().clone();
        let artists = artists_from_library(&library.borrow());
        if artists.is_empty() {
            self.set_empty(true);
        } else {
            self.set_empty(false);
            let store = self
                .imp()
                .store
                .get()
                .expect("AlbumList store should be initialized.");
            store.remove_all();
            for artist in artists {
                store.append(&artist);
            }
        }
    }

    pub fn activate_artist(&self, index: u32) {
        let grid_view = &self.imp().grid_view;
        let selection_model = grid_view
            .model()
            .expect("GridView should have a model")
            .downcast::<gtk::SingleSelection>()
            .expect("Model should be a SingleSelection");
        let current_model = selection_model
            .model()
            .expect("SelectionModel should have a model");
        let artist_model = current_model
            .item(index)
            .expect("Item index invalid")
            .downcast_ref::<ArtistModel>()
            .expect("Item should be an ArtistModel")
            .clone();
        let window = self.get_root_window();
        window.show_artist_detail(&artist_model);
    }

    pub fn setup_library_connection(&self) {
        let app = self.get_application();
        app.connect_closure(
            "library-refreshed",
            false,
            glib::closure_local!(
                #[weak(rename_to = artist_list)]
                self,
                move |_app: Application, _total_record_count: u64| {
                    artist_list.pull_artists();
                }
            ),
        );
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
            name_filter.set_search(Some(query));
            let filter_model = FilterListModel::new(Some(store.clone()), Some(name_filter.clone()));
            let selection_model = gtk::SingleSelection::new(Some(filter_model));
            imp.grid_view.set_model(Some(&selection_model));
        }
    }

    fn setup_model(&self) {
        let imp = self.imp();
        let store = gio::ListStore::new::<ArtistModel>();
        let name_expression =
            PropertyExpression::new(ArtistModel::static_type(), None::<&gtk::Expression>, "name");
        let name_filter = StringFilter::new(Some(name_expression));
        name_filter.set_ignore_case(true);
        name_filter.set_match_mode(gtk::StringFilterMatchMode::Substring);

        imp.store
            .set(store.clone())
            .expect("ArtistList store should only be set once.");
        imp.name_filter
            .set(name_filter)
            .expect("Name filter should only be set once.");

        let selection_model = gtk::SingleSelection::new(Some(store));
        let factory = gtk::SignalListItemFactory::new();

        factory.connect_setup(move |_, list_item| {
            let placeholder = Artist::new();
            let item = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be a ListItem");
            item.set_child(Some(&placeholder))
        });

        factory.connect_bind(move |_, list_item| {
            let list_item = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be a ListItem");
            let artist_model = list_item
                .item()
                .and_downcast::<ArtistModel>()
                .expect("Item should be an ArtistData");
            let artist_widget = list_item
                .child()
                .and_downcast::<Artist>()
                .expect("Child has to be an Artist");

            artist_widget.set_artist_model(&artist_model);
        });

        imp.grid_view.set_model(Some(&selection_model));
        imp.grid_view.set_factory(Some(&factory));
    }

    fn set_empty(&self, empty: bool) {
        self.imp().empty.set_visible(empty);
        self.imp().grid_view.set_visible(!empty);
    }
}

impl Default for ArtistList {
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
    #[template(resource = "/io/m51/Gelly/ui/artist_list.ui")]
    pub struct ArtistList {
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
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ArtistList {
        const NAME: &'static str = "GellyArtistList";
        type Type = super::ArtistList;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ArtistList {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_model();

            self.grid_view.connect_activate(glib::clone!(
                #[weak(rename_to = artist_list)]
                self.obj(),
                move |_, position| {
                    artist_list.activate_artist(position);
                }
            ));

            self.search_entry.connect_search_changed(glib::clone!(
                #[weak(rename_to = artist_list)]
                self.obj(),
                move |entry| {
                    artist_list.search_changed(&entry.text());
                }
            ));

            self.obj().connect_realize(glib::clone!(
                #[weak (rename_to = artist_list)]
                self.obj(),
                move |_| {
                    artist_list.setup_search_connection();
                }
            ));
        }
    }
    impl WidgetImpl for ArtistList {}
    impl BoxImpl for ArtistList {}
}
