use crate::async_utils::spawn_tokio;
use crate::config::{settings, store_jellyfin_api_token};
use crate::jellyfin::{Jellyfin, JellyfinError};
use crate::ui::widget_ext::WidgetApplicationExt;
use adw::prelude::*;
use adw::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use gtk::{gio, glib};
use log::{debug, error, warn};

glib::wrapper! {
    pub struct Setup(ObjectSubclass<imp::Setup>)
    @extends gtk::Widget, gtk::Box,
                @implements gio::ActionMap, gio::ActionGroup;
}

#[derive(Debug)]
pub struct ServerFormValues {
    pub host: String,
    pub username: String,
    pub password: String,
}

impl Setup {
    pub fn new() -> Self {
        Object::builder().build()
    }

    fn select_page(&self) {
        let app = self.get_application();
        if app.jellyfin().is_authenticated() {
            self.show_library_setup();
        } else {
            self.show_server_setup();
        }
    }

    pub fn show_server_setup(&self) {
        let imp = self.imp();
        imp.setup_navigation_view
            .replace(&[imp.setup_servers.get()]);
    }

    pub fn show_library_setup(&self) {
        let imp = self.imp();
        imp.setup_navigation_view
            .replace(&[imp.setup_library.get()]);
        self.populate_library_list();
    }

    pub fn is_complete(&self) -> bool {
        !self.imp().host_entry.text().is_empty()
    }

    pub fn get_value(&self) -> ServerFormValues {
        let imp = self.imp();
        ServerFormValues {
            host: imp.host_entry.text().to_string(),
            username: imp.username_entry.text().to_string(),
            password: imp.password_entry.text().to_string(),
        }
    }

    pub fn host_error(&self) {
        self.imp().host_entry.add_css_class("error");
    }

    pub fn authentication_error(&self) {
        self.imp().username_entry.add_css_class("error");
        self.imp().password_entry.add_css_class("error");
    }

    pub fn clear_errors(&self) {
        self.imp().host_entry.remove_css_class("error");
        self.imp().username_entry.remove_css_class("error");
        self.imp().password_entry.remove_css_class("error");
    }

    pub fn handle_connection_attempt(&self, host: &str, username: &str, password: &str) {
        self.clear_errors();
        let app = self.get_application();
        let host = host.to_string();
        let username = username.to_string();
        let password = password.to_string();
        spawn_tokio(
            async move { Jellyfin::new_authenticate(&host, &username, &password).await },
            glib::clone!(
                #[weak(rename_to=setup)]
                self,
                move |result| {
                    match result {
                        Ok(jellyfin) => {
                            let user_id = jellyfin.user_id.clone();
                            let token = jellyfin.token.clone();
                            let host = jellyfin.host.clone();
                            app.imp().jellyfin.replace(Some(jellyfin));
                            setup.save_server_settings(&host, &user_id, &token);
                            setup.show_library_setup();
                        }
                        Err(err) => setup.handle_connection_error(err),
                    }
                }
            ),
        );
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
                self.host_error();
                self.toast("Error connecting to host", None);
                debug!("Transport error: {}", err);
            }
            JellyfinError::Http { status, message } => {
                self.toast("HTTP {} error when attempting to authenticate", None);
                warn!(
                    "HTTP {} error: {} when attempting to authenticate",
                    status, message
                );
            }
            JellyfinError::AuthenticationFailed { message } => {
                self.toast("Invalid credentials", None);
                self.authentication_error();
                debug!("Authentication failed: {}", message);
            }
            _ => {
                dbg!(error);
            }
        }
    }

    fn populate_library_list(&self) {
        let imp = self.imp();
        let jellyfin = self.get_application().jellyfin();
        let combo = imp.library_combo.clone();
        spawn_tokio(
            async move { jellyfin.get_views().await },
            glib::clone!(
                #[weak(rename_to=setup)]
                self,
                move |result| {
                    match result {
                        Ok(views) => {
                            // If the list contains Music, make it the first element
                            let mut sorted_items = views.items.clone();
                            if let Some(index) = sorted_items
                                .iter()
                                .position(|item| item.name.to_lowercase() == "music")
                            {
                                sorted_items.swap(0, index);
                            }
                            // Make launch button sensitive
                            setup
                                .imp()
                                .library_button
                                .set_sensitive(!sorted_items.is_empty());
                            setup.imp().libraries.replace(sorted_items);
                            let model = gtk::StringList::new(&[]);
                            for item in setup.imp().libraries.borrow().iter() {
                                model.append(&item.name);
                            }
                            combo.set_model(Some(&model));
                        }
                        Err(err) => {
                            error!("Failed to fetch libraries: {:?}", err);
                            setup.toast("Failed to load libraries", None);
                        }
                    }
                }
            ),
        );
    }

    fn handle_library_button_click(&self) {
        let library_id = self.get_selected_library();
        dbg!(library_id);
    }

    fn get_selected_library(&self) -> String {
        let imp = self.imp();
        let selected_index = imp.library_combo.selected() as usize;
        let libraries = imp.libraries.borrow();
        let library = libraries
            .get(selected_index)
            .expect("Failed to get selected library, this should not happen");

        library.id.clone()
    }
}

impl Default for Setup {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use crate::jellyfin::api::BaseItemDto;
    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{CompositeTemplate, glib, prelude::WidgetExt};
    use log::warn;
    use std::cell::RefCell;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/setup.ui")]
    pub struct Setup {
        #[template_child]
        pub setup_navigation_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub setup_servers: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub setup_library: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub host_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub username_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub password_entry: TemplateChild<adw::PasswordEntryRow>,
        #[template_child]
        pub connect_button: TemplateChild<adw::ButtonRow>,
        #[template_child]
        pub library_combo: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub library_button: TemplateChild<adw::ButtonRow>,
        pub libraries: RefCell<Vec<BaseItemDto>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Setup {
        const NAME: &'static str = "GellySetup";
        type Type = super::Setup;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Setup {
        fn constructed(&self) {
            self.parent_constructed();
            self.setup_signals();
        }
    }

    impl Setup {
        fn setup_signals(&self) {
            self.obj().connect_map(|setup| {
                setup.select_page();
            });

            // Setup Connect Button
            self.connect_button.connect_activated(glib::clone!(
                #[weak(rename_to=imp)]
                self,
                move |_| {
                    let obj = imp.obj();
                    if obj.is_complete() {
                        let result = obj.get_value();
                        obj.handle_connection_attempt(
                            &result.host,
                            &result.username,
                            &result.password,
                        );
                    } else {
                        warn!("User attempted to submit without completing the form");
                    }
                }
            ));

            // Setup Library Button
            self.library_button.connect_activated(glib::clone!(
                #[weak(rename_to=imp)]
                self,
                move |_| {
                    let obj = imp.obj();
                    obj.handle_library_button_click();
                }
            ));

            // Make sure connect button remains inactive until form is ready
            self.obj().imp().host_entry.connect_changed(glib::clone!(
                #[weak(rename_to=imp)]
                self,
                move |_| {
                    let obj = imp.obj();
                    let sensitive = obj.is_complete();
                    imp.connect_button.set_sensitive(sensitive);
                }
            ));
        }
    }

    impl WidgetImpl for Setup {}
    impl BoxImpl for Setup {}
}
