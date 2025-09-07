use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::subclass::InitializingObject;
use gtk::{
    CompositeTemplate,
    gio::{ActionEntry, prelude::ActionMapExtManual},
    glib,
    prelude::WidgetExt,
};

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
                    dbg!("User attempted to submit without completing the form");
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
