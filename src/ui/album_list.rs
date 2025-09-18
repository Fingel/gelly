use crate::{library_utils::albums_from_library, ui::widget_ext::WidgetApplicationExt};
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
    pub fn pull_albums(&self) {
        let library = self.get_application().library().clone();
        let albums = albums_from_library(&library.borrow());
        dbg!(albums);
    }
}

impl Default for AlbumList {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {

    use crate::{application::Application, ui::widget_ext::WidgetApplicationExt};
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib::{self, object::ObjectExt},
        prelude::WidgetExt,
    };

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

    impl ObjectImpl for AlbumList {
        fn constructed(&self) {
            self.parent_constructed();

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
