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
mod reporting;
mod ui;

fn main() -> glib::ExitCode {
    env_logger::init();
    gio::resources_register_include!("gelly.gresource").expect("Failed to register resources");
    let app = Application::new();
    app.connect_startup(|_| load_css());
    app.connect_activate(build_ui);
    app.set_accels_for_action("win.refresh-library", &["<Ctrl>r"]);
    app.set_accels_for_action("win.request-library-rescan", &["<Ctrl><Shift>r"]);
    app.set_accels_for_action("win.search", &["<Ctrl>f"]);
    app.set_accels_for_action("win.play-selected", &["<Ctrl>p"]);
    app.set_accels_for_action("win.shortcuts", &["<Ctrl>question"]);
    app.set_accels_for_action("win.preferences", &["<Ctrl>comma"]);
    app.set_accels_for_action("win.show-album-list", &["<Ctrl>1"]);
    app.set_accels_for_action("win.show-artist-list", &["<Ctrl>2"]);
    app.set_accels_for_action("win.show-playlist-list", &["<Ctrl>3"]);
    app.set_accels_for_action("win.show-song-list", &["<Ctrl>4"]);
    app.set_accels_for_action("window.close", &["<Ctrl>q"]);
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
