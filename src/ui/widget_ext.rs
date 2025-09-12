use gtk::glib;
use gtk::prelude::*;

use crate::application::Application;

pub trait WidgetApplicationExt {
    fn get_application(&self) -> Application;
    fn get_root_window(&self) -> Option<crate::ui::window::Window>;
    fn get_gtk_window(&self) -> Option<gtk::Window>;
    fn toast(&self, message: &str, timeout: Option<u32>);
}

impl<W> WidgetApplicationExt for W
where
    W: glib::object::IsA<gtk::Widget>,
{
    fn get_application(&self) -> Application {
        let window = self.get_gtk_window().expect(
            "Widget not attached to window - ensure widget is properly added to UI hierarchy",
        );
        let app = window
            .application()
            .expect("Window missing application - this indicates an architectural problem");
        app.downcast::<Application>()
            .expect("Application type mismatch - ensure consistent Application type usage")
    }

    fn get_root_window(&self) -> Option<crate::ui::window::Window> {
        self.root()?
            .dynamic_cast::<crate::ui::window::Window>()
            .ok()
    }

    fn get_gtk_window(&self) -> Option<gtk::Window> {
        self.root()?.dynamic_cast::<gtk::Window>().ok()
    }

    fn toast(&self, message: &str, timeout: Option<u32>) {
        if let Some(window) = self.get_root_window() {
            window.toast(message, timeout);
        }
    }
}
