use glib::Object;
use gtk::{
    DragSource, DropTarget,
    gdk::{ContentProvider, Drag, DragAction},
    gio::{self, SimpleAction, SimpleActionGroup},
    glib,
    prelude::*,
    subclass::prelude::*,
};
use log::warn;

use crate::{
    async_utils::spawn_tokio,
    audio::stream_info::discover_stream_info,
    cache::CacheError,
    i18n::tr,
    jellyfin::{api::ItemType, utils::format_duration},
    models::SongModel,
    ui::{
        music_context_menu::{ContextActions, add_to_playlist_dialog, construct_menu},
        stream_info_dialog,
        widget_ext::WidgetApplicationExt,
    },
};

glib::wrapper! {
    pub struct Song(ObjectSubclass<imp::Song>)
    @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Actionable;
}

pub struct SongOptions {
    pub dnd: bool,
    pub in_playlist: bool,
    pub in_queue: bool,
}

impl Song {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn new_with(options: SongOptions) -> Self {
        let song: Self = Object::builder()
            .property("in-playlist", options.in_playlist)
            .property("in-queue", options.in_queue)
            .build();
        if options.dnd {
            song.setup_drag_and_drop();
        }
        song
    }

    pub fn new_ghost() -> Self {
        Object::builder().property("is-ghost", true).build()
    }

    pub fn set_song_data(&self, song: &SongModel) {
        let imp = self.imp();
        imp.song_model.replace(Some(song.clone()));
        imp.title_label.set_label(&song.title());
        imp.album_label.set_label(&song.album());
        imp.artist_label.set_label(&song.artists_string());
        imp.duration_label
            .set_label(&format_duration(song.duration()));
    }

    pub fn set_starred(&self, is_favorite: bool) {
        let imp = self.imp();
        imp.song_star.set_active(is_favorite);
        imp.star_icon.set_icon_name(Some(if is_favorite {
            "starred-symbolic"
        } else {
            "non-starred-symbolic"
        }));
    }

    fn song_id(&self) -> String {
        self.imp()
            .song_model
            .borrow()
            .as_ref()
            .map(|m| m.id())
            .unwrap_or_default()
    }

    fn toggle_favorite(&self, is_favorite: bool) {
        let Some(song_model) = self.imp().song_model.borrow().clone() else {
            return;
        };
        song_model.set_favorite(is_favorite);
        self.set_starred(is_favorite);
        let item_id = song_model.id();
        let app = self.get_application();
        let backend = app.backend();
        spawn_tokio(
            async move {
                backend
                    .set_favorite(&item_id, &ItemType::Audio, is_favorite)
                    .await
            },
            glib::clone!(
                #[weak(rename_to = song)]
                self,
                #[weak]
                song_model,
                move |result| {
                    match result {
                        Ok(()) => song.get_application().refresh_favorites(true),
                        Err(err) => {
                            warn!("Failed to set favorite: {err}");
                            song_model.set_favorite(!is_favorite);
                            song.set_starred(!is_favorite);
                        }
                    }
                }
            ),
        );
    }

    pub fn set_playing(&self, playing: bool) {
        if playing {
            self.add_css_class("song-playing");
        } else {
            self.remove_css_class("song-playing");
        }
    }

    pub fn setup_drag_and_drop(&self) {
        let imp = self.imp();
        imp.drag_handle_box.set_visible(true);
        imp.drag_handle.set_cursor_from_name(Some("grab"));
        let drag_source = DragSource::new();

        // Drag Source
        drag_source.set_actions(DragAction::MOVE);
        drag_source.connect_prepare(glib::clone!(
            #[weak(rename_to = song)]
            self,
            #[upgrade_or_default]
            move |_, _, _| {
                let content = ContentProvider::for_value(&song.to_value());
                Some(content)
            }
        ));
        drag_source.connect_drag_begin(glib::clone!(
            #[weak(rename_to = song)]
            self,
            move |_, drag| {
                song.create_drag_icon(drag);
            }
        ));
        self.add_controller(drag_source);

        //Drag Target
        let drop_target = DropTarget::new(Song::static_type(), DragAction::MOVE);
        drop_target.set_preload(true); // deserialize data immediately over drop zone, fine for song lists

        drop_target.connect_drop(glib::clone!(
            #[weak(rename_to = target_song)]
            self,
            #[upgrade_or]
            false,
            move |_, value, _, _| {
                if let Ok(source_song) = value.get::<Song>() {
                    let source_index = source_song.position() as usize;
                    let target_index = target_song.position() as usize;
                    if source_index != target_index {
                        target_song.emit_by_name::<()>("widget-moved", &[&source_song.position()]);
                        return true;
                    }
                }
                false
            }
        ));

        self.add_controller(drop_target);
    }

    /// Build a ghost song widget which will appear when dragging a song.
    fn create_drag_icon(&self, drag: &Drag) {
        let drag_widget = gtk::ListBox::new();
        drag_widget.set_size_request(self.width(), self.height());
        let drag_song = Song::new_ghost();
        let di = drag_song.imp();
        di.title_label.set_label(&self.imp().title_label.label());
        di.album_label.set_label(&self.imp().album_label.label());
        di.artist_label.set_label(&self.imp().artist_label.label());
        di.number_label.set_label(&self.imp().number_label.label());
        di.duration_label
            .set_label(&self.imp().duration_label.label());
        drag_widget.set_opacity(0.9);
        drag_widget.append(&drag_song);
        let drag_icon = gtk::DragIcon::for_drag(drag);
        drag_icon.set_child(Some(&drag_widget));
    }

    fn setup_menu(&self) {
        let options = ContextActions {
            can_remove_from_playlist: self.imp().in_playlist.get(),
            in_queue: self.imp().in_queue.get(),
            action_prefix: "song".to_string(),
            go_to_album: true,
            go_to_artist: true,
            show_info_dialog: true,
            can_download: true,
        };
        let popover_menu = construct_menu(&options);
        self.imp().song_menu.set_popover(Some(&popover_menu));
        let action_group = self.create_action_group();
        self.insert_action_group(&options.action_prefix, Some(&action_group));

        popover_menu.connect_show(glib::clone!(
            #[weak(rename_to = song)]
            self,
            #[strong]
            action_group,
            move |_| {
                if let Some(song_model) = song.imp().song_model.borrow().clone() {
                    if let Some(action) = action_group
                        .lookup_action("delete_local")
                        .and_then(|a| a.downcast::<SimpleAction>().ok())
                    {
                        action.set_enabled(song_model.downloaded());
                    }
                    if let Some(action) = action_group
                        .lookup_action("download")
                        .and_then(|a| a.downcast::<SimpleAction>().ok())
                    {
                        action.set_enabled(!song_model.downloaded());
                    }
                }
            }
        ));
    }

    fn create_action_group(&self) -> SimpleActionGroup {
        let action_group = SimpleActionGroup::new();

        // TODO just make add_noarg_action take a reference to the group instead
        let add_noarg_action = |name: &str, handler: fn(&Self)| {
            let action = SimpleAction::new(name, None);
            action.connect_activate(glib::clone!(
                #[weak(rename_to = song)]
                self,
                move |_, _| handler(&song)
            ));
            action_group.add_action(&action);
        };

        let remove_from_playlist_action = gio::SimpleAction::new("remove_playlist", None);
        remove_from_playlist_action.connect_activate(glib::clone!(
            #[weak(rename_to = song)]
            self,
            move |_, _| song.on_remove_from_playlist()
        ));
        action_group.add_action(&remove_from_playlist_action);

        add_noarg_action("queue_next", Self::on_queue_next);
        add_noarg_action("queue_last", Self::on_queue_last);
        add_noarg_action("go_to_album", Self::on_go_to_album);
        add_noarg_action("go_to_artist", Self::on_go_to_artist);
        add_noarg_action("add_to_playlist_dialog", Self::on_add_to_playlist_dialog);
        add_noarg_action("show_info_dialog", Self::show_info_dialog);
        add_noarg_action("download", Self::download_song);
        add_noarg_action("delete_local", Self::delete_local_song);

        action_group
    }

    fn on_add_to_playlist(&self, playlist_id: String) {
        let song_id = self.song_id();
        let app = self.get_application();
        let backend = app.backend();
        let playlist_id = playlist_id.to_string();
        spawn_tokio(
            async move { backend.add_playlist_items(&playlist_id, &[song_id]).await },
            glib::clone!(
                #[weak(rename_to = song)]
                self,
                move |result| {
                    match result {
                        Ok(()) => {
                            song.toast(&tr("Added song to playlist"), None);
                            app.refresh_playlists(true);
                        }
                        Err(e) => {
                            song.toast(&tr("Failed to add song to playlist"), None);
                            warn!("Failed to add song to playlist: {}", e);
                        }
                    }
                }
            ),
        );
    }

    fn on_add_to_playlist_dialog(&self) {
        let playlists = self.get_application().playlists().borrow().clone();
        add_to_playlist_dialog(
            self.get_gtk_window().as_ref(),
            playlists,
            glib::clone!(
                #[weak(rename_to = song)]
                self,
                move |playlist_id| {
                    if let Some(playlist_id) = playlist_id {
                        song.on_add_to_playlist(playlist_id);
                    }
                }
            ),
        );
    }

    fn show_info_dialog(&self) {
        let song_id = self.song_id();
        let backend = self.get_application().backend();
        let uri = backend.get_stream_uri(&song_id);
        discover_stream_info(
            &uri,
            &song_id,
            &backend,
            glib::clone!(
                #[weak(rename_to = song)]
                self,
                move |stream_info| {
                    stream_info_dialog::show(song.get_gtk_window().as_ref(), stream_info);
                }
            ),
        );
    }

    fn download_song(&self) {
        let song_id = self.song_id();
        let backend = self.get_application().backend();
        let cache = self.get_application().media_cache();
        if let Some(cache) = cache
            && !cache.is_present(&song_id)
        {
            self.get_application().http_with_loading(
                async move { cache.get_media(&song_id, &backend).await },
                glib::clone!(
                    #[weak(rename_to = song)]
                    self,
                    move |result: Result<Vec<u8>, CacheError>| {
                        match result {
                            Ok(_) => {
                                song.get_application()
                                    .emit_by_name::<()>("downloads-updated", &[]);
                                song.toast(&tr("Song downloaded"), Some(1));
                            }
                            Err(err) => {
                                warn!("Failed to download song: {err}");
                                song.toast(&tr("Failed to download song"), None);
                            }
                        }
                    }
                ),
            );
        } else {
            self.toast(&tr("Song already downloaded"), None);
        }
    }

    fn delete_local_song(&self) {
        let song_id = self.song_id();
        let cache = self.get_application().media_cache();
        if let Some(cache) = cache
            && cache.is_present(&song_id)
        {
            spawn_tokio(
                async move { cache.remove_media(&song_id).await },
                glib::clone!(
                    #[weak(rename_to = song)]
                    self,
                    move |result: Result<(), CacheError>| {
                        match result {
                            Ok(_) => {
                                song.get_application()
                                    .emit_by_name::<()>("downloads-updated", &[]);
                                song.toast(&tr("Song deleted"), None);
                            }
                            Err(err) => {
                                warn!("Failed to delete song: {err}");
                                song.toast(&tr("Failed to delete song"), None);
                            }
                        }
                    }
                ),
            );
        }
    }

    fn on_remove_from_playlist(&self) {
        self.emit_by_name::<()>("remove-from-playlist", &[&self.song_id()]);
    }

    fn on_queue_next(&self) {
        let app = self.get_application();
        if let Some(audio_model) = app.audio_model()
            && let Some(song_model) = self.imp().song_model.borrow().clone()
        {
            audio_model.prepend_to_queue(vec![song_model]);
        }
    }

    fn on_queue_last(&self) {
        let app = self.get_application();
        if let Some(audio_model) = app.audio_model()
            && let Some(song_model) = self.imp().song_model.borrow().clone()
        {
            audio_model.append_to_queue(vec![song_model]);
        }
    }

    fn on_go_to_album(&self) {
        self.emit_by_name::<()>("album-clicked", &[&self.song_id()]);
    }

    fn on_go_to_artist(&self) {
        self.emit_by_name::<()>("artist-clicked", &[&self.song_id()]);
    }
}

impl Default for Song {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use std::{
        cell::{Cell, RefCell},
        sync::OnceLock,
    };

    use adw::subclass::prelude::*;
    use glib::{WeakRef, subclass::InitializingObject};
    use gtk::{
        CompositeTemplate,
        glib::{self, Properties, subclass::Signal},
        prelude::*,
    };

    use crate::{Application, models::SongModel};

    #[derive(CompositeTemplate, Default, Properties)]
    #[template(resource = "/io/m51/Gelly/ui/song.ui")]
    #[properties(wrapper_type = super::Song)]
    pub struct Song {
        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub number_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub artist_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub artist_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub album_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub duration_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub drag_handle_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub drag_handle: TemplateChild<gtk::Image>,
        #[template_child]
        pub song_menu: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub song_star: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub star_icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub download_icon: TemplateChild<gtk::Image>,

        #[property(get, set)]
        pub position: Cell<i32>,
        pub song_model: RefCell<Option<SongModel>>,
        pub favorite_indicator_handler:
            RefCell<Option<(glib::SignalHandlerId, WeakRef<Application>)>>,
        pub download_indicator_handler:
            RefCell<Option<(glib::SignalHandlerId, WeakRef<Application>)>>,
        pub download_indicator_binding: RefCell<Option<glib::Binding>>,
        #[property(get, construct_only, name = "in-playlist", default = false)]
        pub in_playlist: Cell<bool>,
        #[property(get, construct_only, name = "in-queue", default = false)]
        pub in_queue: Cell<bool>,

        #[property(get, construct_only, name = "is-ghost", default = false)]
        pub is_ghost: Cell<bool>,

        pub playing_indicator_handler: RefCell<Option<glib::SignalHandlerId>>,
        pub signal_handlers: RefCell<Vec<glib::SignalHandlerId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Song {
        const NAME: &'static str = "GellySong";
        type Type = super::Song;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for Song {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("widget-moved")
                        .param_types([i32::static_type()])
                        .build(),
                    Signal::builder("remove-from-playlist")
                        .param_types([String::static_type()])
                        .build(),
                    Signal::builder("artist-clicked")
                        .param_types([String::static_type()])
                        .build(),
                    Signal::builder("album-clicked")
                        .param_types([String::static_type()])
                        .build(),
                ]
            })
        }

        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_menu();

            // Sadly this can't be done in the template itself
            let orientation = if self.in_queue.get() {
                gtk::Orientation::Vertical
            } else {
                gtk::Orientation::Horizontal
            };
            self.artist_box.set_orientation(orientation);

            self.obj().connect_notify_local(
                Some("position"),
                glib::clone!(
                    #[weak(rename_to = song)]
                    self.obj(),
                    move |_, _| {
                        let position = song.position();
                        // Track number is position + 1 (position is 0-based)
                        song.imp()
                            .number_label
                            .set_label(&(position + 1).to_string());
                    }
                ),
            );

            self.song_star.connect_clicked(glib::clone!(
                #[weak(rename_to = song)]
                self.obj(),
                move |btn| {
                    song.toggle_favorite(btn.is_active());
                }
            ));
        }
    }
    impl BoxImpl for Song {}
    impl WidgetImpl for Song {}
}
