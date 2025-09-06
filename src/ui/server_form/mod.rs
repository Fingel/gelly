use glib::Object;
use gtk::{gio, glib};

mod imp;

glib::wrapper! {
    pub struct ServerForm(ObjectSubclass<imp::ServerForm>)
    @extends gtk::Widget, gtk::Box,
                @implements gio::ActionMap, gio::ActionGroup;
}

impl ServerForm {
    pub fn new() -> Self {
        Object::builder().build()
    }
}

impl Default for ServerForm {
    fn default() -> Self {
        Self::new()
    }
}
