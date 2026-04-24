use std::error::Error;

use crate::async_utils::spawn_tokio;
use crate::backend::{Backend, BackendError};
use crate::config::{
    BackendType, set_backend_type, settings, store_jellyfin_api_token, store_subsonic_password,
};
use crate::jellyfin::Jellyfin;
use crate::subsonic::Subsonic;
use crate::ui::widget_ext::WidgetApplicationExt;
use adw::prelude::*;
use adw::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use gtk::{gio, glib};
use log::{debug, error, warn};

glib::wrapper! {
    pub struct Setup(ObjectSubclass<imp::Setup>)
    @extends gtk::Widget, gtk::Box,
                @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

#[derive(Debug)]
pub struct ServerFormValues {
    pub host: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug)]
enum ConnectionAttemptError {
    Both {
        jellyfin: BackendError,
        subsonic: BackendError,
    },
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
        let host = settings().string("hostname");
        if !host.is_empty() {
            imp.host_entry.set_text(&host);
        }
        imp.password_entry.set_text("");
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
        self.imp().connect_button.set_sensitive(false);
        let hosts = if host.starts_with("http://") || host.starts_with("https://") {
            vec![host]
        } else {
            vec![format!("https://{host}"), format!("http://{host}")]
        };
        app.http_with_loading(
            async move {
                let mut last_err = None;
                for host in &hosts {
                    match Jellyfin::new_authenticate(host, &username, &password).await {
                        Ok(jellyfin) => return Ok(Backend::Jellyfin(jellyfin)),
                        Err(jellyfin_err) => {
                            match Subsonic::new_authenticate(host, &username, &password).await {
                                Ok(subsonic) => return Ok(Backend::Subsonic(subsonic)),
                                Err(subsonic_err) => {
                                    last_err = Some(ConnectionAttemptError::Both {
                                        jellyfin: jellyfin_err,
                                        subsonic: subsonic_err,
                                    });
                                }
                            }
                        }
                    }
                }
                Err(last_err.unwrap())
            },
            glib::clone!(
                #[weak(rename_to=setup)]
                self,
                move |result| {
                    match result {
                        Ok(Backend::Jellyfin(jellyfin)) => {
                            let user_id = jellyfin.user_id.clone();
                            let token = jellyfin.token.clone();
                            let host = jellyfin.host.clone();

                            let app = setup.get_application();
                            app.imp().backend.replace(Backend::Jellyfin(jellyfin));

                            if let Err(err) = setup.save_jellyfin_server_settings(&host, &user_id, &token) {
                                setup.toast("Credentials could not be saved. Do you have a keyring daemon running?", None);
                                error!("Failed to save Jellyfin server settings. Aborting: {}", err);
                            }

                            setup.show_library_setup();
                        }
                        Ok(Backend::Subsonic(subsonic)) => {
                            let host = subsonic.host.clone();
                            let username = subsonic.username.clone();
                            let password = subsonic.password.clone();

                            let app = setup.get_application();
                            app.imp().backend.replace(Backend::Subsonic(subsonic));

                            if let Err(err) = setup.save_subsonic_server_settings(&host, &username, &password) {
                                setup.toast("Credentials could not be saved. Do you have a keyring daemon running?", None);
                                error!("Failed to save server settings. Aborting: {}", err);
                            }
                            setup.show_library_setup();
                        }
                        Err(err) => setup.handle_connection_error(err),
                    }
                    setup.imp().connect_button.set_sensitive(true);
                }
            ),
        );
    }

    fn save_jellyfin_server_settings(
        &self,
        host: &str,
        user_id: &str,
        token: &str,
    ) -> Result<(), Box<dyn Error>> {
        settings().set_string("hostname", host)?;
        settings().set_string("user-id", user_id)?;
        settings().set_string("subsonic-username", "")?;
        set_backend_type(BackendType::Jellyfin);
        store_jellyfin_api_token(host, user_id, token)?;
        Ok(())
    }

    fn save_subsonic_server_settings(
        &self,
        host: &str,
        username: &str,
        password: &str,
    ) -> Result<(), Box<dyn Error>> {
        settings().set_string("hostname", host)?;
        settings().set_string("user-id", "")?;
        settings().set_string("subsonic-username", username)?;
        set_backend_type(BackendType::Subsonic);
        store_subsonic_password(host, username, password)?;
        Ok(())
    }

    fn handle_connection_error(&self, error: ConnectionAttemptError) {
        match error {
            ConnectionAttemptError::Both { jellyfin, subsonic } => {
                let jellyfin_transport = matches!(jellyfin, BackendError::Transport(_));
                let subsonic_transport = matches!(subsonic, BackendError::Transport(_));

                if jellyfin_transport && subsonic_transport {
                    self.host_error();
                    self.toast(
                        "Connection error. Please supply a full URL (http://example.com:8096)",
                        None,
                    );
                    debug!(
                        "Authentication failed: both backends had transport errors (jellyfin={:?}, subsonic={:?})",
                        jellyfin, subsonic
                    );
                    return;
                }

                let jellyfin_auth = matches!(jellyfin, BackendError::AuthenticationFailed { .. });
                let subsonic_auth = matches!(subsonic, BackendError::AuthenticationFailed { .. });

                if jellyfin_auth || subsonic_auth {
                    self.toast("Invalid credentials", None);
                    self.authentication_error();
                    debug!(
                        "Authentication failed: jellyfin={:?}, subsonic={:?}",
                        jellyfin, subsonic
                    );
                    return;
                }

                self.toast("Could not authenticate with Jellyfin or Subsonic", None);
                warn!(
                    "Authentication failed for both backends. jellyfin={:?}, subsonic={:?}",
                    jellyfin, subsonic
                );
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
        settings()
            .set_string("library-id", &library_id)
            .expect("Failed to save library id");
        let app = self.get_application();
        app.imp().library_id.replace(library_id);
        app.refresh_all(true);
        // We did it!
        self.get_root_window().show_main_page();
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
    use crate::jellyfin::api::LibraryDto;
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
        pub connect_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub library_combo: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub library_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub cancel_library_button: TemplateChild<gtk::Button>,
        pub libraries: RefCell<Vec<LibraryDto>>,
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
            self.connect_button.connect_clicked(glib::clone!(
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
            self.library_button.connect_clicked(glib::clone!(
                #[weak(rename_to=imp)]
                self,
                move |_| {
                    let obj = imp.obj();
                    obj.handle_library_button_click();
                }
            ));

            self.cancel_library_button.connect_clicked(glib::clone!(
                #[weak(rename_to=imp)]
                self,
                move |_| {
                    imp.obj().show_server_setup();
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
