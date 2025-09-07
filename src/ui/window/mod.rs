use crate::application::Application;
use adw::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use gtk::{
    gio,
    glib::{self, object::CastNone},
    prelude::*,
};
use log::info;

mod imp;

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
    @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
            @implements gio::ActionMap, gio::ActionGroup;

}

impl Window {
    pub fn new(app: &Application) -> Self {
        let window: Self = Object::builder().property("application", app).build();
        let servers: Vec<String> = vec![];
        if servers.is_empty() {
            window.show_server_setup();
        } else {
            window.show_main_page();
        }
        window
    }

    pub fn show_server_setup(&self) {
        let imp = self.imp();
        imp.setup_navigation.replace(&[imp.setup_servers.get()]);
    }

    pub fn show_main_page(&self) {
        let imp = self.imp();
        imp.setup_navigation.replace(&[imp.main_window.get()]);
    }

    pub fn handle_connection_attempt(&self, host: &str, username: &str, password: &str) {
        if let Some(app) = self.application().and_downcast::<Application>() {
            dbg!("before", app.auth_token());
            app.set_auth_token(Some(username.to_string()));
            dbg!("after", app.auth_token());
            info!("User authenticated successfully");
        }
        dbg!(host, username, password);
    }
}
