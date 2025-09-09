use glib::Object;
use gtk::{gio, glib};

mod imp;

glib::wrapper! {
    pub struct Application(ObjectSubclass<imp::Application>)
    @extends gio::Application, gtk::Application, adw::Application,
    @implements gio::ActionGroup, gio::ActionMap;
}

impl Application {
    pub fn new() -> Self {
        Object::builder()
            .property("application-id", "io.m51.Gelly")
            .build()
    }
}

impl Default for Application {
    fn default() -> Self {
        Self::new()
    }
}
