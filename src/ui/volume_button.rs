use glib::Object;
use gtk::{glib, subclass::prelude::*};

glib::wrapper! {
    pub struct VolumeButton(ObjectSubclass<imp::VolumeButton>)
    @extends gtk::Widget, gtk::Box,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl VolumeButton {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn scale(&self) -> &gtk::Scale {
        &self.imp().volume_scale
    }

    pub fn mute_button(&self) -> &gtk::Button {
        &self.imp().mute_button
    }

    pub fn set_icon_name(&self, icon_name: &str) {
        self.imp().menu_button.set_icon_name(icon_name);
    }
}

impl Default for VolumeButton {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use gtk::{CompositeTemplate, TemplateChild, glib, subclass::prelude::*};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/volume_button.ui")]
    pub struct VolumeButton {
        #[template_child]
        pub menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub volume_scale: TemplateChild<gtk::Scale>,
        #[template_child]
        pub mute_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VolumeButton {
        const NAME: &'static str = "GellyVolumeButton";
        type Type = super::VolumeButton;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for VolumeButton {}
    impl WidgetImpl for VolumeButton {}
    impl BoxImpl for VolumeButton {}
}
