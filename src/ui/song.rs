use glib::Object;
use gtk::{
    DragSource, DropTarget,
    gdk::{ContentProvider, Drag, DragAction},
    gio::SimpleActionGroup,
    glib,
    prelude::*,
    subclass::prelude::*,
};
use log::warn;

use crate::{
    async_utils::spawn_tokio,
    audio::model::AudioModel,
    jellyfin::utils::format_duration,
    models::SongModel,
    ui::{
        music_context_menu::{ContextActions, construct_menu, create_actiongroup},
        widget_ext::WidgetApplicationExt,
    },
};

glib::wrapper! {
    pub struct Song(ObjectSubclass<imp::Song>)
    @extends gtk::Widget, gtk::ListBoxRow,
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
        imp.item_id.replace(song.id());
        imp.title_label.set_label(&song.title());
        imp.album_label.set_label(&song.album());
        imp.artist_label.set_label(&song.artists_string());
        imp.number_label.set_label(&song.track_number().to_string());
        imp.duration_label
            .set_label(&format_duration(song.duration()));
    }

    pub fn set_playing(&self, playing: bool) {
        self.imp().playing_icon.set_visible(playing);
    }

    pub fn set_track_number(&self, track_number: u32) {
        self.imp().number_label.set_label(&track_number.to_string());
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
                    let source_index = source_song.index() as usize;
                    let target_index = target_song.index() as usize;
                    if source_index != target_index {
                        target_song.emit_by_name::<()>("widget-moved", &[&source_song.index()]);
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

    fn listen_for_song_changes(&self) {
        if self.is_ghost() {
            return; // Don't connect for ghost (drag and drop) widgets
        }

        if let Some(audio_model) = self.get_application().audio_model() {
            audio_model.connect_closure(
                "song-changed",
                false,
                glib::closure_local!(
                    #[weak(rename_to = song)]
                    self,
                    move |_audio_model: AudioModel, song_id: &str| {
                        let my_id = song.imp().item_id.borrow().clone();
                        song.set_playing(song_id == my_id);
                    }
                ),
            );
        }
    }

    fn setup_menu(&self) {
        let options = ContextActions {
            can_remove_from_playlist: self.imp().in_playlist.get(),
            in_queue: self.imp().in_queue.get(),
            action_prefix: "song".to_string(),
        };
        let popover_menu = construct_menu(
            &options,
            glib::clone!(
                #[weak(rename_to = song)]
                self,
                #[upgrade_or_default]
                move || song.get_application().playlists().borrow().clone()
            ),
        );
        self.imp().song_menu.set_popover(Some(&popover_menu));
        let action_group = self.create_action_group();
        self.insert_action_group(&options.action_prefix, Some(&action_group));
    }

    fn create_action_group(&self) -> SimpleActionGroup {
        let on_add_to_playlist = glib::clone!(
            #[weak(rename_to = song)]
            self,
            move |playlist_id| {
                song.on_add_to_playlist(playlist_id);
            }
        );

        let on_remove_from_playlist = glib::clone!(
            #[weak(rename_to = song)]
            self,
            move || {
                song.on_remove_from_playlist();
            }
        );

        let on_queue_next = glib::clone!(
            #[weak(rename_to = song)]
            self,
            move || {
                song.on_queue_next();
            }
        );

        let on_queue_last = glib::clone!(
            #[weak(rename_to = song)]
            self,
            move || {
                song.on_queue_last();
            }
        );

        create_actiongroup(
            Some(on_add_to_playlist),
            Some(on_remove_from_playlist),
            Some(on_queue_next),
            Some(on_queue_last),
        )
    }

    fn setup_clickable_labels(&self) {
        let imp = self.imp();
        // Set pointer cursor for better discoverability
        imp.artist_button.set_cursor_from_name(Some("pointer"));
        imp.album_button.set_cursor_from_name(Some("pointer"));

        imp.artist_button.connect_clicked(glib::clone!(
            #[weak(rename_to = song)]
            self,
            move |_| {
                song.emit_by_name::<()>("artist-clicked", &[&song.imp().item_id.borrow().clone()]);
            }
        ));

        imp.album_button.connect_clicked(glib::clone!(
            #[weak(rename_to = song)]
            self,
            move |_| {
                song.emit_by_name::<()>("album-clicked", &[&song.imp().item_id.borrow().clone()]);
            }
        ));
    }

    fn on_add_to_playlist(&self, playlist_id: String) {
        let song_id = self.imp().item_id.borrow().clone();
        let app = self.get_application();
        let jellyfin = app.jellyfin();
        let playlist_id = playlist_id.to_string();
        spawn_tokio(
            async move { jellyfin.add_playlist_items(&playlist_id, &[song_id]).await },
            glib::clone!(
                #[weak(rename_to = song)]
                self,
                move |result| {
                    match result {
                        Ok(()) => {
                            song.toast("Added song to playlist", None);
                            app.refresh_playlists(true);
                        }
                        Err(e) => {
                            song.toast("Failed to add song to playlist", None);
                            warn!("Failed to add song to playlist: {}", e);
                        }
                    }
                }
            ),
        );
    }

    fn on_remove_from_playlist(&self) {
        let song_id = self.imp().item_id.borrow().clone();
        self.emit_by_name::<()>("remove-from-playlist", &[&song_id]);
    }

    fn on_queue_next(&self) {
        let song_id = self.imp().item_id.borrow().clone();
        let app = self.get_application();
        if let Some(audio_model) = app.audio_model()
            && let Some(song) = app
                .library()
                .borrow()
                .iter()
                .find(|song| song.id == song_id)
                .map(SongModel::from)
        {
            audio_model.prepend_to_queue(vec![song]);
        }
    }

    fn on_queue_last(&self) {
        let song_id = self.imp().item_id.borrow().clone();
        let app = self.get_application();
        if let Some(audio_model) = app.audio_model()
            && let Some(song) = app
                .library()
                .borrow()
                .iter()
                .find(|song| song.id == song_id)
                .map(SongModel::from)
        {
            audio_model.append_to_queue(vec![song]);
        }
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
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib::{self, Properties, subclass::Signal},
        prelude::*,
    };

    #[derive(CompositeTemplate, Default, Properties)]
    #[template(resource = "/io/m51/Gelly/ui/song.ui")]
    #[properties(wrapper_type = super::Song)]
    pub struct Song {
        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub number_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub artist_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub artist_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub album_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub album_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub duration_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub playing_icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub drag_handle_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub drag_handle: TemplateChild<gtk::Image>,
        #[template_child]
        pub song_menu: TemplateChild<gtk::MenuButton>,

        pub item_id: RefCell<String>,
        #[property(get, construct_only, name = "in-playlist", default = false)]
        pub in_playlist: Cell<bool>,
        #[property(get, construct_only, name = "in-queue", default = false)]
        pub in_queue: Cell<bool>,

        #[property(get, construct_only, name = "is-ghost", default = false)]
        pub is_ghost: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Song {
        const NAME: &'static str = "GellySong";
        type Type = super::Song;
        type ParentType = gtk::ListBoxRow;

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
            self.obj().setup_clickable_labels();
            self.obj().connect_map(glib::clone!(
                #[weak(rename_to = song)]
                self.obj(),
                move |_| {
                    song.listen_for_song_changes();
                }
            ));
        }
    }
    impl ListBoxRowImpl for Song {}
    impl WidgetImpl for Song {}
}
