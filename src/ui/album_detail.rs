use crate::{
    async_utils::spawn_tokio,
    i18n::{ngettext, tr},
    jellyfin::utils::format_duration,
    models::{AlbumModel, SongModel},
    ui::{
        music_context_menu::{ContextActions, add_to_playlist_dialog, construct_menu},
        page_traits::DetailPage,
        song::Song,
        song_utils::{self, connect_song_navigation},
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
use log::warn;

glib::wrapper! {
    pub struct AlbumDetail(ObjectSubclass<imp::AlbumDetail>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl DetailPage for AlbumDetail {
    type Model = AlbumModel;

    fn set_model(&self, model: &AlbumModel) {
        let imp = self.imp();
        imp.model.replace(Some(model.clone()));
        imp.name_label.set_text(&model.name());
        imp.artist_label.set_text(&model.artists_string());
        if model.year() > 0 {
            imp.year_label.set_text(&model.year().to_string());
        } else {
            imp.year_label.set_text(&tr("N/A"));
        }
        imp.album_image.set_item_id(&model.id(), None);
        let binding = model
            .bind_property("favorite", self, "favorite")
            .sync_create()
            .build();
        self.imp().favorite_binding.replace(Some(binding));
        self.pull_tracks();
    }

    fn get_model(&self) -> Option<Self::Model> {
        self.imp().model.borrow().clone()
    }
}

impl AlbumDetail {
    pub fn new() -> Self {
        Object::builder().build()
    }

    fn get_store(&self) -> &gio::ListStore {
        self.imp()
            .store
            .get()
            .expect("AlbumDetail store should be initialized")
    }

    fn setup_model(&self) {
        let imp = self.imp();
        if imp.store.get().is_some() {
            // Store is already set up with a model.
            return;
        }
        let store = imp
            .store
            .get_or_init(gio::ListStore::new::<SongModel>)
            .clone();
        let Some(audio_model) = self.get_application().audio_model() else {
            warn!("No audio model set, aborting");
            return;
        };
        let selection_model = gtk::NoSelection::new(Some(store));
        let factory = gtk::SignalListItemFactory::new();

        factory.connect_setup(move |_, list_item| {
            let song_widget = Song::new();
            let item = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Item should be a ListItem");

            item.bind_property("position", &song_widget, "position")
                .flags(glib::BindingFlags::SYNC_CREATE)
                .build();

            item.set_child(Some(&song_widget));
        });

        factory.connect_bind(glib::clone!(
            #[weak(rename_to = album_detail)]
            self,
            #[weak]
            audio_model,
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

                song_utils::connect_playing_indicator(&song_widget, &song_model, &audio_model);
                song_utils::connect_favorite_indicator(
                    &song_widget,
                    &song_model,
                    &album_detail.get_application(),
                );
                song_utils::connect_download_indicator(
                    &song_widget,
                    &song_model,
                    &album_detail.get_application(),
                );

                let nav_handlers =
                    connect_song_navigation(&song_widget, &album_detail.get_root_window());
                song_widget.imp().signal_handlers.replace(nav_handlers);
            }
        ));

        factory.connect_unbind(glib::clone!(
            #[weak]
            audio_model,
            move |_, list_item| {
                let list_item = list_item
                    .downcast_ref::<gtk::ListItem>()
                    .expect("Needs to be a ListItem");
                let song_widget = list_item
                    .child()
                    .and_downcast::<Song>()
                    .expect("Child has to be Song");

                song_utils::disconnect_playing_indicator(&song_widget, &audio_model);
                song_utils::disconnect_favorite_indicator(&song_widget);
                song_utils::disconnect_download_indicator(&song_widget);
                song_utils::disconnect_signal_handlers(&song_widget);
            }
        ));

        imp.track_list.set_model(Some(&selection_model));
        imp.track_list.set_factory(Some(&factory));
        imp.track_list.set_single_click_activate(true);
    }

    pub fn pull_tracks(&self) {
        self.setup_model(); // make sure store is initialized
        let songs = self.get_application().library().songs_for_album(&self.id());
        let store = self.get_store();
        store.remove_all();
        store.extend_from_slice(&songs);
        self.imp().songs.replace(songs);
        self.update_track_metadata();
    }

    pub fn song_selected(&self, index: usize) {
        let songs = self.imp().songs.borrow().clone();
        if let Some(audio_model) = self.get_application().audio_model() {
            audio_model.set_queue(songs, index, true);
        } else {
            self.toast(&tr("Audio model not initialized, please restart"), None);
            warn!("No audio model found");
        }
    }

    fn play_album(&self) {
        let songs = self.imp().songs.borrow().clone();
        if let Some(audio_model) = self.get_application().audio_model() {
            audio_model.set_queue(songs, 0, false);
        } else {
            self.toast(&tr("Audio model not initialized, please restart"), None);
            warn!("No audio model found");
        }
    }

    fn enqueue_album(&self, to_end: bool) {
        let songs = self.imp().songs.borrow().clone();
        if let Some(audio_model) = self.get_application().audio_model() {
            let song_cnt = songs.len();
            if to_end {
                audio_model.append_to_queue(songs);
            } else {
                audio_model.prepend_to_queue(songs);
            }
            self.toast(
                &ngettext(
                    "1 song added to queue",
                    "{} songs added to queue",
                    song_cnt as u32,
                )
                .replace("{}", &song_cnt.to_string()),
                None,
            );
        } else {
            self.toast(&tr("Audio model not initialized, please restart"), None);
            warn!("No audio model found");
        }
    }

    fn on_go_to_artist(&self) {
        if let Some(song) = self.imp().songs.borrow().first()
            && let Some(artist_model) = self.get_application().library().artist_for_item(&song.id())
        {
            let window = self.get_root_window();
            window.show_artist_detail(&artist_model);
        }
    }

    fn update_track_metadata(&self) {
        let songs = self.imp().songs.borrow();
        self.imp().track_count.set_text(&songs.len().to_string());
        let duration = songs.iter().map(|song| song.duration()).sum::<u64>();
        self.imp()
            .album_duration
            .set_text(&format_duration(duration));
    }

    fn setup_menu(&self) {
        let options = ContextActions {
            can_remove_from_playlist: false,
            in_queue: false,
            action_prefix: "album".to_string(),
            go_to_artist: true,
            go_to_album: false,
            show_info_dialog: false,
            can_download: false,
        };
        let popover_menu = construct_menu(&options);
        self.imp().action_menu.set_popover(Some(&popover_menu));
        let action_group = self.create_action_group();
        self.insert_action_group(&options.action_prefix, Some(&action_group));
    }

    fn create_action_group(&self) -> SimpleActionGroup {
        let action_group = gio::SimpleActionGroup::new();

        let add_to_playlist_action = gio::SimpleAction::new("add_to_playlist_dialog", None);
        add_to_playlist_action.connect_activate(glib::clone!(
            #[weak(rename_to = album)]
            self,
            move |_, _| {
                album.on_add_to_playlist_dialog();
            }
        ));
        action_group.add_action(&add_to_playlist_action);

        let queue_next_action = gio::SimpleAction::new("queue_next", None);
        queue_next_action.connect_activate(glib::clone!(
            #[weak(rename_to = album)]
            self,
            move |_, _| album.enqueue_album(false)
        ));
        action_group.add_action(&queue_next_action);

        let queue_last_action = gio::SimpleAction::new("queue_last", None);
        queue_last_action.connect_activate(glib::clone!(
            #[weak(rename_to = album)]
            self,
            move |_, _| album.enqueue_album(true)
        ));
        action_group.add_action(&queue_last_action);

        let on_copy_id_action = gio::SimpleAction::new("copy_id", None);
        on_copy_id_action.connect_activate(glib::clone!(
            #[weak(rename_to = album)]
            self,
            move |_, _| {
                let id = album.id();
                album.clipboard().set_text(&id);
                album.toast(&tr("Album ID copied to clipboard"), None);
            }
        ));
        action_group.add_action(&on_copy_id_action);

        let on_go_to_artist_action = gio::SimpleAction::new("go_to_artist", None);
        on_go_to_artist_action.connect_activate(glib::clone!(
            #[weak(rename_to = album)]
            self,
            move |_, _| album.on_go_to_artist()
        ));
        action_group.add_action(&on_go_to_artist_action);

        action_group
    }

    fn on_add_to_playlist(&self, playlist_id: String) {
        let song_ids = self
            .imp()
            .songs
            .borrow()
            .iter()
            .map(|song| song.id())
            .collect::<Vec<_>>();

        let app = self.get_application();
        let backend = app.backend();
        let playlist_id = playlist_id.to_string();
        spawn_tokio(
            async move { backend.add_playlist_items(&playlist_id, &song_ids).await },
            glib::clone!(
                #[weak(rename_to = album)]
                self,
                move |result| {
                    match result {
                        Ok(()) => {
                            album.toast(&tr("Added album to playlist"), None);
                            app.refresh_playlists(true);
                        }
                        Err(e) => {
                            album.toast(&tr("Failed to add album to playlist"), None);
                            warn!("Failed to add album to playlist: {}", e);
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
                #[weak(rename_to = album)]
                self,
                move |playlist_id| {
                    if let Some(playlist_id) = playlist_id {
                        album.on_add_to_playlist(playlist_id);
                    }
                }
            ),
        );
    }

    pub fn toggle_favorite(&self, is_favorite: bool) {
        let Some(model) = self.get_model() else {
            return;
        };
        let app = self.get_application();
        model.toggle_favorite(is_favorite, &app);
    }
}

impl Default for AlbumDetail {
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
        glib::{self, Properties},
        prelude::*,
    };

    use crate::{
        models::{AlbumModel, SongModel},
        ui::album_art::AlbumArt,
    };

    #[derive(CompositeTemplate, Default, Properties)]
    #[template(resource = "/io/m51/Gelly/ui/album_detail.ui")]
    #[properties(wrapper_type = super::AlbumDetail)]
    pub struct AlbumDetail {
        #[template_child]
        pub album_image: TemplateChild<AlbumArt>,
        #[template_child]
        pub name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub artist_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub year_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub track_list: TemplateChild<gtk::ListView>,
        #[template_child]
        pub track_count: TemplateChild<gtk::Label>,
        #[template_child]
        pub album_duration: TemplateChild<gtk::Label>,
        #[template_child]
        pub play_all: TemplateChild<gtk::Button>,
        #[template_child]
        pub action_menu: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub favorite_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub star_icon: TemplateChild<gtk::Image>,

        pub model: RefCell<Option<AlbumModel>>,
        pub songs: RefCell<Vec<SongModel>>,
        pub store: OnceCell<gio::ListStore>,
        pub favorite_binding: RefCell<Option<glib::Binding>>,
        #[property(get, set = Self::set_favorite)]
        favorite: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AlbumDetail {
        const NAME: &'static str = "GellyAlbumDetail";
        type Type = super::AlbumDetail;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }
    impl BoxImpl for AlbumDetail {}

    #[glib::derived_properties]
    impl ObjectImpl for AlbumDetail {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_menu();
            self.setup_signals();
        }
    }
    impl WidgetImpl for AlbumDetail {}

    impl AlbumDetail {
        fn setup_signals(&self) {
            self.track_list.connect_activate(glib::clone!(
                #[weak(rename_to=imp)]
                self,
                move |_track_list, position| {
                    imp.obj().song_selected(position as usize);
                }
            ));

            self.play_all.connect_clicked(glib::clone!(
                #[weak(rename_to=imp)]
                self,
                move |_| {
                    imp.obj().play_album();
                }
            ));

            self.favorite_button.connect_clicked(glib::clone!(
                #[weak(rename_to=imp)]
                self,
                move |button| {
                    imp.obj().toggle_favorite(button.is_active());
                }
            ));
        }

        fn set_favorite(&self, val: bool) {
            self.favorite.set(val);
            self.favorite_button.set_active(val);
            self.star_icon.set_icon_name(Some(if val {
                "starred-symbolic"
            } else {
                "non-starred-symbolic"
            }));
        }
    }
}
