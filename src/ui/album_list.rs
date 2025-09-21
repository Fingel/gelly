use crate::{
    async_utils::spawn_tokio,
    library_utils::albums_from_library,
    models::album_data::AlbumData,
    ui::{album::Album, widget_ext::WidgetApplicationExt},
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
    pub struct AlbumList(ObjectSubclass<imp::AlbumList>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup;
}

impl AlbumList {
    pub fn new() -> Self {
        Object::builder().build()
    }
    pub fn pull_albums(&self) {
        let library = self.get_application().library().clone();
        let albums = albums_from_library(&library.borrow());
        let store = self
            .imp()
            .store
            .get()
            .expect("AlbumList store should be initialized.");
        store.remove_all();
        for album in albums {
            store.append(&album);
        }
    }

    fn setup_model(&self) {
        let imp = self.imp();
        let store = gio::ListStore::new::<AlbumData>();
        imp.store
            .set(store.clone())
            .expect("AlbumList store should only be set once.");

        let selection_model = gtk::SingleSelection::new(Some(store));
        let factory = gtk::SignalListItemFactory::new();

        factory.connect_setup(move |_, list_item| {
            let placeholder = Album::new();
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
                let album_data = list_item
                    .item()
                    .and_downcast::<AlbumData>()
                    .expect("Item should be an AlbumData");
                let album_widget = list_item
                    .child()
                    .and_downcast::<Album>()
                    .expect("Child has to be an Album");

                album_widget.set_album_data(&album_data);

                // Async image loading
                album_list.load_album_image(&album_data, &album_widget);
            }
        ));

        imp.grid_view.set_model(Some(&selection_model));
        imp.grid_view.set_factory(Some(&factory));
    }

    fn load_album_image(&self, album_data: &AlbumData, album_widget: &Album) {
        if album_data.image_loading() || album_data.image_loaded() {
            debug!("Image is already loaded");
            return;
        }

        let Some(image_cache) = self.get_application().image_cache() else {
            warn!("Image cache not available");
            return;
        };

        let item_id = album_data.id();
        let jellyfin = self.get_application().jellyfin();
        album_data.set_image_loading(true);
        album_widget.show_loading();

        spawn_tokio(
            async move { image_cache.get_image(&item_id, &jellyfin).await },
            glib::clone!(
                #[weak]
                album_data,
                #[weak]
                album_widget,
                move |result| {
                    album_data.set_image_loading(false);
                    album_data.set_image_loaded(true);
                    match result {
                        Ok(image_data) => {
                            album_widget.set_album_image(&image_data);
                        }
                        Err(err) => {
                            warn!("Failed to load album art for {}: {}", album_data.id(), err);
                            album_widget.show_error();
                        }
                    }
                }
            ),
        );
    }
}

impl Default for AlbumList {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {

    use std::cell::OnceCell;

    use crate::{application::Application, ui::widget_ext::WidgetApplicationExt};
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate, gio,
        glib::{self, object::ObjectExt},
        prelude::WidgetExt,
    };

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/album_list.ui")]
    pub struct AlbumList {
        #[template_child]
        pub grid_view: TemplateChild<gtk::GridView>,
        pub store: OnceCell<gio::ListStore>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AlbumList {
        const NAME: &'static str = "GellyAlbumList";
        type Type = super::AlbumList;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AlbumList {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_model();

            self.obj().connect_map(glib::clone!(
                #[weak(rename_to = album_list)]
                self.obj(),
                move |_| {
                    let app = album_list.get_application();
                    app.connect_closure(
                        "library-refreshed",
                        false,
                        glib::closure_local!(
                            #[weak]
                            album_list,
                            move |_app: Application| {
                                album_list.pull_albums();
                            }
                        ),
                    );
                }
            ));
        }
    }
    impl WidgetImpl for AlbumList {}
    impl BoxImpl for AlbumList {}
}
