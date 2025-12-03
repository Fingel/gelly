use glib::Object;
use gtk::{
    DragSource, DropTarget,
    gdk::{ContentProvider, Drag, DragAction},
    gio, glib,
    prelude::*,
    subclass::prelude::*,
};

use crate::{
    audio::model::AudioModel,
    jellyfin::utils::format_duration,
    library_utils::find_song,
    models::SongModel,
    ui::{drag_scrollable, widget_ext::WidgetApplicationExt},
};

glib::wrapper! {
    pub struct Song(ObjectSubclass<imp::Song>)
    @extends gtk::Widget, gtk::ListBoxRow,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Actionable;
}

impl Song {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn new_with(dnd: bool, in_playlist: bool, in_queue: bool) -> Self {
        let song: Self = Object::builder()
            .property("in-playlist", in_playlist)
            .property("in-queue", in_queue)
            .build();
        if dnd {
            song.setup_drag_and_drop();
        }
        song
    }

    pub fn new_ghost() -> Self {
        Object::builder().property("is-ghost", true).build()
    }

    pub fn set_song_data(&self, song: &SongModel) {
        let imp = self.imp();
        imp.item_id.replace(Some(song.id()));
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

    pub fn show_details(&self) {
        let imp = self.imp();
        imp.artist_label.set_visible(true);
        imp.album_label.set_visible(true);
    }

    pub fn setup_drag_and_drop(&self) {
        let imp = self.imp();
        imp.drag_handle_box.set_visible(true);
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
        // For auto scrolling
        drop_target.connect_enter(glib::clone!(
            #[weak(rename_to = target_song)]
            self,
            #[upgrade_or]
            DragAction::empty(),
            move |_, _, _| {
                target_song.grab_focus();

                glib::idle_add_local_once(glib::clone!(
                    #[weak]
                    target_song,
                    move || {
                        // Here be dragons, venture into this module at your own peril
                        drag_scrollable::handle_drag_scroll(&target_song);
                    }
                ));
                DragAction::MOVE
            }
        ));

        drop_target.connect_drop(glib::clone!(
            #[weak(rename_to = target_song)]
            self,
            #[upgrade_or]
            false,
            move |_, value, _, _| {
                drag_scrollable::clear_drag_state(&target_song);

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
        drag_song.show_details();
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
                        let my_id = song.imp().item_id.borrow().clone().unwrap_or_default();
                        song.set_playing(song_id == my_id);
                    }
                ),
            );
        }
    }

    fn setup_menu(&self) {
        let menu_model = self.create_menu_model();
        let popover_menu = gtk::PopoverMenu::from_model(Some(&menu_model));
        self.imp().song_menu.set_popover(Some(&popover_menu));

        self.setup_actions();
    }

    fn create_menu_model(&self) -> gio::Menu {
        let in_playlist = self.imp().in_playlist.get();
        let in_queue = self.imp().in_queue.get();
        let menu = gio::Menu::new();

        // Queue section
        if !in_queue {
            let queue_section = gio::Menu::new();
            queue_section.append(Some("Queue Next"), Some("song.queue_next"));
            queue_section.append(Some("Queue Last"), Some("song.queue_last"));
            menu.append_section(None, &queue_section);
        }
        // Playlist section
        let playlist_section = gio::Menu::new();
        playlist_section.append(Some("Add to Playlist…"), Some("song.add_playlist"));

        // Only add "Remove from Playlist" if we're actually in a playlist
        if in_playlist {
            playlist_section.append(Some("Remove from Playlist"), Some("song.remove_playlist"));
        }

        menu.append_section(None, &playlist_section);

        menu
    }

    fn setup_actions(&self) {
        let action_group = gio::SimpleActionGroup::new();

        let add_playlist_action = gio::SimpleAction::new("add_playlist", None);
        add_playlist_action.connect_activate(glib::clone!(
            #[weak(rename_to = song)]
            self,
            move |_, _| {
                song.on_add_to_playlist();
            }
        ));
        action_group.add_action(&add_playlist_action);

        let remove_playlist_action = gio::SimpleAction::new("remove_playlist", None);
        remove_playlist_action.connect_activate(glib::clone!(
            #[weak(rename_to = song)]
            self,
            move |_, _| {
                song.on_remove_from_playlist();
            }
        ));
        action_group.add_action(&remove_playlist_action);

        let queue_next_action = gio::SimpleAction::new("queue_next", None);
        queue_next_action.connect_activate(glib::clone!(
            #[weak(rename_to = song)]
            self,
            move |_, _| {
                song.on_queue_next();
            }
        ));
        action_group.add_action(&queue_next_action);

        let queue_last_action = gio::SimpleAction::new("queue_last", None);
        queue_last_action.connect_activate(glib::clone!(
            #[weak(rename_to = song)]
            self,
            move |_, _| {
                song.on_queue_last();
            }
        ));
        action_group.add_action(&queue_last_action);

        self.insert_action_group("song", Some(&action_group));
    }

    fn on_add_to_playlist(&self) {
        if let Some(song_id) = self.imp().item_id.borrow().clone() {
            self.emit_by_name::<()>("add-to-playlist-requested", &[&song_id]);
        }
    }

    fn on_remove_from_playlist(&self) {
        if let Some(song_id) = self.imp().item_id.borrow().clone() {
            dbg!("remove from playlist");
            self.emit_by_name::<()>("remove-from-playlist-requested", &[&song_id]);
        }
    }

    fn on_queue_next(&self) {
        if let Some(song_id) = self.imp().item_id.borrow().clone() {
            let app = self.get_application();
            if let Some(audio_model) = app.audio_model()
                && let Some(song) = find_song(&song_id, &app)
            {
                audio_model.prepend_to_queue(vec![song]);
            }
        }
    }

    fn on_queue_last(&self) {
        if let Some(song_id) = self.imp().item_id.borrow().clone() {
            let app = self.get_application();
            if let Some(audio_model) = app.audio_model()
                && let Some(song) = find_song(&song_id, &app)
            {
                audio_model.append_to_queue(vec![song]);
            }
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
        pub artist_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub album_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub duration_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub playing_icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub drag_handle_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub song_menu: TemplateChild<gtk::MenuButton>,

        pub item_id: RefCell<Option<String>>, // TODO: Why is this an option?

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
                    Signal::builder("add-to-playlist-requested")
                        .param_types([String::static_type()])
                        .build(),
                    Signal::builder("remove-from-playlist-requested")
                        .param_types([String::static_type()])
                        .build(),
                    Signal::builder("queue-next-requested")
                        .param_types([String::static_type()])
                        .build(),
                    Signal::builder("queue-last-requested")
                        .param_types([String::static_type()])
                        .build(),
                ]
            })
        }

        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_menu();
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
