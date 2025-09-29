use crate::config::settings;
use crate::models::AlbumModel;
use crate::{application::Application, ui::widget_ext::WidgetApplicationExt};
use adw::{prelude::*, subclass::prelude::ObjectSubclassIsExt};
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
        imp.main_navigation.replace(&[imp.main_window.get()]);
        self.get_application().refresh_library();
    }

    pub fn show_album_detail(&self, album_model: &AlbumModel) {
        let imp = self.imp();
        imp.album_detail_page.set_title(&album_model.name());
        imp.main_navigation.push(&imp.album_detail_page.get());
        imp.album_detail.set_album_model(album_model);
    }

    pub fn logout(&self) {
        self.get_application().logout();
        self.show_server_setup();
        self.toast("Logged out", None);
    }

    pub fn save_window_size(&self) -> Result<(), glib::BoolError> {
        let size = self.default_size();
        let settings = settings();
        settings.set_int("window-width", size.0)?;
        settings.set_int("window-height", size.1)?;
        settings.set_boolean("window-maximized", self.is_maximized())?;

        Ok(())
    }

    fn load_window_size(&self) {
        let settings = settings();
        let width = settings.int("window-width");
        let height = settings.int("window-height");
        let maximized = settings.boolean("window-maximized");

        self.set_default_size(width, height);
        if maximized {
            self.maximize();
        }
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

    use crate::ui::album_list::AlbumList;
    use crate::ui::setup::Setup;
    use crate::ui::widget_ext::WidgetApplicationExt;
    use crate::{application::Application, ui::album_detail::AlbumDetail};

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
        pub main_window: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub album_list: TemplateChild<AlbumList>,
        #[template_child]
        pub album_detail_page: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub album_detail: TemplateChild<AlbumDetail>,
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
            self.obj().load_window_size();

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

    impl WindowImpl for Window {
        fn close_request(&self) -> glib::Propagation {
            self.obj()
                .save_window_size()
                .expect("Could not save window size");
            glib::Propagation::Proceed
        }
    }

    impl AdwApplicationWindowImpl for Window {}

    impl ApplicationWindowImpl for Window {}
}
