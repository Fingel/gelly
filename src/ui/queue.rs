use crate::{
    async_utils::spawn_tokio,
    models::SongModel,
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
            let store = self.imp().store.get().expect("Queue store should exist"); // todo make a method
            store.remove_all();

            if tracks.is_empty() {
                self.set_empty(true);
            } else {
                self.set_empty(false);
                for track in &tracks {
                    store.append(track);
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

        let store = self.imp().store.get().expect("Queue store should exist");
        store.remove_all();
        for track in audio_model.queue() {
            store.append(&track);
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

    fn setup_model(&self) {
        let imp = self.imp();
        let store = gio::ListStore::new::<SongModel>();
        imp.store
            .set(store.clone())
            .expect("Store should only be set once");
        let selection_model = gtk::NoSelection::new(Some(store));
        let factory = gtk::SignalListItemFactory::new();

        factory.connect_setup(move |_, list_item| {
            let placeholder = Song::new_with(SongOptions {
                dnd: true,
                in_playlist: false,
                in_queue: true,
            });
            let item = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be a ListItem");

            // Keep track number in sync
            item.bind_property("position", &placeholder, "position")
                .flags(glib::BindingFlags::SYNC_CREATE)
                .build();

            item.set_child(Some(&placeholder))
        });

        factory.connect_bind(glib::clone!(
            #[weak(rename_to = queue)]
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

                song_widget.set_song_data(&song_model);

                // Mark of this song is playing
                if let Some(audio_model) = queue.get_application().audio_model() {
                    let current_track = audio_model.current_song_id();
                    song_widget.set_playing(song_model.id() == current_track);
                }

                connect_song_navigation(&song_widget, &queue.get_root_window());

                song_widget.connect_closure(
                    "widget-moved",
                    false,
                    glib::closure_local!(move |song_widget: Song, source_index: i32| {
                        let target_index = song_widget.position() as usize;
                        let source_index = source_index as usize;
                        queue.handle_song_moved(source_index, target_index)
                    }),
                );
            }
        ));

        imp.track_list.set_model(Some(&selection_model));
        imp.track_list.set_factory(Some(&factory));
        imp.track_list.set_single_click_activate(true);
    }
}

impl Default for Queue {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use std::cell::OnceCell;

    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate, gio,
        glib::{self},
        prelude::*,
    };

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/queue.ui")]
    pub struct Queue {
        #[template_child]
        pub empty: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub track_list: TemplateChild<gtk::ListView>,
        #[template_child]
        pub queue_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub clear_queue: TemplateChild<gtk::Button>,
        #[template_child]
        pub save_as_playlist: TemplateChild<gtk::Button>,
        pub store: OnceCell<gio::ListStore>,
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
            self.obj().setup_model();
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
            self.track_list.connect_activate(glib::clone!(
                #[weak(rename_to=imp)]
                self,
                move |_, position| {
                    imp.obj().song_selected(position as usize);
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
