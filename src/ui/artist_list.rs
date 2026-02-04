use crate::{
    application::Application,
    config,
    library_utils::{artists_from_library, play_artist},
    models::ArtistModel,
    ui::{artist::Artist, list_helpers::*, page_traits::TopPage, widget_ext::WidgetApplicationExt},
};
use glib::Object;
use gtk::{
    CustomSorter, SingleSelection, SortListModel, gio,
    glib::{self},
    prelude::*,
    subclass::prelude::*,
};
use num_enum::TryFromPrimitive;

glib::wrapper! {
    pub struct ArtistList(ObjectSubclass<imp::ArtistList>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

#[derive(Debug, TryFromPrimitive)]
#[repr(u32)]
pub enum ArtistSort {
    Name = 0,
    PlayCount = 1,
}

impl TopPage for ArtistList {
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
            && let Some(artist_model) = selected_item.downcast_ref::<ArtistModel>()
        {
            play_artist(&artist_model.id(), &self.get_application());
        }
    }
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
            store.extend_from_slice(&artists);
            self.apply_sorting();
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

    pub fn search_changed(&self, query: &str) {
        let imp = self.imp();
        let store = imp.store.get().expect("Store should be initialized");
        let name_filter = imp
            .name_filter
            .get()
            .expect("Name filter should be initialized");
        let sorter = if let Some(current_sorter) = imp.current_sorter.borrow().as_ref() {
            current_sorter.clone()
        } else {
            let default_sorter = self.build_sorter(ArtistSort::Name, SortDirection::Ascending);
            imp.current_sorter.replace(Some(default_sorter.clone()));
            default_sorter
        };
        apply_single_filter_search(query, sorter.upcast(), store, name_filter, &imp.grid_view);
    }

    fn sort_changed(&self) {
        config::set_artists_sort_by(self.imp().sort_dropdown.selected());
        config::set_artists_sort_direction(self.imp().sort_direction.active());
        self.apply_sorting();
    }

    fn apply_sorting(&self) {
        let imp = self.imp();
        let sort_option = ArtistSort::try_from_primitive(imp.sort_dropdown.selected())
            .unwrap_or(ArtistSort::Name);
        let sort_direction = SortDirection::try_from_primitive(imp.sort_direction.active())
            .unwrap_or(SortDirection::Ascending);
        let sorter = self.build_sorter(sort_option, sort_direction);
        imp.current_sorter.replace(Some(sorter.clone()));
        let store = imp.store.get().expect("Store should be initialized");
        let sort_model = SortListModel::new(Some(store.clone()), Some(sorter));
        let selection_model = SingleSelection::new(Some(sort_model));
        imp.grid_view.set_model(Some(&selection_model));
    }

    fn build_sorter(&self, sort: ArtistSort, direction: SortDirection) -> CustomSorter {
        CustomSorter::new(move |obj1, obj2| {
            let (obj1, obj2) = match direction {
                SortDirection::Ascending => (obj1, obj2),
                SortDirection::Descending => (obj2, obj1),
            };
            let artist1 = obj1.downcast_ref::<ArtistModel>().unwrap();
            let artist2 = obj2.downcast_ref::<ArtistModel>().unwrap();

            match sort {
                ArtistSort::Name => artist1
                    .name()
                    .to_lowercase()
                    .cmp(&artist2.name().to_lowercase())
                    .into(),
                ArtistSort::PlayCount => artist1.play_count().cmp(&artist2.play_count()).into(),
            }
        })
    }

    fn setup_model(&self) {
        let imp = self.imp();
        let store = gio::ListStore::new::<ArtistModel>();
        let name_filter = create_string_filter::<ArtistModel>("name");
        imp.store
            .set(store.clone())
            .expect("Store should only be set once");
        imp.name_filter
            .set(name_filter)
            .expect("Name filter should only be set once");

        let selection_model = gtk::SingleSelection::new(Some(store));
        let factory = gtk::SignalListItemFactory::new();

        factory.connect_setup(move |_, list_item| {
            let placeholder = Artist::new();
            let item = list_item
                .downcast_ref::<gtk::ListItem>()
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
    use std::cell::{OnceCell, RefCell};

    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{CompositeTemplate, gio, glib, prelude::*};

    use crate::config;

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
        #[template_child]
        pub sort_bar: TemplateChild<gtk::SearchBar>,
        #[template_child]
        pub sort_dropdown: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub sort_direction: TemplateChild<adw::ToggleGroup>,

        pub store: OnceCell<gio::ListStore>,
        pub name_filter: OnceCell<gtk::StringFilter>,
        pub current_sorter: RefCell<Option<gtk::CustomSorter>>,
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
            self.sort_dropdown
                .set_selected(config::get_artists_sort_by());
            self.sort_direction
                .set_active(config::get_artists_sort_direction());

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

            self.sort_dropdown.connect_selected_notify(glib::clone!(
                #[weak(rename_to = artist_list)]
                self.obj(),
                move |_| {
                    artist_list.sort_changed();
                }
            ));

            self.sort_direction.connect_active_notify(glib::clone!(
                #[weak(rename_to = artist_list)]
                self.obj(),
                move |_| {
                    artist_list.sort_changed();
                }
            ));
        }
    }
    impl WidgetImpl for ArtistList {}
    impl BoxImpl for ArtistList {}
}
