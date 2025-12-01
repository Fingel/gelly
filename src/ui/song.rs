use glib::Object;
use gtk::{
    DragSource, DropTarget,
    gdk::{ContentProvider, Drag, DragAction},
    gio, glib,
    prelude::*,
    subclass::prelude::*,
};

use crate::{
    audio::model::AudioModel, jellyfin::utils::format_duration, models::SongModel,
    ui::widget_ext::WidgetApplicationExt,
};

glib::wrapper! {
    pub struct Song(ObjectSubclass<imp::Song>)
    @extends gtk::Widget, gtk::ListBoxRow,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Actionable;
}

impl Song {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn new_with_dnd() -> Self {
        let song = Self::new();
        song.setup_drag_and_drop();
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
        imp.song_actions_box.set_visible(true);
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
        // TODO: This seems like a giant hack
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
                        if let Some(listbox) = target_song
                            .parent()
                            .and_then(|p| p.downcast::<gtk::ListBox>().ok())
                        {
                            let current_index = target_song.index();

                            // Try to focus the next widget to ensure it's visible
                            if let Some(next_row) = listbox.row_at_index(current_index + 1) {
                                next_row.grab_focus();
                            }
                        }
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
        pub song_actions_box: TemplateChild<gtk::Box>,

        pub item_id: RefCell<Option<String>>,

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
                ]
            })
        }

        fn constructed(&self) {
            self.parent_constructed();
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
