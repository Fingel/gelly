use crate::{
    application::Application,
    async_utils::spawn_tokio,
    library_utils::artists_from_library,
    models::ArtistModel,
    ui::{artist::Artist, widget_ext::WidgetApplicationExt},
};
use glib::Object;
use gtk::{
    ListItem, gio,
    glib::{self, object::Cast},
    prelude::*,
    subclass::prelude::*,
};
use log::{debug, warn};

glib::wrapper! {
    pub struct ArtistList(ObjectSubclass<imp::ArtistList>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup;
}

impl ArtistList {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn pull_artists(&self) {
        let library = self.get_application().library().clone();
        let artists = artists_from_library(&library.borrow());
        let store = self
            .imp()
            .store
            .get()
            .expect("AlbumList store should be initialized.");
        store.remove_all();
        for artist in artists {
            store.append(&artist);
        }
    }

    pub fn activate_artist(&self, index: u32) {
        let store = self
            .imp()
            .store
            .get()
            .expect("ArtistList store should be initialized.");
        let artist_model = store
            .item(index)
            .expect("Item index invalid")
            .downcast_ref::<ArtistModel>()
            .expect("Item should be an ArtistModel")
            .clone();
        let window = self.get_root_window();
        window.show_artist_detail(&artist_model);
    }

    pub fn setup_library_connection(&self) {
        let app = self.get_application();
        app.connect_closure(
            "library-refreshed",
            false,
            glib::closure_local!(
                #[weak(rename_to = artist_list)]
                self,
                move |_app: Application| {
                    artist_list.pull_artists();
                }
            ),
        );
    }

    fn setup_model(&self) {
        let imp = self.imp();
        let store = gio::ListStore::new::<ArtistModel>();
        imp.store
            .set(store.clone())
            .expect("ArtistList store should only be set once.");

        let selection_model = gtk::SingleSelection::new(Some(store));
        let factory = gtk::SignalListItemFactory::new();

        factory.connect_setup(move |_, list_item| {
            let placeholder = Artist::new();
            let item = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be a ListItem");
            item.set_child(Some(&placeholder))
        });

        factory.connect_bind(glib::clone!(
            #[weak(rename_to = album_list)]
            self,
            move |_, list_item| {
                let list_item = list_item
                    .downcast_ref::<gtk::ListItem>()
                    .expect("Needs to be a ListItem");
                let artist_model = list_item
                    .item()
                    .and_downcast::<ArtistModel>()
                    .expect("Item should be an ArtistData");
                let artist_widget = list_item
                    .child()
                    .and_downcast::<Artist>()
                    .expect("Child has to be an Artist");

                artist_widget.set_artist_model(&artist_model);

                // Async image loading
                album_list.load_image(&artist_model, &artist_widget);
            }
        ));

        imp.grid_view.set_model(Some(&selection_model));
        imp.grid_view.set_factory(Some(&factory));
    }

    fn load_image(&self, artist_model: &ArtistModel, artist_widget: &Artist) {
        if artist_model.image_loading() || artist_model.image_loaded() {
            debug!("Image is already loaded");
            return;
        }

        let Some(image_cache) = self.get_application().image_cache() else {
            warn!("Image cache not available");
            return;
        };

        let item_id = artist_model.id();
        let jellyfin = self.get_application().jellyfin();
        artist_model.set_image_loading(true);
        artist_widget.set_loading(true);

        spawn_tokio(
            async move { image_cache.get_image(&item_id, &jellyfin).await },
            glib::clone!(
                #[weak]
                artist_model,
                #[weak]
                artist_widget,
                move |result| {
                    artist_model.set_image_loading(false);
                    artist_model.set_image_loaded(true);
                    match result {
                        Ok(image_data) => {
                            artist_widget.set_loading(false);
                            artist_widget.set_image(&image_data);
                            artist_model.set_image_data(image_data);
                        }
                        Err(err) => {
                            warn!(
                                "Failed to load album art for {}: {}",
                                artist_model.id(),
                                err
                            );
                            artist_widget.show_error();
                        }
                    }
                }
            ),
        );
    }
}

impl Default for ArtistList {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use std::cell::OnceCell;

    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{CompositeTemplate, gio, glib};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/artist_list.ui")]
    pub struct ArtistList {
        #[template_child]
        pub grid_view: TemplateChild<gtk::GridView>,
        pub store: OnceCell<gio::ListStore>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ArtistList {
        const NAME: &'static str = "GellyArtistList";
        type Type = super::ArtistList;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }
    impl ObjectImpl for ArtistList {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_model();

            self.grid_view.connect_activate(glib::clone!(
                #[weak(rename_to = artist_list)]
                self.obj(),
                move |_, position| {
                    artist_list.activate_artist(position);
                }
            ));
        }
    }
    impl WidgetImpl for ArtistList {}
    impl BoxImpl for ArtistList {}
}
