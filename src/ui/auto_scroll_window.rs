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

    const MARGIN: f64 = 64.0;

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

            let scrolled_window = gtk::ScrolledWindow::builder()
                .hexpand(true)
                .vexpand(true)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .vscrollbar_policy(gtk::PolicyType::Automatic)
                .propagate_natural_height(true)
                .propagate_natural_width(true)
                .build();

            let drop_motion_ctrl = gtk::DropControllerMotion::new();
            drop_motion_ctrl.connect_motion(glib::clone!(
                #[weak(rename_to = auto_scroll)]
                self.obj(),
                move |_ctrl, _x, y| {
                    let imp = auto_scroll.imp();
                    let Some(scrolled_window) = imp.scrolled_window.get() else {
                        return;
                    };

                    let height = scrolled_window.height() as f64;

                    let mut should_scroll = false;
                    let mut scroll_up = false;
                    let mut distance_from_edge = 0.0;

                    if y < MARGIN {
                        distance_from_edge = MARGIN - y;
                        should_scroll = true;
                        scroll_up = true;
                    } else if y > (height - MARGIN) {
                        distance_from_edge = y - (height - MARGIN);
                        should_scroll = true;
                        scroll_up = false;
                    }

                    if should_scroll {
                        let scroll_speed = distance_from_edge / MARGIN;
                        println!(
                            "Scroll {} distance: {:.1}, speed: {:.2}",
                            if scroll_up { "up" } else { "down" },
                            distance_from_edge,
                            scroll_speed
                        );
                    }
                }
            ));

            drop_motion_ctrl.connect_leave(|_ctrl| {
                println!("Drag left scrolled window");
            });

            scrolled_window.add_controller(drop_motion_ctrl);

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
