// In src/ui/widget_ext.rs
use gtk::glib;
use gtk::prelude::*;

pub trait WidgetApplicationExt {
    fn get_application<T>(&self) -> Option<T>
    where
        T: glib::object::IsA<gtk::Application>;

    fn get_root_window(&self) -> Option<crate::ui::window::Window>;
    fn get_gtk_window(&self) -> Option<gtk::Window>;
    fn toast(&self, message: &str, timeout: Option<u32>);
}

impl<W> WidgetApplicationExt for W
where
    W: glib::object::IsA<gtk::Widget>,
{
    fn get_application<T>(&self) -> Option<T>
    where
        T: glib::object::IsA<gtk::Application>,
    {
        self.get_gtk_window()?.application()?.downcast::<T>().ok()
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
