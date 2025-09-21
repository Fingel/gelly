use application::Application;
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use log::info;
use ui::window::Window;

mod application;
mod async_utils;
mod cache;
mod config;
mod jellyfin;
mod library_utils;
mod models;
mod ui;

fn main() -> glib::ExitCode {
    env_logger::init();
    gio::resources_register_include!("gelly.gresource").expect("Failed to register resources");
    let app = Application::new();
    app.connect_activate(build_ui);
    info!("Application started");
    app.run()
}

fn build_ui(app: &Application) {
    let window = Window::new(app);
    window.present();
}
