use application::Application;
use gtk::CssProvider;
use gtk::gdk::Display;
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use log::info;
use ui::window::Window;

mod application;
mod async_utils;
mod audio;
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
    app.connect_startup(|_| load_css());
    app.connect_activate(build_ui);
    app.set_accels_for_action("win.refresh-library", &["<Ctrl>r"]);
    app.set_accels_for_action("win.search", &["<Ctrl>f"]);
    info!("Application started");
    app.run()
}

fn build_ui(app: &Application) {
    if let Some(window) = app.active_window() {
        window.present();
        return;
    }
    let window = Window::new(app);
    window.present();
}

fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_resource("/io/m51/Gelly/style.css");
    gtk::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
