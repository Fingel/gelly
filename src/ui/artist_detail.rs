use crate::{
    async_utils::spawn_tokio,
    jellyfin::api::ImageType,
    library_utils::play_artist,
    models::{AlbumModel, ArtistModel},
    ui::{
        album_detail::AlbumDetail,
        music_context_menu::{ContextActions, construct_menu, create_actiongroup},
        page_traits::DetailPage,
        widget_ext::WidgetApplicationExt,
    },
};
use glib::Object;
use gtk::{
    gdk::Texture,
    gio::{self, SimpleActionGroup},
    glib,
    prelude::*,
    subclass::prelude::*,
};
use log::warn;

glib::wrapper! {
    pub struct ArtistDetail(ObjectSubclass<imp::ArtistDetail>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl DetailPage for ArtistDetail {
    type Model = ArtistModel;

    fn set_model(&self, model: &ArtistModel) {
        let imp = self.imp();
        imp.model.replace(Some(model.clone()));
        imp.artist_name.set_text(&model.name());
        self.load_banner_image();
        let binding = model
            .bind_property("favorite", self, "favorite")
            .sync_create()
            .build();
        self.imp().favorite_binding.replace(Some(binding));
        self.pull_albums();
    }

    fn get_model(&self) -> Option<Self::Model> {
        self.imp().model.borrow().clone()
    }
}

impl ArtistDetail {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn pull_albums(&self) {
        let albums: Vec<AlbumModel> = self
            .get_application()
            .library()
            .albums_for_artist(&self.id());
        self.imp().albums.replace(albums);
        while let Some(child) = self.imp().albums_box.first_child() {
            self.imp().albums_box.remove(&child);
        }
        for album in self.imp().albums.borrow().iter() {
            let album_widget = AlbumDetail::new();
            self.imp().albums_box.append(&album_widget);
            album_widget.set_model(album);
            album_widget.imp().artist_label.set_visible(false);
        }
    }

    pub fn load_banner_image(&self) {
        self.imp().banner_image.set_paintable(None::<&Texture>);
        let Some(image_cache) = self.get_application().image_cache() else {
            return;
        };
        let Some(model) = self.get_model() else {
            return;
        };
        let jellyfin = self.get_application().jellyfin();
        let item_id = model.id();
        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to = artist_detail)]
            self,
            async move {
                match image_cache
                    .get_texture(&item_id, ImageType::Backdrop, &jellyfin)
                    .await
                {
                    Ok(texture) => {
                        artist_detail.imp().banner_overlay.set_height_request(300);
                        artist_detail
                            .imp()
                            .banner_image
                            .set_paintable(Some(&texture));
                    }
                    Err(err) => {
                        artist_detail.imp().banner_overlay.set_height_request(75);
                        warn!("Failed to load artist banner image: {}", err);
                    }
                }
            }
        ));
    }

    pub fn play_artist(&self) {
        if let Some(model) = self.imp().model.borrow().as_ref() {
            play_artist(&model.id(), &self.get_application());
        }
    }

    fn setup_menu(&self) {
        let options = ContextActions {
            can_remove_from_playlist: false,
            in_queue: false,
            action_prefix: "artist".to_string(),
        };
        let popover_menu = construct_menu(
            &options,
            glib::clone!(
                #[weak(rename_to = artist)]
                self,
                #[upgrade_or_default]
                move || artist.get_application().playlists().borrow().clone()
            ),
        );
        self.imp().action_menu.set_popover(Some(&popover_menu));
        let action_group = self.create_action_group();
        self.insert_action_group(&options.action_prefix, Some(&action_group));
    }

    fn create_action_group(&self) -> SimpleActionGroup {
        let on_add_to_playlist = glib::clone!(
            #[weak(rename_to = artist)]
            self,
            move |playlist_id| {
                artist.on_add_to_playlist(playlist_id);
            }
        );

        let on_queue_next = glib::clone!(
            #[weak(rename_to = artist)]
            self,
            move || {
                artist.enqueue_artist(false);
            }
        );

        let on_queue_last = glib::clone!(
            #[weak(rename_to = artist)]
            self,
            move || {
                artist.enqueue_artist(true);
            }
        );

        create_actiongroup(
            Some(on_add_to_playlist),
            None::<fn()>,
            Some(on_queue_next),
            Some(on_queue_last),
        )
    }

    fn on_add_to_playlist(&self, playlist_id: String) {
        if let Some(model) = self.get_model() {
            let id = model.id();
            let app = self.get_application();
            let jellyfin = app.jellyfin();
            let playlist_id = playlist_id.to_string();
            let song_ids: Vec<String> = app
                .library()
                .songs_for_artist(&id)
                .iter()
                .map(|song| song.id().to_string())
                .collect();
            spawn_tokio(
                async move { jellyfin.add_playlist_items(&playlist_id, &song_ids).await },
                glib::clone!(
                    #[weak(rename_to = artist)]
                    self,
                    move |result| {
                        match result {
                            Ok(()) => {
                                artist.toast("Added artist to playlist", None);
                                app.refresh_playlists(true);
                            }
                            Err(e) => {
                                artist.toast("Failed to add artist to playlist", None);
                                warn!("Failed to add artist to playlist: {}", e);
                            }
                        }
                    }
                ),
            );
        }
    }

    fn enqueue_artist(&self, to_end: bool) {
        if let Some(model) = self.get_model() {
            let app = self.get_application();
            let id = model.id();
            let songs = app.library().songs_for_artist(&id);
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
    }

    pub fn toggle_favorite(&self, is_favorite: bool) {
        let Some(model) = self.get_model() else {
            return;
        };
        let app = self.get_application();
        model.toggle_favorite(is_favorite, &app);
        app.refresh_favorites(true);
    }
}

impl Default for ArtistDetail {
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
        glib::{self, Properties},
        prelude::*,
    };

    use crate::models::{AlbumModel, ArtistModel};

    #[derive(CompositeTemplate, Default, Properties)]
    #[template(resource = "/io/m51/Gelly/ui/artist_detail.ui")]
    #[properties(wrapper_type = super::ArtistDetail)]
    pub struct ArtistDetail {
        #[template_child]
        pub banner_overlay: TemplateChild<gtk::Overlay>,
        #[template_child]
        pub artist_name: TemplateChild<gtk::Label>,
        #[template_child]
        pub banner_image: TemplateChild<gtk::Picture>,
        #[template_child]
        pub albums_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub play_all: TemplateChild<gtk::Button>,
        #[template_child]
        pub action_menu: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub favorite_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub star_icon: TemplateChild<gtk::Image>,

        pub model: RefCell<Option<ArtistModel>>,
        pub albums: RefCell<Vec<AlbumModel>>,
        pub favorite_binding: RefCell<Option<glib::Binding>>,
        #[property(get, set = Self::set_favorite)]
        favorite: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ArtistDetail {
        const NAME: &'static str = "GellyArtistDetail";
        type Type = super::ArtistDetail;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl BoxImpl for ArtistDetail {}

    #[glib::derived_properties]
    impl ObjectImpl for ArtistDetail {
        fn constructed(&self) {
            self.parent_constructed();
            self.setup_signals();
            self.obj().setup_menu();
        }
    }
    impl WidgetImpl for ArtistDetail {}
    impl ArtistDetail {
        fn setup_signals(&self) {
            self.play_all.connect_clicked(glib::clone!(
                #[weak(rename_to=imp)]
                self,
                move |_| {
                    imp.obj().play_artist();
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
