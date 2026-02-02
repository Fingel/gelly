use crate::{
    async_utils::spawn_tokio,
    jellyfin::{JellyfinError, api::MusicDto, utils::format_duration},
    library_utils::songs_for_playlist,
    models::{PlaylistModel, SongModel},
    ui::{
        music_context_menu::{ContextActions, construct_menu, create_actiongroup},
        page_traits::DetailPage,
        playlist_dialogs,
        song::{Song, SongOptions},
        song_utils::connect_song_navigation,
        widget_ext::WidgetApplicationExt,
    },
};
use glib::Object;
use gtk::{
    gio::{self, SimpleActionGroup},
    glib,
    prelude::*,
    subclass::prelude::*,
};
use log::{error, warn};

glib::wrapper! {
    pub struct PlaylistDetail(ObjectSubclass<imp::PlaylistDetail>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl DetailPage for PlaylistDetail {
    type Model = PlaylistModel;

    fn set_model(&self, model: &PlaylistModel) {
        let imp = self.imp();
        imp.model.replace(Some(model.clone()));
        imp.name_label.set_text(&model.name());
        imp.delete.set_sensitive(!model.is_smart());
        if model.is_smart() {
            self.use_static_icon(model.playlist_type().icon_name());
        } else {
            self.use_playlist_icon(&model.id());
        }
        self.pull_tracks();
    }

    fn get_model(&self) -> Option<Self::Model> {
        self.imp().model.borrow().clone()
    }
}

impl PlaylistDetail {
    pub fn new() -> Self {
        Object::builder().build()
    }

    fn get_store(&self) -> &gio::ListStore {
        self.imp()
            .store
            .get()
            .expect("PlaylistDetail store should be initialized")
    }

    fn repopulate_store(&self) {
        let store = self.get_store();
        store.remove_all();
        let songs = self.imp().songs.borrow();
        for song in songs.iter() {
            store.append(song);
        }
    }

    fn setup_model(&self) {
        let imp = self.imp();
        let store = gio::ListStore::new::<SongModel>();
        imp.store
            .set(store.clone())
            .expect("Store should only be set once");
        let selection_model = gtk::NoSelection::new(Some(store));
        let factory = gtk::SignalListItemFactory::new();

        factory.connect_setup(glib::clone!(
            #[weak(rename_to = playlist_detail)]
            self,
            move |_, list_item| {
                let is_smart_playlist = playlist_detail
                    .get_model()
                    .map(|p| p.is_smart())
                    .unwrap_or(true);
                let song_widget = if is_smart_playlist {
                    Song::new()
                } else {
                    Song::new_with(SongOptions {
                        dnd: true,
                        in_playlist: true,
                        in_queue: false,
                    })
                };

                let item = list_item
                    .downcast_ref::<gtk::ListItem>()
                    .expect("Needs to be a ListItem");
                item.set_child(Some(&song_widget))
            }
        ));

        factory.connect_bind(glib::clone!(
            #[weak (rename_to = playlist_detail)]
            self,
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

                let position = list_item.position();
                song_widget.set_position(position as i32);
                song_widget.set_song_data(&song_model);
                song_widget.set_track_number(position + 1);

                if let Some(audio_model) = playlist_detail.get_application().audio_model()
                    && let Some(current_song) = audio_model.current_song()
                {
                    song_widget.set_playing(current_song.id() == song_model.id());
                }

                connect_song_navigation(&song_widget, &playlist_detail.get_root_window());

                let is_smart_playlist = playlist_detail
                    .get_model()
                    .map(|p| p.is_smart())
                    .unwrap_or(true);

                if !is_smart_playlist {
                    song_widget.connect_closure(
                        "widget-moved",
                        false,
                        glib::closure_local!(
                            #[weak(rename_to= playlist_detail_for_move)]
                            playlist_detail,
                            move |song_widget: Song, source_index: i32| {
                                let target_index = song_widget.get_position() as usize;
                                let source_index = source_index as usize;
                                playlist_detail_for_move
                                    .handle_song_moved(source_index, target_index)
                            }
                        ),
                    );
                    song_widget.connect_closure(
                        "remove-from-playlist",
                        false,
                        glib::closure_local!(
                            #[weak(rename_to= playlist_detail_for_remove)]
                            playlist_detail,
                            move |_: Song, song_id: String| {
                                playlist_detail_for_remove.handle_remove_from_playlist(song_id)
                            }
                        ),
                    );
                }
            }
        ));

        imp.track_list.set_single_click_activate(true);
        imp.track_list.set_model(Some(&selection_model));
        imp.track_list.set_factory(Some(&factory));
    }

    fn use_static_icon(&self, name: &str) {
        let imp = self.imp();
        imp.album_image.set_visible(false);
        imp.static_icon.set_icon_name(Some(name));
        imp.static_icon.set_visible(true);
    }

    fn use_playlist_icon(&self, id: &str) {
        let imp = self.imp();
        imp.static_icon.set_visible(false);
        imp.album_image.set_item_id(id, None);
        imp.album_image.set_visible(true);
    }

    fn pull_tracks(&self) {
        let Some(playlist_model) = self.get_model() else {
            warn!("No playlist model set");
            return;
        };
        let app = self.get_application();
        songs_for_playlist(
            &playlist_model,
            &app,
            glib::clone!(
                #[weak(rename_to=playlist_detail)]
                self,
                move |result: Result<Vec<MusicDto>, JellyfinError>| {
                    match result {
                        Ok(music_data) => {
                            let songs: Vec<SongModel> =
                                music_data.iter().map(SongModel::from).collect();
                            playlist_detail.populate_tracks_with_songs(songs);
                        }
                        Err(error) => {
                            playlist_detail
                                .toast("Could not load playlist, please try again.", None);
                            warn!("Unable to load playlist: {error}");
                        }
                    }
                }
            ),
        );
    }

    fn populate_tracks_with_songs(&self, songs: Vec<SongModel>) {
        let store = self.get_store();
        store.remove_all();
        for song in &songs {
            store.append(song);
        }

        self.imp().songs.replace(songs);
        self.update_track_metadata();
    }

    fn setup_menu(&self) {
        let options = ContextActions {
            can_remove_from_playlist: false,
            in_queue: false,
            action_prefix: "playlist_detail".to_string(),
        };
        let popover_menu = construct_menu(
            &options,
            glib::clone!(
                #[weak(rename_to = playlist_detail)]
                self,
                #[upgrade_or_default]
                move || playlist_detail
                    .get_application()
                    .playlists()
                    .borrow()
                    .clone()
            ),
        );
        self.imp().action_menu.set_popover(Some(&popover_menu));
        let action_group = self.create_action_group();
        self.insert_action_group(&options.action_prefix, Some(&action_group));
    }

    fn create_action_group(&self) -> SimpleActionGroup {
        let on_add_to_playlist = glib::clone!(
            #[weak(rename_to = playlist)]
            self,
            move |playlist_id| {
                playlist.on_add_to_playlist(playlist_id);
            }
        );

        let on_queue_next = glib::clone!(
            #[weak(rename_to = playlist)]
            self,
            move || {
                playlist.enqueue_playlist(false);
            }
        );

        let on_queue_last = glib::clone!(
            #[weak(rename_to = playlist)]
            self,
            move || {
                playlist.enqueue_playlist(true);
            }
        );

        create_actiongroup(
            Some(on_add_to_playlist),
            None::<fn()>,
            Some(on_queue_next),
            Some(on_queue_last),
        )
    }

    fn on_add_to_playlist(self, playlist_id: String) {
        let song_ids = self
            .imp()
            .songs
            .borrow()
            .iter()
            .map(|song| song.id())
            .collect::<Vec<_>>();

        let app = self.get_application();
        let jellyfin = app.jellyfin();
        let playlist_id = playlist_id.to_string();
        spawn_tokio(
            async move { jellyfin.add_playlist_items(&playlist_id, &song_ids).await },
            glib::clone!(
                #[weak(rename_to = playlist)]
                self,
                move |result| {
                    match result {
                        Ok(()) => {
                            playlist.toast("Added songs to playlist", None);
                            app.refresh_playlists(true);
                        }
                        Err(e) => {
                            playlist.toast("Failed to add songs to playlist", None);
                            warn!("Failed to add songs to playlist: {}", e);
                        }
                    }
                }
            ),
        );
    }

    fn handle_song_moved(&self, source_index: usize, target_index: usize) {
        if source_index == target_index {
            return;
        }
        let Some(playlist_model) = self.get_model() else {
            warn!("No playlist model found");
            return;
        };
        if playlist_model.is_smart() {
            warn!("Attempted to re-order a smart playlist");
            return;
        }

        let mut songs = self.imp().songs.borrow_mut().clone();
        if source_index >= songs.len() || target_index >= songs.len() {
            warn!(
                "Invalid reorder indices: {} -> {} (length: {})",
                source_index,
                target_index,
                songs.len()
            );
            return;
        }
        let song_being_moved = songs[source_index].clone();
        let item_id = song_being_moved.id();
        songs.remove(source_index);
        songs.insert(target_index, song_being_moved);
        self.imp().songs.replace(songs);

        let store = self.get_store();
        if let Some(item) = store.item(source_index as u32) {
            store.remove(source_index as u32);
            store.insert(target_index as u32, &item);
        }

        self.repopulate_store();

        // Persist the change
        let playlist_id = playlist_model.id();
        let app = self.get_application();

        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to = playlist_detail)]
            self,
            async move {
                let jellyfin_client = app.jellyfin();
                match jellyfin_client
                    .move_playlist_item(&playlist_id, &item_id, target_index as i32)
                    .await
                {
                    Ok(_) => {
                        log::debug!(
                            "Successfully moved playlist item '{}' from position {} to {}",
                            item_id,
                            source_index,
                            target_index
                        );
                    }
                    Err(error) => {
                        log::error!("Failed to move playlist item: {}", error);
                        playlist_detail.toast("Failed to save playlist order.", None);
                        // Revert to server state
                        playlist_detail.pull_tracks();
                    }
                }
            }
        ));
    }

    fn handle_remove_from_playlist(&self, song_id: String) {
        let Some(playlist_model) = self.get_model() else {
            warn!("No playlist model found");
            return;
        };
        if playlist_model.is_smart() {
            warn!("Attempted to remove from a smart playlist");
            return;
        }
        let mut songs = self.imp().songs.borrow_mut().clone();
        if let Some(index) = songs.iter().position(|s| s.id() == song_id) {
            songs.remove(index);
            self.imp().songs.replace(songs);

            let store = self.get_store();
            store.remove(index as u32);
            self.repopulate_store();
            self.update_track_metadata();

            // Persist the change
            let playlist_id = playlist_model.id();
            let app = self.get_application();
            let jellyfin = app.jellyfin();

            spawn_tokio(
                async move { jellyfin.remove_playlist_item(&playlist_id, &song_id).await },
                glib::clone!(
                    #[weak(rename_to = playlist_detail)]
                    self,
                    move |result| {
                        match result {
                            Ok(_) => {
                                log::debug!("Successfully removed item from playlist");
                                app.refresh_playlists(true);
                            }
                            Err(error) => {
                                log::error!("Failed to remove song from playlist: {}", error);
                                playlist_detail.toast("Failed to modify playlist.", None);
                                // Revert to server state
                                playlist_detail.pull_tracks();
                            }
                        }
                    }
                ),
            );
        }
    }

    pub fn song_selected(&self, index: usize) {
        let songs = self.imp().songs.borrow().clone();
        if let Some(audio_model) = self.get_application().audio_model() {
            audio_model.set_queue(songs, index);
        } else {
            self.toast("Audio model not initialized, please restart", None);
            warn!("No audio model found");
        }
    }

    fn play_playlist(&self) {
        let songs = self.imp().songs.borrow().clone();
        if let Some(audio_model) = self.get_application().audio_model() {
            audio_model.set_queue(songs, 0);
        } else {
            self.toast("Audio model not initialized, please restart", None);
            warn!("No audio model found");
        }
    }

    fn enqueue_playlist(&self, to_end: bool) {
        let songs = self.imp().songs.borrow().clone();
        if let Some(audio_model) = self.get_application().audio_model() {
            let song_cnt = songs.len();
            if to_end {
                audio_model.append_to_queue(songs);
            } else {
                audio_model.prepend_to_queue(songs);
            }
            self.toast(&format!("{} songs added to queue", song_cnt), None);
        } else {
            self.toast("Audio model not initialized, please restart", None);
            warn!("No audio model found");
        }
    }

    fn update_track_metadata(&self) {
        let songs = self.imp().songs.borrow();
        self.imp().track_count.set_text(&songs.len().to_string());
        let duration = songs.iter().map(|song| song.duration()).sum::<u64>();
        self.imp()
            .playlist_duration
            .set_text(&format_duration(duration));
    }

    fn confirm_delete(&self) {
        if let Some(model) = self.get_model()
            && model.is_smart()
        {
            self.toast("Smart playlists cannot be deleted", None);
            return;
        }

        playlist_dialogs::confirm_delete(
            Some(&self.get_root_window()),
            glib::clone!(
                #[weak (rename_to = playlist_detail)]
                self,
                move |delete| {
                    if delete {
                        playlist_detail.delete_playlist();
                    }
                }
            ),
        );
    }

    fn delete_playlist(&self) {
        let app = self.get_application();
        let jellyfin = app.jellyfin();
        let window = self.get_root_window();
        let item_id = self.id();
        spawn_tokio(
            async move { jellyfin.delete_item(&item_id).await },
            glib::clone!(
                #[weak (rename_to = playlist_detail)]
                self,
                move |result| {
                    match result {
                        Ok(()) => {
                            app.refresh_playlists(true);
                            window.go_back();
                            playlist_detail.toast("Playlist deleted", None);
                        }
                        Err(err) => {
                            playlist_detail
                                .toast(&format!("Failed to delete playlist: {}", err), None);
                            error!("Failed to delete playlist: {}", err);
                        }
                    }
                }
            ),
        );
    }
}

impl Default for PlaylistDetail {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use std::cell::{Cell, OnceCell, RefCell};

    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate, gio,
        glib::{self},
        prelude::*,
    };

    use crate::{
        models::{PlaylistModel, SongModel},
        ui::album_art::AlbumArt,
    };

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/playlist_detail.ui")]
    pub struct PlaylistDetail {
        #[template_child]
        pub album_image: TemplateChild<AlbumArt>,
        #[template_child]
        pub static_icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub track_list: TemplateChild<gtk::ListView>,
        #[template_child]
        pub track_count: TemplateChild<gtk::Label>,
        #[template_child]
        pub playlist_duration: TemplateChild<gtk::Label>,
        #[template_child]
        pub play_all: TemplateChild<gtk::Button>,
        #[template_child]
        pub action_menu: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub delete: TemplateChild<gtk::Button>,

        pub model: RefCell<Option<PlaylistModel>>,
        pub songs: RefCell<Vec<SongModel>>,
        pub song_change_signal_connected: Cell<bool>,
        pub store: OnceCell<gio::ListStore>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlaylistDetail {
        const NAME: &'static str = "GellyPlaylistDetail";
        type Type = super::PlaylistDetail;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }
    impl BoxImpl for PlaylistDetail {}
    impl ObjectImpl for PlaylistDetail {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_model();
            self.obj().setup_menu();
            self.setup_signals();
        }
    }
    impl WidgetImpl for PlaylistDetail {}

    impl PlaylistDetail {
        fn setup_signals(&self) {
            self.track_list.connect_activate(glib::clone!(
                #[weak(rename_to=imp)]
                self,
                move |_, position| {
                    imp.obj().song_selected(position as usize);
                }
            ));

            self.play_all.connect_clicked(glib::clone!(
                #[weak(rename_to=imp)]
                self,
                move |_| {
                    imp.obj().play_playlist();
                }
            ));

            self.delete.connect_clicked(glib::clone!(
                #[weak(rename_to=imp)]
                self,
                move |_| {
                    imp.obj().confirm_delete();
                }
            ));
        }
    }
}
