//! Custom implementation of a scrollable window with automatic scrolling during D&D.
use adw::{prelude::*, subclass::prelude::*};
use gtk::glib;

const MARGIN: f64 = 64.0;
const SPEED: f64 = 25.0;

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
    use glib::{Properties, source::SourceId};
    use std::cell::{Cell, OnceCell, RefCell};
    use std::time::Duration;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::AutoScrollWindow)]
    pub struct AutoScrollWindow {
        pub scrolled_window: OnceCell<gtk::ScrolledWindow>,

        #[property(get, set = Self::set_content)]
        content: RefCell<Option<gtk::Widget>>,

        scroll_timeout_id: RefCell<Option<SourceId>>,
        scroll_speed: Cell<f64>,
    }

    impl AutoScrollWindow {
        fn set_content(&self, content: Option<gtk::Widget>) {
            if let Some(scrolled_window) = self.scrolled_window.get() {
                scrolled_window.set_child(content.as_ref());
            }
            self.content.replace(content);
        }

        fn start_auto_scroll(&self, drop_motion_ctrl: &gtk::DropControllerMotion) {
            if self.scroll_timeout_id.borrow().is_some() {
                return;
            }

            let timeout_id = glib::timeout_add_local(
                Duration::from_millis(16), // ~60 FPS
                glib::clone!(
                    #[weak(rename_to = auto_scroll)]
                    self,
                    #[weak]
                    drop_motion_ctrl,
                    #[upgrade_or]
                    glib::ControlFlow::Break,
                    move || {
                        // Stop if the pointer is no longer over the widget
                        if !drop_motion_ctrl.contains_pointer() {
                            auto_scroll.stop_auto_scroll();
                            return glib::ControlFlow::Break;
                        }

                        let Some(scrolled_window) = auto_scroll.scrolled_window.get() else {
                            return glib::ControlFlow::Break;
                        };

                        let scroll_speed = auto_scroll.scroll_speed.get();
                        let adj = scrolled_window.vadjustment();
                        let new_value = adj.value() + (scroll_speed * SPEED);

                        // Clamp to valid range
                        let lower = adj.lower();
                        let upper = adj.upper();
                        let page_size = adj.page_size();
                        let clamped_value = new_value.max(lower).min(upper - page_size);

                        adj.set_value(clamped_value);

                        glib::ControlFlow::Continue
                    }
                ),
            );

            self.scroll_timeout_id.replace(Some(timeout_id));
        }

        fn stop_auto_scroll(&self) {
            if let Some(timeout_id) = self.scroll_timeout_id.take() {
                timeout_id.remove();
            }
            self.scroll_speed.set(0.0);
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
                .build();

            let drop_motion_ctrl = gtk::DropControllerMotion::new();
            drop_motion_ctrl.connect_motion(glib::clone!(
                #[weak(rename_to = auto_scroll)]
                self,
                #[weak]
                drop_motion_ctrl,
                move |_ctrl, _x, y| {
                    let Some(scrolled_window) = auto_scroll.scrolled_window.get() else {
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
                        let scroll_speed =
                            (distance_from_edge / MARGIN) * if scroll_up { -1.0 } else { 1.0 };
                        auto_scroll.scroll_speed.set(scroll_speed);
                        auto_scroll.start_auto_scroll(&drop_motion_ctrl);
                    } else {
                        auto_scroll.stop_auto_scroll();
                    }
                }
            ));

            drop_motion_ctrl.connect_leave(glib::clone!(
                #[weak (rename_to = auto_scroll)]
                self,
                move |_ctrl| {
                    auto_scroll.stop_auto_scroll();
                }
            ));

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
