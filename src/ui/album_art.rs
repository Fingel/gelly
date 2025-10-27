use crate::{
    async_utils::spawn_tokio,
    ui::{image_utils::bytes_to_texture, widget_ext::WidgetApplicationExt},
};
use glib::Object;
use gtk::{gio, glib, prelude::WidgetExt, subclass::prelude::*};
use log::warn;

glib::wrapper! {
    pub struct AlbumArt(ObjectSubclass<imp::AlbumArt>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
impl AlbumArt {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn set_item_id(&self, item_id: &str, fallback_id: Option<&str>) {
        if let Some(id) = fallback_id {
            self.imp().fallback_image.replace(Some(id.to_string()));
        }
        let current_item_id = self.imp().item_id.borrow().clone();
        if current_item_id != item_id {
            self.imp().is_loaded.set(false);
            self.imp().item_id.replace(item_id.to_string());
            if self.get_gtk_window().is_some() {
                self.load_image();
            }
        }
    }

    pub fn set_image(&self, image_data: &[u8]) {
        let image_data_copy = image_data.to_vec();
        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to=album_art)]
            self,
            async move {
                match bytes_to_texture(&image_data_copy).await {
                    Ok(texture) => {
                        album_art.imp().album_image.set_paintable(Some(&texture));
                    }
                    Err(err) => {
                        warn!("Failed to load album image: {}", err);
                    }
                }
            }
        ));
    }

    pub fn set_loading(&self, loading: bool) {
        self.imp().spinner.set_visible(loading);
        self.imp().is_loading.set(loading);
        if loading {
            self.imp().spinner.start();
        } else {
            self.imp().spinner.stop();
        }
    }

    pub fn show_error(&self) {
        self.set_loading(false);
        self.imp().error_icon.set_visible(true);
    }

    pub fn load_image(&self) {
        let item_id = if self.imp().item_id.borrow().is_empty() {
            return;
        } else {
            self.imp().item_id.borrow().clone()
        };

        let fallback_id = self.imp().fallback_image.borrow().clone();

        if self.imp().is_loading.get() || self.imp().is_loaded.get() {
            return;
        }

        let Some(image_cache) = self.get_application().image_cache() else {
            warn!("Could not get image cache");
            return;
        };

        let jellyfin = self.get_application().jellyfin();

        self.set_loading(true);
        spawn_tokio(
            async move {
                image_cache
                    .get_images(&item_id, fallback_id.as_deref(), &jellyfin)
                    .await
            },
            glib::clone!(
                #[weak(rename_to = album_art)]
                self,
                move |result| {
                    album_art.set_loading(false);
                    match result {
                        Ok(image_data) => {
                            album_art.imp().is_loaded.set(true);
                            album_art.set_image(&image_data);
                        }
                        Err(err) => {
                            warn!("Failed to load image {}", err);
                            album_art.show_error();
                        }
                    }
                }
            ),
        );
    }
}

impl Default for AlbumArt {
    fn default() -> Self {
        Self::new()
    }
}
mod imp {
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib::{self, Properties},
        prelude::*,
    };
    use std::cell::{Cell, RefCell};

    #[derive(Properties, CompositeTemplate, Default)]
    #[properties(wrapper_type = super::AlbumArt)]
    #[template(resource = "/io/m51/Gelly/ui/album_art.ui")]
    pub struct AlbumArt {
        #[template_child]
        pub album_image: TemplateChild<gtk::Image>,
        #[template_child]
        pub spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub error_icon: TemplateChild<gtk::Image>,
        #[property(get, set, default = 200_u32)]
        pub size: Cell<u32>,

        pub item_id: RefCell<String>,
        pub is_loading: Cell<bool>,
        pub is_loaded: Cell<bool>,
        pub fallback_image: RefCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AlbumArt {
        const NAME: &'static str = "GellyAlbumArt";
        type Type = super::AlbumArt;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }
    impl BoxImpl for AlbumArt {}
    impl WidgetImpl for AlbumArt {}

    #[glib::derived_properties]
    impl ObjectImpl for AlbumArt {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().connect_realize(|widget| {
                widget.load_image();
            });

            // Bind the width and height properties to the picture widget
            self.obj()
                .bind_property("size", &self.album_image.get(), "pixel-size")
                .sync_create()
                .build();
        }
    }
}
