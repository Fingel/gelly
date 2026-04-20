use crate::{
    application::Application,
    config,
    library_utils::play_album,
    models::AlbumModel,
    ui::{
        album::Album,
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
    pub struct AlbumList(ObjectSubclass<imp::AlbumList>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl TopPage for AlbumList {
    fn can_new(&self) -> bool {
        false
    }

    fn play_selected(&self) {
        if let Some(selection) = self.imp().grid_view.model()
            && let Some(single_selection) = selection.downcast_ref::<gtk::SingleSelection>()
            && let Some(selected_item) = single_selection.selected_item()
            && let Some(album_model) = selected_item.downcast_ref::<AlbumModel>()
        {
            play_album(&album_model.id(), &self.get_application());
        }
    }

    fn search_changed(&self, query: &str) {
        let search = if query.is_empty() { None } else { Some(query) };
        let imp = self.imp();
        imp.name_filter.get().unwrap().set_search(search);
        imp.artists_filter.get().unwrap().set_search(search);
    }

    fn sort_options(&self) -> &[SortType] {
        &[
            SortType::DateAdded,
            SortType::Name,
            SortType::Artist,
            SortType::Year,
            SortType::PlayCount,
        ]
    }

    fn current_sort_by(&self) -> u32 {
        config::get_albums_sort_by()
    }

    fn current_sort_direction(&self) -> u32 {
        config::get_albums_sort_direction()
    }

    fn apply_sort(&self, sort_by: u32, direction: u32) {
        config::set_albums_sort_by(sort_by);
        config::set_albums_sort_direction(direction);
        self.imp().sort_state.set((sort_by, direction));
        self.imp()
            .sorter
            .get()
            .unwrap()
            .changed(gtk::SorterChange::Different);
        self.reset_position();
    }

    fn filter_favorites(&self, active: bool) {
        let filter = self.imp().favorites_filter.get().unwrap();
        if active {
            filter.set_filter_func(|obj| {
                obj.downcast_ref::<AlbumModel>()
                    .is_some_and(|m| m.favorite())
            });
        } else {
            filter.unset_filter_func();
        }
    }

    fn reset_position(&self) {
        let imp = self.imp();
        if imp.grid_view.model().is_some_and(|m| m.n_items() > 0) {
            imp.grid_view
                .scroll_to(0, gtk::ListScrollFlags::NONE, None::<gtk::ScrollInfo>);
        }
    }
}

impl AlbumList {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn pull_albums(&self) {
        let albums = self.get_application().library().albums_from_library();
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
            store.extend_from_slice(&albums);
            self.apply_sort(self.current_sort_by(), self.current_sort_direction());
        }
    }

    pub fn activate_album(&self, index: u32) {
        let window = self.get_root_window();
        handle_grid_activation::<AlbumModel, _>(&self.imp().grid_view, index, |album_model| {
            window.show_album_detail(album_model);
        });
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
            let a = a.downcast_ref::<AlbumModel>().unwrap();
            let b = b.downcast_ref::<AlbumModel>().unwrap();
            match options[sort_by as usize] {
                SortType::Name => a.name().to_lowercase().cmp(&b.name().to_lowercase()).into(),
                SortType::Artist => a
                    .artists_string()
                    .to_lowercase()
                    .cmp(&b.artists_string().to_lowercase())
                    .into(),
                SortType::DateAdded => b.date_created().cmp(&a.date_created()).into(), // Reverse order for newest first
                SortType::Year => a.year().cmp(&b.year()).into(),
                SortType::PlayCount => a.play_count().cmp(&b.play_count()).into(),
                _ => gtk::Ordering::Equal,
            }
        })
    }

    fn setup_model(&self) {
        let store = gio::ListStore::new::<AlbumModel>();

        let favorites_filter = gtk::CustomFilter::new(|_| true);
        let fav_model =
            gtk::FilterListModel::new(Some(store.clone()), Some(favorites_filter.clone()));

        let name_filter = create_string_filter::<AlbumModel>("name");
        let artists_filter = create_string_filter::<AlbumModel>("artists-string");
        let search_filter = gtk::AnyFilter::new();
        search_filter.append(name_filter.clone());
        search_filter.append(artists_filter.clone());
        let search_model = gtk::FilterListModel::new(Some(fav_model), Some(search_filter));

        let sorter = self.build_sorter();
        let sort_model = gtk::SortListModel::new(Some(search_model), Some(sorter.clone()));
        let selection = gtk::SingleSelection::new(Some(sort_model));

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            let item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
            item.set_child(Some(&Album::new()));
        });
        factory.connect_bind(move |_, list_item| {
            let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
            let album_model = list_item.item().and_downcast::<AlbumModel>().unwrap();
            let album_widget = list_item.child().and_downcast::<Album>().unwrap();
            album_widget.set_album_model(&album_model);
        });

        let imp = self.imp();
        imp.grid_view.set_model(Some(&selection));
        imp.grid_view.set_factory(Some(&factory));
        imp.store.set(store).unwrap();
        imp.favorites_filter.set(favorites_filter).unwrap();
        imp.name_filter.set(name_filter).unwrap();
        imp.artists_filter.set(artists_filter).unwrap();
        imp.sorter.set(sorter).unwrap();
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
    use std::cell::{Cell, OnceCell};
    use std::rc::Rc;

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
        pub favorites_filter: OnceCell<gtk::CustomFilter>,
        pub name_filter: OnceCell<gtk::StringFilter>,
        pub artists_filter: OnceCell<gtk::StringFilter>,
        pub sorter: OnceCell<gtk::CustomSorter>,
        pub sort_state: Rc<Cell<(u32, u32)>>,
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
