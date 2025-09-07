use adw::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use gtk::{gio, glib};

mod imp;

glib::wrapper! {
    pub struct Application(ObjectSubclass<imp::Application>)
    @extends adw::Application, gio::Application,
    @implements gio::ActionGroup, gio::ActionMap;
}

impl Application {
    pub fn new() -> Self {
        Object::builder()
            .property("application-id", "io.m51.Gelly")
            .build()
    }

    pub fn set_auth_token(&self, token: Option<String>) {
        self.imp().auth_token.replace(token);
    }

    pub fn auth_token(&self) -> Option<String> {
        self.imp().auth_token.borrow().clone()
    }
}

impl Default for Application {
    fn default() -> Self {
        Self::new()
    }
}
