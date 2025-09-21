use crate::{application::Application, ui::widget_ext::WidgetApplicationExt};
use adw::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use gtk::{
    gio,
    glib::{self},
};

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
    @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
            @implements gio::ActionMap, gio::ActionGroup;

}

impl Window {
    pub fn new(app: &Application) -> Self {
        let window: Self = Object::builder().property("application", app).build();
        if !app.setup_complete() {
            window.show_server_setup();
        } else {
            window.show_main_page();
        }
        window
    }

    pub fn toast(&self, title: &str, timeout: Option<u32>) {
        let mut toast = adw::Toast::builder().title(title);
        if let Some(timeout) = timeout {
            toast = toast.timeout(timeout);
        }
        self.imp().toaster.add_toast(toast.build());
    }

    pub fn show_server_setup(&self) {
        let imp = self.imp();
        imp.setup_stack.set_visible_child(&imp.setup.get());
    }

    pub fn show_main_page(&self) {
        let imp = self.imp();
        imp.setup_stack
            .set_visible_child(&imp.main_navigation.get());
        self.get_application().refresh_library();
    }

    pub fn logout(&self) {
        self.get_application().logout();
        self.show_server_setup();
        self.toast("Logged out", None);
    }
}

mod imp {
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        gio::{ActionEntry, prelude::ActionMapExtManual},
        glib,
        prelude::*,
    };
    use log::{debug, warn};

    use crate::application::Application;
    use crate::ui::album_list::AlbumList;
    use crate::ui::setup::Setup;
    use crate::ui::widget_ext::WidgetApplicationExt;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/window.ui")]
    pub struct Window {
        #[template_child]
        pub toaster: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub setup_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub setup: TemplateChild<Setup>,
        #[template_child]
        pub main_navigation: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub album_list: TemplateChild<AlbumList>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "GellyApplicationWindow";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Window {
        fn constructed(&self) {
            self.parent_constructed();

            let action_logout = ActionEntry::builder("logout")
                .activate(glib::clone!(
                    #[weak(rename_to=window)]
                    self,
                    move |_, _, _| {
                        window.obj().logout();
                    }
                ))
                .build();

            let action_clear_cache = ActionEntry::builder("clear-cache")
                .activate(glib::clone!(
                    #[weak(rename_to=window)]
                    self,
                    move |_, _, _| {
                        if let Some(image_cache) = window.obj().get_application().image_cache() {
                            image_cache.clear_cache();
                            debug!("Image cache cleared");
                        } else {
                            warn!("No image cache found");
                        }
                    }
                ))
                .build();

            self.obj()
                .add_action_entries([action_logout, action_clear_cache]);

            self.obj().connect_map(glib::clone!(
                #[weak(rename_to = window)]
                self.obj(),
                move |_| {
                    let app = window.get_application();
                    app.connect_closure(
                        "global-error",
                        false,
                        glib::closure_local!(
                            #[weak]
                            window,
                            move |_app: Application, title: &str| {
                                window.toast(title, None);
                            }
                        ),
                    );
                }
            ));
        }
    }

    impl WidgetImpl for Window {}

    impl WindowImpl for Window {}

    impl AdwApplicationWindowImpl for Window {}

    impl ApplicationWindowImpl for Window {}
}
