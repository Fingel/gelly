use crate::{
    application::Application,
    library_utils::{albums_from_library, play_album},
    models::AlbumModel,
    ui::{album::Album, list_helpers::*, page_traits::TopPage, widget_ext::WidgetApplicationExt},
};
use glib::Object;
use gtk::{
    SortListModel, gio,
    glib::{self},
    prelude::*,
    subclass::prelude::*,
};

glib::wrapper! {
    pub struct AlbumList(ObjectSubclass<imp::AlbumList>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

#[derive(Debug)]
pub enum AlbumSort {
    Name,
    Artist,
    DateAdded,
    Year,
    PlayCount,
}

impl TopPage for AlbumList {
    fn can_search(&self) -> bool {
        true
    }

    fn can_sort(&self) -> bool {
        true
    }

    fn can_new(&self) -> bool {
        false
    }

    fn reveal_search_bar(&self, visible: bool) {
        self.imp().search_bar.set_search_mode(visible);
    }

    fn reveal_sort_bar(&self, visible: bool) {
        self.imp().sort_bar.set_search_mode(visible);
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

    pub fn get_default_sorter(&self) -> gtk::CustomSorter {
        // TODO get saved sort order from settings
        gtk::CustomSorter::new(|obj1, obj2| {
            let album1 = obj1.downcast_ref::<AlbumModel>().unwrap();
            let album2 = obj2.downcast_ref::<AlbumModel>().unwrap();
            album2.date_created().cmp(&album1.date_created()).into()
        })
    }

    pub fn search_changed(&self, query: &str) {
        let imp = self.imp();
        let store = imp.store.get().expect("Store should be initialized");
        let sorter = if let Some(current_sorter) = imp.current_sorter.borrow().as_ref() {
            current_sorter.clone()
        } else {
            let default_sorter = self.get_default_sorter();
            imp.current_sorter.replace(Some(default_sorter.clone()));
            default_sorter
        };
        let filters = vec![
            imp.name_filter
                .get()
                .expect("Name filter should be initialized")
                .clone(),
            imp.artists_filter
                .get()
                .expect("Artists filter should be initialized")
                .clone(),
        ];
        apply_multi_filter_search(query, sorter.upcast(), store, &filters, &imp.grid_view);
    }

    fn handle_sort_changed(&self) {
        let imp = self.imp();
        let sort_option = match imp.sort_dropdown.selected() {
            0 => AlbumSort::DateAdded,
            1 => AlbumSort::Name,
            2 => AlbumSort::Artist,
            3 => AlbumSort::Year,
            4 => AlbumSort::PlayCount,
            _ => AlbumSort::DateAdded,
        };
        let sort_direction = match imp.sort_direction.active() {
            0 => SortDirection::Ascending,
            1 => SortDirection::Descending,
            _ => SortDirection::Ascending,
        };
        self.sort_changed(sort_option, sort_direction);
    }

    fn sort_changed(&self, sort: AlbumSort, direction: SortDirection) {
        let imp = self.imp();
        let sorter = gtk::CustomSorter::new(move |obj1, obj2| {
            let (obj1, obj2) = match direction {
                SortDirection::Ascending => (obj1, obj2),
                SortDirection::Descending => (obj2, obj1),
            };
            let album1 = obj1.downcast_ref::<AlbumModel>().unwrap();
            let album2 = obj2.downcast_ref::<AlbumModel>().unwrap();

            match sort {
                AlbumSort::Name => album1
                    .name()
                    .to_lowercase()
                    .cmp(&album2.name().to_lowercase())
                    .into(),
                AlbumSort::Artist => album1
                    .artists_string()
                    .to_lowercase()
                    .cmp(&album2.artists_string().to_lowercase())
                    .into(),
                AlbumSort::DateAdded => {
                    // Reverse order for newest first
                    album2.date_created().cmp(&album1.date_created()).into()
                }
                AlbumSort::Year => album1.year().cmp(&album2.year()).into(),
                AlbumSort::PlayCount => album1.play_count().cmp(&album2.play_count()).into(),
            }
        });
        imp.current_sorter.replace(Some(sorter.clone()));
        let store = imp.store.get().expect("Store should be initialized");
        let sort_model = SortListModel::new(Some(store.clone()), Some(sorter));
        let selection_model = gtk::SingleSelection::new(Some(sort_model));
        imp.grid_view.set_model(Some(&selection_model));
    }

    fn setup_model(&self) {
        let imp = self.imp();
        let store = gio::ListStore::new::<AlbumModel>();
        let name_filter = create_string_filter::<AlbumModel>("name");
        let artists_filter = create_string_filter::<AlbumModel>("artists-string");
        imp.store
            .set(store.clone())
            .expect("Store should only be set once");
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
                .downcast_ref::<gtk::ListItem>()
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

    use std::cell::{OnceCell, RefCell};

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
        #[template_child]
        pub sort_bar: TemplateChild<gtk::SearchBar>,
        #[template_child]
        pub sort_dropdown: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub sort_direction: TemplateChild<adw::ToggleGroup>,

        pub store: OnceCell<gio::ListStore>,
        pub name_filter: OnceCell<gtk::StringFilter>,
        pub artists_filter: OnceCell<gtk::StringFilter>,
        pub current_sorter: RefCell<Option<gtk::CustomSorter>>,
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

            self.sort_dropdown.connect_selected_notify(glib::clone!(
                #[weak(rename_to = album_list)]
                self.obj(),
                move |_| {
                    album_list.handle_sort_changed();
                }
            ));

            self.sort_direction.connect_active_notify(glib::clone!(
                #[weak(rename_to = album_list)]
                self.obj(),
                move |_| {
                    album_list.handle_sort_changed();
                }
            ));
        }
    }

    impl WidgetImpl for AlbumList {}
    impl BoxImpl for AlbumList {}
}
