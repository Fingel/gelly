use crate::{
    async_utils::spawn_tokio,
    jellyfin::api::ImageType,
    library_utils::{albums_for_artist, play_artist, songs_for_artist},
    models::{AlbumModel, ArtistModel},
    ui::{
        album_detail::AlbumDetail,
        image_utils::bytes_to_texture,
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
        let library = self.get_application().library().clone();
        let albums: Vec<AlbumModel> = albums_for_artist(&self.id(), &library.borrow());
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
        // clear existing image
        self.imp().banner_image.set_paintable(None::<&Texture>);
        let Some(image_cache) = self.get_application().image_cache() else {
            return;
        };
        let Some(model) = self.get_model() else {
            return;
        };
        let jellyfin = self.get_application().jellyfin();
        let item_id = model.id();
        spawn_tokio(
            async move {
                image_cache
                    .get_image(&item_id, ImageType::Backdrop, &jellyfin)
                    .await
            },
            glib::clone!(
                #[weak(rename_to = artist_detail)]
                self,
                move |result| {
                    match result {
                        Ok(image_data) => {
                            artist_detail.imp().banner_overlay.set_height_request(300);
                            artist_detail.set_image(&image_data);
                        }
                        Err(err) => {
                            artist_detail.imp().banner_overlay.set_height_request(75);
                            warn!("Failed to load artist banner image: {}", err);
                        }
                    }
                }
            ),
        );
    }

    pub fn set_image(&self, image_data: &[u8]) {
        let image_data_copy = image_data.to_vec();
        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to=artist_detail)]
            self,
            async move {
                match bytes_to_texture(&image_data_copy).await {
                    Ok(texture) => {
                        artist_detail
                            .imp()
                            .banner_image
                            .set_paintable(Some(&texture));
                    }
                    Err(err) => {
                        warn!("Failed to load album image: {}", err);
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
            let library = app.library().clone();
            let playlist_id = playlist_id.to_string();
            let song_ids: Vec<String> = songs_for_artist(&id, &library.borrow())
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
            let library = app.library().clone();
            let id = model.id();
            let songs = songs_for_artist(&id, &library.borrow());
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
}

impl Default for ArtistDetail {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use std::cell::RefCell;

    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib::{self},
        prelude::*,
    };

    use crate::models::{AlbumModel, ArtistModel};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/artist_detail.ui")]
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

        pub model: RefCell<Option<ArtistModel>>,
        pub albums: RefCell<Vec<AlbumModel>>,
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
        }
    }
}
