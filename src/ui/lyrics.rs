use glib::Object;
use gtk::{gio, glib};

glib::wrapper! {
    pub struct Lyrics(ObjectSubclass<imp::Lyrics>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Lyrics {
    pub fn new() -> Self {
        Object::builder().build()
    }
}

impl Default for Lyrics {
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
    };

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/lyrics.ui")]
    pub struct Lyrics {
        #[template_child]
        pub toolbar_view: TemplateChild<adw::ToolbarView>,
        #[template_child]
        pub lyrics_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Lyrics {
        const NAME: &'static str = "GellyLyrics";
        type Type = super::Lyrics;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Lyrics {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for Lyrics {}
    impl BoxImpl for Lyrics {}
}
