use adw::subclass::prelude::*;
use glib::subclass::InitializingObject;
use gtk::{Button, CompositeTemplate, glib};

#[derive(CompositeTemplate, Default)]
#[template(resource = "/io/m51/Gelly/ui/window.ui")]
pub struct Window {
    #[template_child]
    pub button: TemplateChild<Button>,
}

#[glib::object_subclass]
impl ObjectSubclass for Window {
    const NAME: &'static str = "GellyApplicationWindow";
    type Type = super::Window;
    type ParentType = adw::ApplicationWindow;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for Window {
    fn constructed(&self) {
        self.parent_constructed();
    }
}

impl WidgetImpl for Window {}

impl WindowImpl for Window {}

impl AdwApplicationWindowImpl for Window {}

impl ApplicationWindowImpl for Window {}
