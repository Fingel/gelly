use crate::application::Application;
use crate::async_utils::spawn_tokio;
use crate::config::{settings, store_jellyfin_api_token};
use crate::jellyfin::{Jellyfin, JellyfinError};
use adw::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use gtk::{
    gio,
    glib::{self, object::CastNone},
    prelude::*,
};
use log::debug;

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
    @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
            @implements gio::ActionMap, gio::ActionGroup;

}

impl Window {
    pub fn new(app: &Application) -> Self {
        let window: Self = Object::builder().property("application", app).build();
        let servers: Vec<String> = vec![];
        if servers.is_empty() {
            window.show_server_setup();
        } else {
            window.show_main_page();
        }
        window
    }

    pub fn show_server_setup(&self) {
        let imp = self.imp();
        imp.setup_navigation.replace(&[imp.setup_servers.get()]);
    }

    pub fn show_main_page(&self) {
        let imp = self.imp();
        imp.setup_navigation.replace(&[imp.main_window.get()]);
    }

    pub fn handle_connection_attempt(&self, host: &str, username: &str, password: &str) {
        if let Some(app) = self.application().and_downcast::<Application>() {
            let host = host.to_string();
            let username = username.to_string();
            let password = password.to_string();
            spawn_tokio(
                async move { Jellyfin::new_authenticate(&host, &username, &password).await },
                glib::clone!(
                    #[weak(rename_to=window)]
                    self,
                    move |result| {
                        match result {
                            Ok(jellyfin) => {
                                let user_id = jellyfin.user_id.clone();
                                let token = jellyfin.token.clone();
                                let host = jellyfin.host.clone();
                                app.imp().jellyfin.replace(Some(jellyfin));
                                window.save_server_settings(&host, &user_id, &token);
                            }
                            Err(err) => window.handle_connection_error(err),
                        }
                    }
                ),
            );
        }
        dbg!(host, username, password);
    }

    fn save_server_settings(&self, host: &str, user_id: &str, token: &str) {
        settings()
            .set_string("hostname", host)
            .expect("Failed to save hostname");
        settings()
            .set_string("user-id", user_id)
            .expect("Failed to save user-id");
        store_jellyfin_api_token(host, user_id, token);
    }

    fn handle_connection_error(&self, error: JellyfinError) {
        match error {
            JellyfinError::Transport(err) => {
                debug!("Transport error: {}", err);
            }
            JellyfinError::Http { status, message } => {
                debug!("HTTP {} error: {}", status, message);
            }
            JellyfinError::AuthenticationFailed { message } => {
                debug!("Authentication failed: {}", message);
            }
            _ => {
                dbg!(error);
            }
        }
    }
}

mod imp {
    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        gio::{ActionEntry, prelude::ActionMapExtManual},
        glib,
        prelude::WidgetExt,
    };
    use log::warn;

    use crate::ui::server_form::ServerForm;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/window.ui")]
    pub struct Window {
        #[template_child]
        pub setup_navigation: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub setup_servers: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub main_window: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub server_form: TemplateChild<ServerForm>,
        #[template_child]
        pub connect_button: TemplateChild<adw::ButtonRow>,
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

            let action_servers = ActionEntry::builder("servers")
                .activate(|_, _, _| {
                    println!("Server list action");
                })
                .build();
            self.obj().add_action_entries([action_servers]);
            self.setup_signals();
        }
    }

    impl WidgetImpl for Window {}

    impl WindowImpl for Window {}

    impl AdwApplicationWindowImpl for Window {}

    impl ApplicationWindowImpl for Window {}

    impl Window {
        fn setup_signals(&self) {
            // Setup Connect Button
            self.connect_button.connect_activated(glib::clone!(
                #[weak(rename_to=window)]
                self,
                move |_| {
                    if window.server_form.is_complete() {
                        let result = window.server_form.get_value();
                        window.obj().handle_connection_attempt(
                            &result.host,
                            &result.username,
                            &result.password,
                        );
                    } else {
                        warn!("User attempted to submit without completing the form");
                    }
                }
            ));

            // Make sure connect button remains inactive until form is ready
            self.server_form
                .imp()
                .host_entry
                .connect_changed(glib::clone!(
                    #[weak(rename_to=window)]
                    self,
                    move |_| {
                        let sensitive = window.server_form.is_complete();
                        window.connect_button.set_sensitive(sensitive);
                    }
                ));
        }
    }
}
