use crate::{
    application::Application,
    config,
    library_utils::play_artist,
    models::ArtistModel,
    ui::{
        artist::Artist,
        list_helpers::{create_string_filter, handle_grid_activation},
        page_traits::{SortDirection, SortType, TopPage},
        widget_ext::WidgetApplicationExt,
    },
};
use glib::Object;
use gtk::{
    gio,
    glib::{self},
    prelude::*,
    subclass::prelude::*,
};

glib::wrapper! {
    pub struct ArtistList(ObjectSubclass<imp::ArtistList>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl TopPage for ArtistList {
    fn can_new(&self) -> bool {
        false
    }

    fn play_selected(&self) {
        if let Some(selection) = self.imp().grid_view.model()
            && let Some(single_selection) = selection.downcast_ref::<gtk::SingleSelection>()
            && let Some(selected_item) = single_selection.selected_item()
            && let Some(artist_model) = selected_item.downcast_ref::<ArtistModel>()
        {
            play_artist(&artist_model.id(), &self.get_application());
        }
    }

    fn search_changed(&self, query: &str) {
        let search = if query.is_empty() { None } else { Some(query) };
        self.imp().name_filter.get().unwrap().set_search(search);
    }

    fn sort_options(&self) -> &[SortType] {
        &[SortType::Name, SortType::PlayCount]
    }

    fn current_sort_by(&self) -> u32 {
        config::get_artists_sort_by()
    }

    fn current_sort_direction(&self) -> u32 {
        config::get_artists_sort_direction()
    }

    fn apply_sort(&self, sort_by: u32, direction: u32) {
        config::set_artists_sort_by(sort_by);
        config::set_artists_sort_direction(direction);
        self.imp().sort_state.set((sort_by, direction));
        self.imp().sorter.get().unwrap().changed(gtk::SorterChange::Different);
    }

    fn filter_favorites(&self, active: bool) {
        let filter = self.imp().favorites_filter.get().unwrap();
        if active {
            filter.set_filter_func(|obj| {
                obj.downcast_ref::<ArtistModel>().is_some_and(|m| m.favorite())
            });
        } else {
            filter.unset_filter_func();
        }
    }
}

impl ArtistList {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn pull_artists(&self) {
        let artists = self.get_application().library().artists_from_library();
        if artists.is_empty() {
            self.set_empty(true);
        } else {
            self.set_empty(false);
            let store = self
                .imp()
                .store
                .get()
                .expect("ArtistList store should be initialized.");
            store.remove_all();
            store.extend_from_slice(&artists);
            self.apply_sort(self.current_sort_by(), self.current_sort_direction());
        }
    }

    pub fn activate_artist(&self, index: u32) {
        let window = self.get_root_window();
        handle_grid_activation::<ArtistModel, _>(&self.imp().grid_view, index, |artist_model| {
            window.show_artist_detail(artist_model);
        });
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

    fn build_sorter(&self) -> gtk::CustomSorter {
        let sort_state = self.imp().sort_state.clone();
        let options: Vec<SortType> = self.sort_options().to_vec();
        gtk::CustomSorter::new(move |a, b| {
            let (sort_by, direction_raw) = sort_state.get();
            let direction =
                SortDirection::try_from(direction_raw).unwrap_or(SortDirection::Ascending);
            let (a, b) = match direction {
                SortDirection::Ascending => (a, b),
                SortDirection::Descending => (b, a),
            };
            let a = a.downcast_ref::<ArtistModel>().unwrap();
            let b = b.downcast_ref::<ArtistModel>().unwrap();
            match options[sort_by as usize] {
                SortType::Name => a.name().to_lowercase().cmp(&b.name().to_lowercase()).into(),
                SortType::PlayCount => a.play_count().cmp(&b.play_count()).into(),
                _ => gtk::Ordering::Equal,
            }
        })
    }

    fn setup_model(&self) {
        let store = gio::ListStore::new::<ArtistModel>();

        let favorites_filter = gtk::CustomFilter::new(|_| true);
        let fav_model =
            gtk::FilterListModel::new(Some(store.clone()), Some(favorites_filter.clone()));

        let name_filter = create_string_filter::<ArtistModel>("name");
        let search_model =
            gtk::FilterListModel::new(Some(fav_model), Some(name_filter.clone()));

        let sorter = self.build_sorter();
        let sort_model = gtk::SortListModel::new(Some(search_model), Some(sorter.clone()));
        let selection = gtk::SingleSelection::new(Some(sort_model));

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            let item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
            item.set_child(Some(&Artist::new()));
        });
        factory.connect_bind(move |_, list_item| {
            let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
            let artist_model = list_item.item().and_downcast::<ArtistModel>().unwrap();
            let artist_widget = list_item.child().and_downcast::<Artist>().unwrap();
            artist_widget.set_artist_model(&artist_model);
        });

        let imp = self.imp();
        imp.grid_view.set_model(Some(&selection));
        imp.grid_view.set_factory(Some(&factory));
        imp.store.set(store).unwrap();
        imp.favorites_filter.set(favorites_filter).unwrap();
        imp.name_filter.set(name_filter).unwrap();
        imp.sorter.set(sorter).unwrap();
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
    use std::cell::{Cell, OnceCell};
    use std::rc::Rc;

    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{CompositeTemplate, gio, glib};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/artist_list.ui")]
    pub struct ArtistList {
        #[template_child]
        pub grid_view: TemplateChild<gtk::GridView>,
        #[template_child]
        pub empty: TemplateChild<adw::StatusPage>,

        pub store: OnceCell<gio::ListStore>,
        pub favorites_filter: OnceCell<gtk::CustomFilter>,
        pub name_filter: OnceCell<gtk::StringFilter>,
        pub sorter: OnceCell<gtk::CustomSorter>,
        pub sort_state: Rc<Cell<(u32, u32)>>,
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
        }
    }

    impl WidgetImpl for ArtistList {}
    impl BoxImpl for ArtistList {}
}
