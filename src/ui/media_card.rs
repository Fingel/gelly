use glib::Object;
use gtk::{self, gio, glib, prelude::*, subclass::prelude::*};

glib::wrapper! {
    pub struct MediaCard(ObjectSubclass<imp::MediaCard>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl MediaCard {
    pub fn new(has_play_button: bool, has_secondary_label: bool) -> Self {
        Object::builder()
            .property("has-play-button", has_play_button)
            .property("has-secondary-label", has_secondary_label)
            .build()
    }

    pub fn set_primary_text(&self, text: &str) {
        self.imp().primary_label.set_text(text);
    }

    pub fn set_secondary_text(&self, text: &str) {
        self.imp().secondary_label.set_text(text);
    }

    pub fn set_image_id(&self, id: &str) {
        self.imp().image.set_item_id(id, None);
    }

    pub fn set_static_icon(&self, icon_name: &str) {
        self.imp().static_icon.set_icon_name(Some(icon_name));
    }

    pub fn display_icon(&self) {
        self.imp().static_icon.set_visible(true);
        self.imp().image.set_visible(false);
    }

    pub fn connect_play_clicked<F>(&self, f: F)
    where
        F: Fn() + 'static,
    {
        self.imp().overlay_play.connect_clicked(move |_| f());
    }
}

impl Default for MediaCard {
    fn default() -> Self {
        Self::new(false, false)
    }
}

mod imp {
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib::{self, Properties},
        prelude::*,
    };
    use std::cell::Cell;

    use crate::ui::album_art::AlbumArt;

    #[derive(CompositeTemplate, Default, Properties)]
    #[template(resource = "/io/m51/Gelly/ui/media_card.ui")]
    #[properties(wrapper_type = super::MediaCard)]
    pub struct MediaCard {
        #[template_child]
        pub primary_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub image: TemplateChild<AlbumArt>,

        //potentially hidden
        #[template_child]
        pub secondary_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub play_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub motion_controller: TemplateChild<gtk::EventControllerMotion>,
        #[template_child]
        pub overlay_play: TemplateChild<gtk::Button>,
        #[template_child]
        pub static_icon: TemplateChild<gtk::Image>,

        #[property(get, set)]
        has_play_button: Cell<bool>,
        #[property(get, set)]
        has_secondary_label: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MediaCard {
        const NAME: &'static str = "GellyMediaCard";
        type Type = super::MediaCard;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for MediaCard {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().connect_map(|widget| {
                let imp = widget.imp();

                imp.secondary_label
                    .set_visible(widget.has_secondary_label());
                imp.play_revealer.set_visible(widget.has_play_button());

                if widget.has_play_button() {
                    imp.setup_play_signals();
                }
            });
        }
    }

    impl MediaCard {
        fn setup_play_signals(&self) {
            self.motion_controller.connect_enter(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |_, _, _| {
                    imp.play_revealer.set_reveal_child(true);
                }
            ));

            self.motion_controller.connect_leave(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |_| {
                    imp.play_revealer.set_reveal_child(false);
                }
            ));
        }
    }

    impl WidgetImpl for MediaCard {}
    impl BoxImpl for MediaCard {}
}
