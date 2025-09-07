use adw::prelude::*;
use adw::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use gtk::{gio, glib};

mod imp;

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
}

impl Default for ServerForm {
    fn default() -> Self {
        Self::new()
    }
}
