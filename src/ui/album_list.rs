use glib::Object;
use gtk::{gio, glib};

glib::wrapper! {
    pub struct AlbumList(ObjectSubclass<imp::AlbumList>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup;
}

impl AlbumList {
    pub fn new() -> Self {
        Object::builder().build()
    }
}

impl Default for AlbumList {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{CompositeTemplate, glib};
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/album_list.ui")]
    pub struct AlbumList {}

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

    impl ObjectImpl for AlbumList {}
    impl WidgetImpl for AlbumList {}
    impl BoxImpl for AlbumList {}
}
