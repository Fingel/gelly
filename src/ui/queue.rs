use crate::{
    async_utils::spawn_tokio,
    ui::{
        page_traits::TopPage,
        playlist_dialogs,
        song::{Song, SongOptions},
        song_utils::connect_song_navigation,
        widget_ext::WidgetApplicationExt,
    },
};
use glib::Object;
use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use log::{error, warn};

glib::wrapper! {
    pub struct Queue(ObjectSubclass<imp::Queue>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl TopPage for Queue {
    fn can_search(&self) -> bool {
        false
    }

    fn can_sort(&self) -> bool {
        false
    }

    fn can_new(&self) -> bool {
        false
    }

    fn reveal_search_bar(&self, _visible: bool) {}
    fn reveal_sort_bar(&self, _visible: bool) {}
    fn play_selected(&self) {}
}

impl Queue {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn display_queue(&self) {
        if let Some(audio_model) = self.get_application().audio_model() {
            let tracks = audio_model.queue();
            self.imp().track_list.remove_all();
            if tracks.is_empty() {
                self.set_empty(true);
            } else {
                self.set_empty(false);
                let current_track = audio_model.current_song_id();
                for track in &tracks {
                    let song_widget = Song::new_with(SongOptions {
                        dnd: true,
                        in_queue: true,
                        in_playlist: false,
                    });
                    song_widget.set_song_data(track);
                    // connect navigation signals
                    connect_song_navigation(&song_widget, &self.get_root_window());
                    self.imp().track_list.append(&song_widget);
                    if track.id() == current_track {
                        song_widget.set_playing(true);
                    }
                    song_widget.connect_closure(
                        "widget-moved",
                        false,
                        glib::closure_local!(
                            #[weak(rename_to= queue)]
                            self,
                            move |song_widget: Song, source_index: i32| {
                                let target_index = song_widget.index() as usize;
                                let source_index = source_index as usize;
                                queue.handle_song_moved(source_index, target_index)
                            }
                        ),
                    );
                }
            }
        } else {
            self.toast("Audio model not initialized, please restart", None);
            warn!("No audio model found");
        }
    }

    fn handle_song_moved(&self, source_index: usize, target_index: usize) {
        if source_index == target_index {
            return;
        }
        let Some(audio_model) = self.get_application().audio_model() else {
            warn!("No audio model found");
            return;
        };
        let mut songs = audio_model.queue();
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
        songs.remove(source_index);
        songs.insert(target_index, song_being_moved);
        audio_model.replace_queue(songs);
        if source_index == audio_model.queue_index() as usize {
            audio_model.set_queue_index(target_index as i32);
        }

        let track_list = &self.imp().track_list;
        if let Some(source_row) = track_list.row_at_index(source_index as i32) {
            // Remove and reinsert the widget
            track_list.remove(&source_row);
            track_list.insert(&source_row, target_index as i32);
        }
    }

    fn set_empty(&self, empty: bool) {
        self.imp().empty.set_visible(empty);
        self.imp().queue_box.set_visible(!empty);
    }

    pub fn song_selected(&self, index: usize) {
        if let Some(audio_model) = self.get_application().audio_model() {
            audio_model.play_song(index);
        }
    }

    pub fn clear_queue(&self) {
        if let Some(audio_model) = self.get_application().audio_model() {
            audio_model.clear_queue();
            self.display_queue();
        }
    }

    pub fn save_as_playlist(&self) {
        let window = self.get_root_window();
        let Some(audio_model) = self.get_application().audio_model() else {
            warn!("No audio model found");
            return;
        };
        let song_ids: Vec<String> = audio_model.queue().iter().map(|song| song.id()).collect();
        playlist_dialogs::new_playlist(
            Some(&window),
            glib::clone!(
                #[weak (rename_to = queue)]
                self,
                move |playlist_name| {
                    queue.create_new_playlist(playlist_name, song_ids.clone());
                }
            ),
        );
    }

    fn create_new_playlist(&self, name: String, song_ids: Vec<String>) {
        let app = self.get_application();
        let jellyfin = app.jellyfin();
        spawn_tokio(
            async move { jellyfin.new_playlist(&name, song_ids).await },
            glib::clone!(
                #[weak (rename_to = queue)]
                self,
                move |result| {
                    match result {
                        Ok(_id) => {
                            app.refresh_playlists(true);
                            queue.toast("Playlist created", None);
                        }
                        Err(err) => {
                            queue.toast(&format!("Failed to create playlist: {}", err), None);
                            error!("Failed to create playlist: {}", err);
                        }
                    }
                }
            ),
        );
    }
}

impl Default for Queue {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib::{self},
        prelude::*,
    };

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/queue.ui")]
    pub struct Queue {
        #[template_child]
        pub empty: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub track_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub queue_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub clear_queue: TemplateChild<gtk::Button>,
        #[template_child]
        pub save_as_playlist: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Queue {
        const NAME: &'static str = "GellyQueue";
        type Type = super::Queue;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            crate::ui::auto_scroll_window::AutoScrollWindow::static_type();
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl BoxImpl for Queue {}
    impl WidgetImpl for Queue {}
    impl ObjectImpl for Queue {
        fn constructed(&self) {
            self.parent_constructed();
            self.setup_signals();
            self.obj().connect_map(glib::clone!(
                #[weak(rename_to = queue)]
                self.obj(),
                move |_| {
                    queue.display_queue();
                }
            ));
        }
    }

    impl Queue {
        fn setup_signals(&self) {
            self.track_list.connect_row_activated(glib::clone!(
                #[weak(rename_to=imp)]
                self,
                move |_track_list, row| {
                    let index = row.index();
                    imp.obj().song_selected(index as usize);
                }
            ));

            self.clear_queue.connect_clicked(glib::clone!(
                #[weak(rename_to=imp)]
                self,
                move |_| {
                    imp.obj().clear_queue();
                }
            ));

            self.save_as_playlist.connect_clicked(glib::clone!(
                #[weak(rename_to=imp)]
                self,
                move |_| {
                    imp.obj().save_as_playlist();
                }
            ));
        }
    }
}
