use glib::Object;
use gtk::{glib, subclass::prelude::*};

glib::wrapper! {
    pub struct VolumeButton(ObjectSubclass<imp::VolumeButton>)
    @extends gtk::Widget, gtk::Box,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl VolumeButton {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn scale(&self) -> &gtk::Scale {
        &self.imp().volume_scale
    }

    pub fn mute_button(&self) -> &gtk::Button {
        &self.imp().mute_button
    }

    pub fn set_icon_name(&self, icon_name: &str) {
        self.imp().menu_button.set_icon_name(icon_name);
    }
}

impl Default for VolumeButton {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use gtk::{
        CompositeTemplate, EventControllerScrollFlags, TemplateChild, glib,
        prelude::{AdjustmentExt, RangeExt, WidgetExt},
        subclass::prelude::*,
    };

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/volume_button.ui")]
    pub struct VolumeButton {
        #[template_child]
        pub menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub volume_scale: TemplateChild<gtk::Scale>,
        #[template_child]
        pub mute_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VolumeButton {
        const NAME: &'static str = "GellyVolumeButton";
        type Type = super::VolumeButton;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for VolumeButton {
        fn constructed(&self) {
            // update volume level in tooltip, starts from 100% in template
            let adjustment = self.obj().scale().adjustment();
            adjustment.connect_value_changed(glib::clone!(
                #[weak(rename_to = menu_button)]
                self.menu_button,
                move |adj| {
                    menu_button.set_tooltip_text(Some(&format!(
                        "Volume ({}%)",
                        (adj.value() * 100.0).round()
                    )));
                }
            ));

            let scroll_event_controller = gtk::EventControllerScroll::new(
                EventControllerScrollFlags::VERTICAL | EventControllerScrollFlags::DISCRETE,
            );
            scroll_event_controller.connect_scroll(move |_, _, delta_y| {
                // -1 is scroll up, 1 is scroll down
                adjustment
                    .set_value((adjustment.value() + (-(delta_y.trunc()) * 0.05)).clamp(0.0, 1.0));
                glib::Propagation::Stop
            });
            self.menu_button.add_controller(scroll_event_controller);
        }
    }
    impl WidgetImpl for VolumeButton {}
    impl BoxImpl for VolumeButton {}
}
