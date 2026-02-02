//! Custom implementation of a scrollable window with automatic scrolling during D&D.
use adw::{prelude::*, subclass::prelude::*};
use gtk::glib;

glib::wrapper! {
    pub struct AutoScrollWindow(ObjectSubclass<imp::AutoScrollWindow>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl AutoScrollWindow {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn scrolled_window(&self) -> &gtk::ScrolledWindow {
        self.imp()
            .scrolled_window
            .get()
            .expect("scrolled_window should be set")
    }

    pub fn hadjustment(&self) -> gtk::Adjustment {
        self.scrolled_window().hadjustment()
    }

    pub fn vadjustment(&self) -> gtk::Adjustment {
        self.scrolled_window().vadjustment()
    }
}

impl Default for AutoScrollWindow {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use super::*;
    use glib::Properties;
    use std::cell::{OnceCell, RefCell};

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::AutoScrollWindow)]
    pub struct AutoScrollWindow {
        pub scrolled_window: OnceCell<gtk::ScrolledWindow>,

        #[property(get, set = Self::set_content)]
        content: RefCell<Option<gtk::Widget>>,
    }

    impl AutoScrollWindow {
        fn set_content(&self, content: Option<gtk::Widget>) {
            if let Some(scrolled_window) = self.scrolled_window.get() {
                scrolled_window.set_child(content.as_ref());
            }
            self.content.replace(content);
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AutoScrollWindow {
        const NAME: &'static str = "GellyAutoScrollWindow";
        type Type = super::AutoScrollWindow;
        type ParentType = adw::Bin;
    }

    #[glib::derived_properties]
    impl ObjectImpl for AutoScrollWindow {
        fn constructed(&self) {
            self.parent_constructed();

            // Create a GtkScrolledWindow with standard configuration
            let scrolled_window = gtk::ScrolledWindow::builder()
                .hexpand(true)
                .vexpand(true)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .vscrollbar_policy(gtk::PolicyType::Automatic)
                .propagate_natural_height(true)
                .build();

            // Set the scrolled window as the Bin's child
            self.obj()
                .upcast_ref::<adw::Bin>()
                .set_child(Some(&scrolled_window));

            self.scrolled_window
                .set(scrolled_window)
                .expect("scrolled_window should only be set once");
        }
    }

    impl WidgetImpl for AutoScrollWindow {}
    impl BinImpl for AutoScrollWindow {}
}
