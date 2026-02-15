use gtk::{
    CustomSorter, FilterListModel, SortListModel, gio,
    glib::{self, Object},
    prelude::*,
    subclass::prelude::*,
};
use log::warn;

use crate::{
    application::Application,
    library_utils::all_songs,
    models::SongModel,
    ui::{
        list_helpers::{SortDirection, create_string_filter},
        page_traits::TopPage,
        song::Song,
        song_utils,
        widget_ext::WidgetApplicationExt,
    },
};
use num_enum::TryFromPrimitive;

glib::wrapper! {
    pub struct SongList(ObjectSubclass<imp::SongList>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

#[derive(Debug, TryFromPrimitive)]
#[repr(u32)]
pub enum SongSort {
    DateAdded = 0,
    Name = 1,
    Artist = 2,
    Album = 3,
}

impl TopPage for SongList {
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
        if let Some(selection) = self.imp().track_list.model()
            && let Some(single_selection) = selection.downcast_ref::<gtk::SingleSelection>()
        {
            self.activate_song(single_selection.selected() as usize);
        }
    }
}

impl SongList {
    pub fn new() -> Self {
        Object::builder().build()
    }

    fn is_loaded(&self) -> bool {
        self.imp()
            .store
            .get()
            .map(|store| store.n_items() > 0)
            .unwrap_or(false)
    }

    pub fn pull_songs(&self) {
        // Ensure factory is set up (will only happen once, after widget is attached)
        self.setup_factory();

        let library = self.get_application().library().clone();
        let songs = all_songs(&library.borrow());
        if songs.is_empty() {
            self.set_empty(true);
        } else {
            self.set_empty(false);
            let store = self
                .imp()
                .store
                .get()
                .expect("SongList store should be initialized");
            store.remove_all();
            store.extend_from_slice(&songs);
            self.bind_song_count(store);
        }
    }

    pub fn activate_song(&self, index: usize) {
        let library = self.get_application().library().clone();
        let songs = all_songs(&library.borrow());
        if let Some(audio_model) = self.get_application().audio_model() {
            audio_model.set_queue(songs, index);
        } else {
            self.toast("Audio model not initialized, please restart", None);
            warn!("No audio model found");
        }
    }

    pub fn setup_library_connection(&self) {
        let app = self.get_application();
        app.connect_closure(
            "library-refreshed",
            false,
            glib::closure_local!(
                #[weak(rename_to = song_list)]
                self,
                move |_app: Application, _total_record_count: u64| {
                    // Only refresh if songs have already been loaded once to prevent lag on startup
                    if song_list.is_loaded() {
                        song_list.pull_songs();
                    }
                }
            ),
        );
    }

    pub fn search_changed(&self, query: &str) {
        let imp = self.imp();
        let store = imp.store.get().expect("Store should be initialized");
        let sorter = if let Some(current_sorter) = imp.current_sorter.borrow().as_ref() {
            current_sorter.clone()
        } else {
            let default_sorter = self.build_sorter(SongSort::DateAdded, SortDirection::Ascending);
            imp.current_sorter.replace(Some(default_sorter.clone()));
            default_sorter
        };
        let filters = vec![
            imp.name_filter
                .get()
                .expect("Name filter should be initialized")
                .clone(),
            imp.artist_filter
                .get()
                .expect("Artist filter should be initialized")
                .clone(),
        ];
        if query.is_empty() {
            let sort_model = SortListModel::new(Some(store.clone()), Some(sorter));
            self.bind_song_count(&sort_model);
            let selection_model = gtk::SingleSelection::new(Some(sort_model));
            imp.track_list.set_model(Some(&selection_model));
        } else {
            let any_filter = gtk::AnyFilter::new();
            for filter in filters {
                filter.set_search(Some(query));
                any_filter.append(filter.clone());
            }

            let filter_model = FilterListModel::new(Some(store.clone()), Some(any_filter));
            let sort_model = SortListModel::new(Some(filter_model), Some(sorter));
            self.bind_song_count(&sort_model);
            let selection_model = gtk::SingleSelection::new(Some(sort_model));
            imp.track_list.set_model(Some(&selection_model));
        }
    }

    fn apply_sorting(&self) {
        let imp = self.imp();
        let sort_option = SongSort::try_from_primitive(imp.sort_dropdown.selected())
            .unwrap_or(SongSort::DateAdded);
        let sort_direction = SortDirection::try_from_primitive(imp.sort_direction.active())
            .unwrap_or(SortDirection::Ascending);
        let sorter = self.build_sorter(sort_option, sort_direction);
        imp.current_sorter.replace(Some(sorter.clone()));
        let store = imp.store.get().expect("Store should be initialized");
        let sort_model = SortListModel::new(Some(store.clone()), Some(sorter));
        self.bind_song_count(&sort_model);
        let selection_model = gtk::SingleSelection::new(Some(sort_model));
        imp.track_list.set_model(Some(&selection_model));
    }

    fn build_sorter(&self, sort: SongSort, direction: SortDirection) -> CustomSorter {
        gtk::CustomSorter::new(move |obj1, obj2| {
            let (obj1, obj2) = match direction {
                SortDirection::Ascending => (obj1, obj2),
                SortDirection::Descending => (obj2, obj1),
            };
            let song1 = obj1.downcast_ref::<SongModel>().unwrap();
            let song2 = obj2.downcast_ref::<SongModel>().unwrap();

            match sort {
                SongSort::Name => song1
                    .title()
                    .to_lowercase()
                    .cmp(&song2.title().to_lowercase())
                    .into(),
                SongSort::Artist => song1
                    .artists_string()
                    .to_lowercase()
                    .cmp(&song2.artists_string().to_lowercase())
                    .into(),
                SongSort::DateAdded => {
                    // Reverse order for newest first
                    song2.date_created().cmp(&song1.date_created()).into()
                }
                SongSort::Album => song1
                    .album()
                    .to_lowercase()
                    .cmp(&song2.album().to_lowercase())
                    .into(),
            }
        })
    }

    fn bind_song_count(&self, model: &impl IsA<gio::ListModel>) {
        let imp = self.imp();
        model
            .bind_property("n-items", &imp.num_songs.get(), "label")
            .transform_to(|_, n_items: u32| Some(n_items.to_string()))
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();
    }

    fn setup_model(&self) {
        let imp = self.imp();
        if imp.store.get().is_some() {
            // Store is already set up with a model.
            return;
        }
        imp.store.get_or_init(gio::ListStore::new::<SongModel>);
        let name_filter = create_string_filter::<SongModel>("title");
        imp.name_filter
            .set(name_filter)
            .expect("Name filter should only be set once");
        let artist_filter = create_string_filter::<SongModel>("artists-string");
        imp.artist_filter
            .set(artist_filter)
            .expect("Artist filter should only be set once");
    }

    fn setup_factory(&self) {
        let imp = self.imp();

        if imp.track_list.factory().is_some() {
            return;
        }

        let Some(audio_model) = self.get_application().audio_model() else {
            warn!("No audio model set, aborting");
            return;
        };

        let store = imp.store.get().expect("Store should be initialized");
        let selection_model = gtk::NoSelection::new(Some(store.clone()));
        let factory = gtk::SignalListItemFactory::new();

        factory.connect_setup(move |_, list_item| {
            let song_widget = Song::new();
            let item = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Should be a ListItem");

            item.bind_property("position", &song_widget, "position")
                .flags(glib::BindingFlags::SYNC_CREATE)
                .build();

            item.set_child(Some(&song_widget))
        });

        factory.connect_bind(glib::clone!(
            #[weak (rename_to = song_list)]
            self,
            #[weak]
            audio_model,
            move |_, list_item| {
                let list_item = list_item
                    .downcast_ref::<gtk::ListItem>()
                    .expect("Needs to be a ListItem");
                let song_model = list_item
                    .item()
                    .and_downcast::<SongModel>()
                    .expect("Item should be an SongModel");
                let song_widget = list_item
                    .child()
                    .and_downcast::<Song>()
                    .expect("Child has to be Song");

                song_widget.set_song_data(&song_model);

                song_utils::connect_playing_indicator(&song_widget, &song_model, &audio_model);

                let nav_handlers =
                    song_utils::connect_song_navigation(&song_widget, &song_list.get_root_window());
                song_widget.imp().signal_handlers.replace(nav_handlers);
            }
        ));

        factory.connect_unbind(glib::clone!(
            #[weak]
            audio_model,
            move |_, list_item| {
                let list_item = list_item
                    .downcast_ref::<gtk::ListItem>()
                    .expect("Needs to be a ListItem");
                let song_widget = list_item
                    .child()
                    .and_downcast::<Song>()
                    .expect("Child has to be Song");

                // disconnect song-changed handler, it's connected to audio_model
                song_utils::disconnect_playing_indicator(&song_widget, &audio_model);

                // disconnect other handlers connected to song
                song_utils::disconnect_signal_handlers(&song_widget);
            }
        ));

        imp.track_list.set_single_click_activate(true);
        imp.track_list.set_model(Some(&selection_model));
        imp.track_list.set_factory(Some(&factory));
    }

    fn set_empty(&self, empty: bool) {
        self.imp().empty.set_visible(empty);
        self.imp().track_list.set_visible(!empty);
    }
}

impl Default for SongList {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use std::cell::{OnceCell, RefCell};

    use adw::subclass::prelude::*;
    use gtk::{
        CompositeTemplate, gio,
        glib::{self, subclass::InitializingObject},
        prelude::*,
    };

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/song_list.ui")]
    pub struct SongList {
        #[template_child]
        pub track_list: TemplateChild<gtk::ListView>,
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
        #[template_child]
        pub num_songs: TemplateChild<gtk::Label>,

        pub store: OnceCell<gio::ListStore>,
        pub name_filter: OnceCell<gtk::StringFilter>,
        pub artist_filter: OnceCell<gtk::StringFilter>,
        pub current_sorter: RefCell<Option<gtk::CustomSorter>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SongList {
        const NAME: &'static str = "GellySongList";
        type Type = super::SongList;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SongList {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_model();
            self.setup_signals();
        }
    }
    impl WidgetImpl for SongList {}
    impl BoxImpl for SongList {}
    impl SongList {
        fn setup_signals(&self) {
            self.track_list.connect_activate(glib::clone!(
                #[weak(rename_to=imp)]
                self,
                move |_, position| {
                    imp.obj().activate_song(position as usize);
                }
            ));

            self.search_entry.connect_search_changed(glib::clone!(
                #[weak(rename_to = song_list)]
                self.obj(),
                move |entry| {
                    song_list.search_changed(&entry.text());
                }
            ));

            self.sort_dropdown.connect_selected_notify(glib::clone!(
                #[weak(rename_to = song_list)]
                self.obj(),
                move |_| {
                    song_list.apply_sorting();
                }
            ));

            self.sort_direction.connect_active_notify(glib::clone!(
                #[weak(rename_to = song_list)]
                self.obj(),
                move |_| {
                    song_list.apply_sorting();
                }
            ));

            self.obj().connect_map(glib::clone!(
                #[weak(rename_to = song_list)]
                self.obj(),
                move |_| {
                    if !song_list.is_loaded() {
                        song_list.pull_songs();
                    }
                }
            ));
        }
    }
}
