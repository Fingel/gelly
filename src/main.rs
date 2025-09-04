mod ui;

use adw::Application;
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use ui::window::Window;

fn main() -> glib::ExitCode {
    gio::resources_register_include!("gelly.gresource").expect("Failed to register resources");
    let app = Application::builder()
        .application_id("io.m51.Gelly")
        .build();
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &Application) {
    let window = Window::new(app);
    window.present();
}
