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
        list_helpers::{create_string_filter, handle_grid_activation},
        page_traits::{SortDirection, SortType, TopPage},
        playlist::Playlist,
        playlist_dialogs,
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
                                let songs: Vec<SongModel> = music_data
                                    .iter()
                                    .map(|dto| SongModel::new(dto, false)) // TODO get favorite status here
                                    .collect();
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
        let search = if query.is_empty() { None } else { Some(query) };
        self.imp().name_filter.get().unwrap().set_search(search);
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
        config::set_playlists_sort_by(sort_by);
        config::set_playlists_sort_direction(direction);
        self.imp().sort_state.set((sort_by, direction));
        self.imp()
            .sorter
            .get()
            .unwrap()
            .changed(gtk::SorterChange::Different);
        self.reset_position();
    }

    fn filter_favorites(&self, _active: bool) {
        // TODO: playlists do not have have favorite implemented yet
    }

    fn reset_position(&self) {
        let imp = self.imp();
        if imp.grid_view.model().is_some_and(|m| m.n_items() > 0) {
            imp.grid_view
                .scroll_to(0, gtk::ListScrollFlags::NONE, None::<gtk::ScrollInfo>);
        }
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
        let library_cnt = self.get_application().library().songs.borrow().len();
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
            store.append(&PlaylistModel::new(shuffle_type));
        }
        if config::get_playlist_most_played_enabled() {
            let most_played_type = PlaylistType::MostPlayed {
                count: DEFAULT_SMART_COUNT,
            };
            store.append(&PlaylistModel::new(most_played_type));
        }

        for playlist in playlists {
            let playlist_type = PlaylistType::new_regular(
                playlist.id.clone(),
                playlist.name.clone(),
                playlist.child_count,
            );
            store.append(&PlaylistModel::new(playlist_type));
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
            let a = a.downcast_ref::<PlaylistModel>().unwrap();
            let b = b.downcast_ref::<PlaylistModel>().unwrap();
            match options[sort_by as usize] {
                SortType::Name => b
                    .is_smart()
                    .cmp(&a.is_smart())
                    .then(a.name().to_lowercase().cmp(&b.name().to_lowercase()))
                    .into(),
                SortType::NumSongs => b
                    .is_smart()
                    .cmp(&a.is_smart())
                    .then(a.child_count().cmp(&b.child_count()))
                    .into(),
                _ => gtk::Ordering::Equal,
            }
        })
    }

    fn setup_model(&self) {
        let store = gio::ListStore::new::<PlaylistModel>();

        let name_filter = create_string_filter::<PlaylistModel>("name");
        let search_model =
            gtk::FilterListModel::new(Some(store.clone()), Some(name_filter.clone()));

        let sorter = self.build_sorter();
        let sort_model = gtk::SortListModel::new(Some(search_model), Some(sorter.clone()));
        let selection = gtk::SingleSelection::new(Some(sort_model));

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            let item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
            item.set_child(Some(&Playlist::new()));
        });
        factory.connect_bind(move |_, list_item| {
            let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
            let playlist_model = list_item.item().and_downcast::<PlaylistModel>().unwrap();
            let playlist_widget = list_item.child().and_downcast::<Playlist>().unwrap();
            playlist_widget.set_playlist_model(&playlist_model);
        });

        let imp = self.imp();
        imp.grid_view.set_model(Some(&selection));
        imp.grid_view.set_factory(Some(&factory));
        imp.store.set(store).unwrap();
        imp.name_filter.set(name_filter).unwrap();
        imp.sorter.set(sorter).unwrap();
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
    use std::cell::{Cell, OnceCell};
    use std::rc::Rc;

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
        pub sorter: OnceCell<gtk::CustomSorter>,
        pub sort_state: Rc<Cell<(u32, u32)>>,
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
