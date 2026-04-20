use gtk::{
    gio,
    glib::{self, Object},
    prelude::*,
    subclass::prelude::*,
};
use log::warn;

use crate::{
    application::Application,
    models::SongModel,
    ui::{
        list_helpers::create_string_filter,
        page_traits::{SortDirection, SortType, TopPage},
        song::Song,
        song_utils,
        widget_ext::WidgetApplicationExt,
    },
};

glib::wrapper! {
    pub struct SongList(ObjectSubclass<imp::SongList>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl TopPage for SongList {
    fn can_new(&self) -> bool {
        false
    }

    fn play_selected(&self) {
        if let Some(selection) = self.imp().track_list.model()
            && let Some(single_selection) = selection.downcast_ref::<gtk::SingleSelection>()
        {
            self.activate_song(single_selection.selected() as usize);
        }
    }

    fn search_changed(&self, query: &str) {
        let search = if query.is_empty() { None } else { Some(query) };
        let imp = self.imp();
        imp.name_filter.get().unwrap().set_search(search);
        imp.artist_filter.get().unwrap().set_search(search);
    }

    fn sort_options(&self) -> &[SortType] {
        &[SortType::DateAdded, SortType::Name, SortType::Artist]
    }

    // Saving a non-default sort order would cause an expensive sort on every app load
    // so we only save it per session
    fn current_sort_by(&self) -> u32 {
        self.imp().sort_state.get().0
    }

    fn current_sort_direction(&self) -> u32 {
        self.imp().sort_state.get().1
    }

    fn apply_sort(&self, sort_by: u32, direction: u32) {
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
                obj.downcast_ref::<SongModel>()
                    .is_some_and(|m| m.favorite())
            });
        } else {
            filter.unset_filter_func();
        }
    }

    fn reset_position(&self) {
        let imp = self.imp();
        if imp.track_list.model().is_some_and(|m| m.n_items() > 0) {
            imp.track_list
                .scroll_to(0, gtk::ListScrollFlags::NONE, None::<gtk::ScrollInfo>);
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
        self.setup_factory();

        let songs = self.get_application().library().all_songs();
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
        }
    }

    pub fn activate_song(&self, index: usize) {
        let selection_model = self
            .imp()
            .track_list
            .model()
            .expect("Track list should have a model")
            .downcast::<gtk::SingleSelection>()
            .expect("Model should be a SingleSelection");
        let current_model = selection_model
            .model()
            .expect("SingleSelection should have a model");
        let songs = (0..current_model.n_items())
            .filter_map(|i| current_model.item(i)?.downcast::<SongModel>().ok())
            .collect();
        if let Some(audio_model) = self.get_application().audio_model() {
            audio_model.set_queue(songs, index, true);
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
                    if song_list.is_loaded() {
                        song_list.pull_songs();
                    }
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
            let a = a.downcast_ref::<SongModel>().unwrap();
            let b = b.downcast_ref::<SongModel>().unwrap();
            match options[sort_by as usize] {
                SortType::Name => a
                    .title()
                    .to_lowercase()
                    .cmp(&b.title().to_lowercase())
                    .into(),
                SortType::Artist => a
                    .artists_string()
                    .to_lowercase()
                    .cmp(&b.artists_string().to_lowercase())
                    .into(),
                SortType::DateAdded => b.date_created().cmp(&a.date_created()).into(),
                _ => gtk::Ordering::Equal,
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
            return;
        }

        let store = gio::ListStore::new::<SongModel>();

        let favorites_filter = gtk::CustomFilter::new(|_| true);
        let fav_model =
            gtk::FilterListModel::new(Some(store.clone()), Some(favorites_filter.clone()));

        let name_filter = create_string_filter::<SongModel>("title");
        let artist_filter = create_string_filter::<SongModel>("artists-string");
        let search_filter = gtk::AnyFilter::new();
        search_filter.append(name_filter.clone());
        search_filter.append(artist_filter.clone());
        let search_model = gtk::FilterListModel::new(Some(fav_model), Some(search_filter));

        let sorter = self.build_sorter();
        let sort_model = gtk::SortListModel::new(Some(search_model), Some(sorter.clone()));
        self.bind_song_count(&sort_model);
        let selection = gtk::SingleSelection::new(Some(sort_model));

        imp.track_list.set_single_click_activate(true);
        imp.track_list.set_model(Some(&selection));
        imp.store.set(store).unwrap();
        imp.favorites_filter.set(favorites_filter).unwrap();
        imp.name_filter.set(name_filter).unwrap();
        imp.artist_filter.set(artist_filter).unwrap();
        imp.sorter.set(sorter).unwrap();
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

                song_utils::disconnect_playing_indicator(&song_widget, &audio_model);
                song_utils::disconnect_signal_handlers(&song_widget);
            }
        ));

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
    use std::cell::{Cell, OnceCell};
    use std::rc::Rc;

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
        pub num_songs: TemplateChild<gtk::Label>,

        pub store: OnceCell<gio::ListStore>,
        pub favorites_filter: OnceCell<gtk::CustomFilter>,
        pub name_filter: OnceCell<gtk::StringFilter>,
        pub artist_filter: OnceCell<gtk::StringFilter>,
        pub sorter: OnceCell<gtk::CustomSorter>,
        pub sort_state: Rc<Cell<(u32, u32)>>,
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
