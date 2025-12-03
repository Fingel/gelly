use crate::{
    jellyfin::{JellyfinError, api::MusicDto, utils::format_duration},
    library_utils::songs_for_playlist,
    models::{PlaylistModel, SongModel},
    ui::{drag_scrollable::DragScrollable, song::Song, widget_ext::WidgetApplicationExt},
};
use glib::Object;
use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use log::warn;

glib::wrapper! {
    pub struct PlaylistDetail(ObjectSubclass<imp::PlaylistDetail>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl PlaylistDetail {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn set_playlist_model(&self, playlist_model: &PlaylistModel) {
        let imp = self.imp();
        imp.playlist_model.replace(Some(playlist_model.clone()));
        imp.name_label.set_text(&playlist_model.name());
        if playlist_model.is_smart() {
            self.use_static_icon(playlist_model.playlist_type().icon_name());
        } else {
            self.use_playlist_icon(&playlist_model.id());
        }
        self.pull_tracks();
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

    fn get_playlist_model(&self) -> Option<PlaylistModel> {
        self.imp().playlist_model.borrow().clone()
    }

    fn pull_tracks(&self) {
        let Some(playlist_model) = self.get_playlist_model() else {
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
        let track_list = &self.imp().track_list;
        track_list.remove_all();
        // Smart playlists cannot be reordered
        let is_smart_playlist = self
            .get_playlist_model()
            .map(|p| p.is_smart())
            .unwrap_or(true);

        for (i, song) in songs.iter().enumerate() {
            let song_widget = if is_smart_playlist {
                Song::new()
            } else {
                let song_widget = Song::new_with(true, true);
                song_widget.connect_closure(
                    "widget-moved",
                    false,
                    glib::closure_local!(
                        #[weak(rename_to= playlist_detail)]
                        self,
                        move |song_widget: Song, source_index: i32| {
                            let target_index = song_widget.index() as usize;
                            let source_index = source_index as usize;
                            playlist_detail.handle_song_moved(source_index, target_index)
                        }
                    ),
                );
                song_widget
            };
            // we don't want the track number here, we want the playlist index
            song.set_track_number(i as u32 + 1);
            song_widget.set_song_data(song);
            song_widget.show_details();
            // Check if currently playing song is in the playlist
            if let Some(audio_model) = self.get_application().audio_model()
                && audio_model
                    .current_song()
                    .is_some_and(|current_song| current_song.id() == song.id())
            {
                song_widget.set_playing(true);
            }

            track_list.append(&song_widget);
        }
        self.imp().songs.replace(songs);
        self.update_track_metadata();
    }

    fn handle_song_moved(&self, source_index: usize, target_index: usize) {
        if source_index == target_index {
            return;
        }
        let Some(playlist_model) = self.get_playlist_model() else {
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

        // We do this instead of re-drawing all widgets to avoid the jump in scroll.
        let track_list = &self.imp().track_list;
        if let Some(source_row) = track_list.row_at_index(source_index as i32) {
            // Remove and reinsert the widget
            track_list.remove(&source_row);
            track_list.insert(&source_row, target_index as i32);
            self.update_all_track_numbers();
        }

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

    fn update_all_track_numbers(&self) {
        let track_list = &self.imp().track_list;
        for i in 0..track_list.observe_children().n_items() {
            if let Some(row) = track_list.row_at_index(i as i32)
                && let Some(song_widget) = row.downcast_ref::<Song>()
            {
                song_widget.set_track_number(i + 1);
            }
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

    fn enqueue_playlist(&self) {
        let songs = self.imp().songs.borrow().clone();
        if let Some(audio_model) = self.get_application().audio_model() {
            let song_cnt = songs.len();
            audio_model.append_to_queue(songs);
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
}

impl Default for PlaylistDetail {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use std::cell::{Cell, RefCell};

    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
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
        pub track_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub track_count: TemplateChild<gtk::Label>,
        #[template_child]
        pub playlist_duration: TemplateChild<gtk::Label>,
        #[template_child]
        pub play_all: TemplateChild<gtk::Button>,
        #[template_child]
        pub enqueue: TemplateChild<gtk::Button>,

        pub playlist_model: RefCell<Option<PlaylistModel>>,
        pub songs: RefCell<Vec<SongModel>>,
        pub song_change_signal_connected: Cell<bool>,
        pub last_drag_focused: Cell<Option<i32>>,
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
            self.setup_signals();
        }
    }
    impl WidgetImpl for PlaylistDetail {}

    impl PlaylistDetail {
        fn setup_signals(&self) {
            self.track_list.connect_row_activated(glib::clone!(
                #[weak(rename_to=imp)]
                self,
                move |_track_list, row| {
                    let index = row.index();
                    imp.obj().song_selected(index as usize);
                }
            ));

            self.play_all.connect_clicked(glib::clone!(
                #[weak(rename_to=imp)]
                self,
                move |_| {
                    imp.obj().play_playlist();
                }
            ));

            self.enqueue.connect_clicked(glib::clone!(
                #[weak(rename_to=imp)]
                self,
                move |_| {
                    imp.obj().enqueue_playlist();
                }
            ));
        }
    }
}

impl DragScrollable for PlaylistDetail {
    fn get_last_drag_focused(&self) -> Option<i32> {
        self.imp().last_drag_focused.get()
    }

    fn set_last_drag_focused(&self, index: Option<i32>) {
        self.imp().last_drag_focused.set(index);
    }
}
