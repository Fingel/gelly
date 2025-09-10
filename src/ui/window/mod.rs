use crate::application::Application;
use crate::async_utils::spawn_tokio;
use crate::jellyfin::{Jellyfin, JellyfinError};
use adw::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use gtk::{
    gio,
    glib::{self, object::CastNone},
    prelude::*,
};
use log::debug;

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
            let host = host.to_string();
            let username = username.to_string();
            let password = password.to_string();
            spawn_tokio(
                async move { Jellyfin::new_authenticate(&host, &username, &password).await },
                glib::clone!(
                    #[weak(rename_to=window)]
                    self,
                    move |result| {
                        match result {
                            Ok(jellyfin) => {
                                app.imp().jellyfin.replace(Some(jellyfin));
                            }
                            Err(err) => window.handle_connection_error(err),
                        }
                    }
                ),
            );
        }
        dbg!(host, username, password);
    }

    fn handle_connection_error(&self, error: JellyfinError) {
        match error {
            JellyfinError::Transport(err) => {
                debug!("Transport error: {}", err);
            }
            JellyfinError::Http { status, message } => {
                debug!("HTTP {} error: {}", status, message);
            }
            JellyfinError::AuthenticationFailed { message } => {
                debug!("Authentication failed: {}", message);
            }
            _ => {
                dbg!(error);
            }
        }
    }
}
