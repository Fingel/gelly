use adw::subclass::prelude::*;
use glib::subclass::InitializingObject;
use gtk::{CompositeTemplate, glib};

#[derive(CompositeTemplate, Default)]
#[template(resource = "/io/m51/Gelly/ui/server_form.ui")]
pub struct ServerForm {
    #[template_child]
    pub host_entry: TemplateChild<adw::EntryRow>,
    #[template_child]
    pub port_entry: TemplateChild<adw::EntryRow>,
    #[template_child]
    pub username_entry: TemplateChild<adw::EntryRow>,
    #[template_child]
    pub password_entry: TemplateChild<adw::PasswordEntryRow>,
}

#[glib::object_subclass]
impl ObjectSubclass for ServerForm {
    const NAME: &'static str = "GellyServerForm";
    type Type = super::ServerForm;
    type ParentType = gtk::Box;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for ServerForm {
    fn constructed(&self) {
        self.parent_constructed();
    }
}

impl WidgetImpl for ServerForm {}

impl BoxImpl for ServerForm {}
