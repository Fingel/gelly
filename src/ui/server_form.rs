use adw::prelude::*;
use adw::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use gtk::{gio, glib};

glib::wrapper! {
    pub struct ServerForm(ObjectSubclass<imp::ServerForm>)
    @extends gtk::Widget, gtk::Box,
                @implements gio::ActionMap, gio::ActionGroup;
}

#[derive(Debug)]
pub struct ServerFormValues {
    pub host: String,
    pub username: String,
    pub password: String,
}

impl ServerForm {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn is_complete(&self) -> bool {
        !self.imp().host_entry.text().is_empty()
    }

    pub fn get_value(&self) -> ServerFormValues {
        let imp = self.imp();
        ServerFormValues {
            host: imp.host_entry.text().to_string(),
            username: imp.username_entry.text().to_string(),
            password: imp.password_entry.text().to_string(),
        }
    }

    pub fn host_error(&self) {
        self.imp().host_entry.add_css_class("error");
    }

    pub fn authentication_error(&self) {
        self.imp().username_entry.add_css_class("error");
        self.imp().password_entry.add_css_class("error");
    }

    pub fn clear_errors(&self) {
        self.imp().host_entry.remove_css_class("error");
        self.imp().username_entry.remove_css_class("error");
        self.imp().password_entry.remove_css_class("error");
    }
}

impl Default for ServerForm {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{CompositeTemplate, glib};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/server_form.ui")]
    pub struct ServerForm {
        #[template_child]
        pub host_entry: TemplateChild<adw::EntryRow>,
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
}
