use crate::{
    application::Application,
    async_utils::spawn_tokio,
    backend::BackendError,
    config,
    jellyfin::api::MusicDto,
    library_utils::songs_for_playlist,
    models::{
        PlaylistModel, SongModel,
        playlist_type::{DEFAULT_SMART_COUNT, PlaylistType},
    },
    ui::{
        list_helpers::*,
        page_traits::{SortDirection, SortType, TopPage},
        playlist::Playlist,
        playlist_dialogs,
        widget_ext::WidgetApplicationExt,
    },
};
use glib::Object;
use gtk::{
    CustomSorter, SingleSelection, SortListModel, gio,
    glib::{self},
    prelude::*,
    subclass::prelude::*,
};
use log::{error, warn};

glib::wrapper! {
    pub struct PlaylistList(ObjectSubclass<imp::PlaylistList>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl TopPage for PlaylistList {
    fn can_new(&self) -> bool {
        true
    }

    fn play_selected(&self) {
        if let Some(selection) = self.imp().grid_view.model()
            && let Some(single_selection) = selection.downcast_ref::<gtk::SingleSelection>()
            && let Some(selected_item) = single_selection.selected_item()
            && let Some(playlist_model) = selected_item.downcast_ref::<PlaylistModel>()
        {
            let app = self.get_application();
            songs_for_playlist(
                playlist_model,
                &app,
                glib::clone!(
                    #[weak(rename_to=playlist_list)]
                    self,
                    move |result: Result<Vec<MusicDto>, BackendError>| {
                        match result {
                            Ok(music_data) => {
                                let songs: Vec<SongModel> =
                                    music_data.iter().map(SongModel::from).collect();
                                if let Some(audio_model) =
                                    playlist_list.get_application().audio_model()
                                {
                                    audio_model.set_queue(songs, 0, false);
                                } else {
                                    playlist_list
                                        .toast("No audio model found, please restart.", None);
                                    log::warn!("No audio model found");
                                }
                            }
                            Err(error) => {
                                playlist_list
                                    .toast("Could not load playlist, please try again.", None);
                                warn!("Unable to load playlist: {error}");
                            }
                        }
                    }
                ),
            );
        }
    }

    fn search_changed(&self, query: &str) {
        let imp = self.imp();
        let store = imp.store.get().expect("Store should be initialized");
        let name_filter = imp
            .name_filter
            .get()
            .expect("Name filter should be initialized");
        let sorter = if let Some(current_sorter) = imp.current_sorter.borrow().as_ref() {
            current_sorter.clone()
        } else {
            let default_sorter = self.build_sorter(SortType::Name, SortDirection::Ascending);
            imp.current_sorter.replace(Some(default_sorter.clone()));
            default_sorter
        };
        apply_single_filter_search(query, sorter.upcast(), store, name_filter, &imp.grid_view);
    }

    fn create_new(&self) {
        playlist_dialogs::new_playlist(
            Some(&self.get_root_window()),
            glib::clone!(
                #[weak (rename_to = playlist_list)]
                self,
                move |name| {
                    playlist_list.create_new_playlist(name);
                }
            ),
        );
    }

    fn sort_options(&self) -> &[SortType] {
        &[SortType::Name, SortType::NumSongs]
    }

    fn current_sort_by(&self) -> u32 {
        config::get_playlists_sort_by()
    }

    fn current_sort_direction(&self) -> u32 {
        config::get_playlists_sort_direction()
    }

    fn apply_sort(&self, sort_by: u32, direction: u32) {
        let imp = self.imp();
        let sort_option = self.sort_options()[sort_by as usize];
        let sort_direction = SortDirection::try_from(direction).unwrap_or(SortDirection::Ascending);
        config::set_playlists_sort_by(sort_by);
        config::set_playlists_sort_direction(direction);
        let sorter = self.build_sorter(sort_option, sort_direction);
        imp.current_sorter.replace(Some(sorter.clone()));
        let store = imp.store.get().expect("Store should be initialized");
        let sort_model = SortListModel::new(Some(store.clone()), Some(sorter));
        let selection_model = SingleSelection::new(Some(sort_model));
        imp.grid_view.set_model(Some(&selection_model));
    }
}

impl PlaylistList {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn create_new_playlist(&self, name: String) {
        let app = self.get_application();
        let jellyfin = app.jellyfin();
        spawn_tokio(
            async move { jellyfin.new_playlist(&name, vec![]).await },
            glib::clone!(
                #[weak (rename_to = playlist_list)]
                self,
                move |result| {
                    match result {
                        Ok(_id) => {
                            app.refresh_playlists(true);
                            playlist_list.toast("Playlist created", None);
                        }
                        Err(err) => {
                            playlist_list
                                .toast(&format!("Failed to create playlist: {}", err), None);
                            error!("Failed to create playlist: {}", err);
                        }
                    }
                }
            ),
        );
    }

    pub fn pull_playlists(&self) {
        let playlists = self.get_application().playlists().borrow().clone();
        let library_cnt = self.get_application().library().borrow().len();
        self.set_empty(false);
        let store = self
            .imp()
            .store
            .get()
            .expect("PlaylistList store should be initialized");
        store.remove_all();

        if config::get_playlist_shuffle_enabled() {
            let shuffle_type = PlaylistType::ShuffleLibrary {
                count: library_cnt as u64,
            };
            let shuffle_playlist = PlaylistModel::new(shuffle_type);
            store.append(&shuffle_playlist);
        }
        if config::get_playlist_most_played_enabled() {
            let most_played_type = PlaylistType::MostPlayed {
                count: DEFAULT_SMART_COUNT,
            };
            let most_played_playlist = PlaylistModel::new(most_played_type);
            store.append(&most_played_playlist);
        }

        for playlist in playlists {
            let playlist_type = PlaylistType::new_regular(
                playlist.id.clone(),
                playlist.name.clone(),
                playlist.child_count,
            );
            let playlist_obj = PlaylistModel::new(playlist_type);
            store.append(&playlist_obj);
        }
        self.apply_sort(self.current_sort_by(), self.current_sort_direction());
    }

    pub fn activate_playlist(&self, index: u32) {
        let window = self.get_root_window();
        handle_grid_activation::<PlaylistModel, _>(
            &self.imp().grid_view,
            index,
            |playlist_model| {
                window.show_playlist_detail(playlist_model);
            },
        );
    }

    pub fn setup_library_connection(&self) {
        let app = self.get_application();
        app.connect_closure(
            "playlists-refreshed",
            false,
            glib::closure_local!(
                #[weak(rename_to = playlist_list)]
                self,
                move |_app: Application, _total_record_count: u64| {
                    playlist_list.pull_playlists();
                }
            ),
        );
    }

    fn build_sorter(&self, sort: SortType, direction: SortDirection) -> CustomSorter {
        CustomSorter::new(move |obj1, obj2| {
            let (obj1, obj2) = match direction {
                SortDirection::Ascending => (obj1, obj2),
                SortDirection::Descending => (obj2, obj1),
            };
            let playlist1 = obj1.downcast_ref::<PlaylistModel>().unwrap();
            let playlist2 = obj2.downcast_ref::<PlaylistModel>().unwrap();

            match sort {
                SortType::Name => playlist2
                    .is_smart()
                    .cmp(&playlist1.is_smart())
                    .then(
                        playlist1
                            .name()
                            .to_lowercase()
                            .cmp(&playlist2.name().to_lowercase()),
                    )
                    .into(),
                SortType::NumSongs => playlist2
                    .is_smart()
                    .cmp(&playlist1.is_smart())
                    .then(playlist1.child_count().cmp(&playlist2.child_count()))
                    .into(),
                _ => std::cmp::Ordering::Equal.into(),
            }
        })
    }

    fn setup_model(&self) {
        let imp = self.imp();
        let store = gio::ListStore::new::<PlaylistModel>();
        let name_filter = create_string_filter::<PlaylistModel>("name");
        imp.store
            .set(store.clone())
            .expect("PlaylistList store should only be set once");
        imp.name_filter
            .set(name_filter)
            .expect("PlaylistList name filter should only be set once");

        let selection_model = gtk::SingleSelection::new(Some(store));
        let factory = gtk::SignalListItemFactory::new();

        factory.connect_setup(move |_, list_item| {
            let placeholder = Playlist::new();
            let item = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be a ListItem");
            item.set_child(Some(&placeholder));
        });

        factory.connect_bind(move |_, list_item| {
            let list_item = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be a ListItem");
            let playlist_model = list_item
                .item()
                .and_downcast::<PlaylistModel>()
                .expect("Item should be a PlaylistModel");
            let playlist_widget = list_item
                .child()
                .and_downcast::<Playlist>()
                .expect("child should be a Playlist");
            playlist_widget.set_playlist_model(&playlist_model);
        });

        imp.grid_view.set_model(Some(&selection_model));
        imp.grid_view.set_factory(Some(&factory));
    }

    fn set_empty(&self, empty: bool) {
        self.imp().empty.set_visible(empty);
        self.imp().grid_view.set_visible(!empty);
    }
}

impl Default for PlaylistList {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use std::cell::{OnceCell, RefCell};

    use crate::config::settings;
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{CompositeTemplate, gio, glib, prelude::*};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/playlist_list.ui")]
    pub struct PlaylistList {
        #[template_child]
        pub grid_view: TemplateChild<gtk::GridView>,
        #[template_child]
        pub empty: TemplateChild<adw::StatusPage>,

        pub store: OnceCell<gio::ListStore>,
        pub name_filter: OnceCell<gtk::StringFilter>,
        pub current_sorter: RefCell<Option<gtk::CustomSorter>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlaylistList {
        const NAME: &'static str = "GellyPlaylistList";
        type Type = super::PlaylistList;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PlaylistList {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_model();

            self.grid_view.connect_activate(glib::clone!(
                #[weak(rename_to = playlist_list)]
                self.obj(),
                move |_, position| {
                    playlist_list.activate_playlist(position);
                }
            ));

            settings().connect_changed(
                Some("playlist-shuffle-enabled"),
                glib::clone!(
                    #[weak(rename_to = playlist_list)]
                    self.obj(),
                    move |_settings, _key| {
                        playlist_list.pull_playlists();
                    }
                ),
            );

            settings().connect_changed(
                Some("playlist-most-played-enabled"),
                glib::clone!(
                    #[weak(rename_to = playlist_list)]
                    self.obj(),
                    move |_settings, _key| {
                        playlist_list.pull_playlists();
                    }
                ),
            );
        }
    }
    impl WidgetImpl for PlaylistList {}
    impl BoxImpl for PlaylistList {}
}
