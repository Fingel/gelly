use adw::subclass::prelude::*;
use gtk::glib;
use std::cell::RefCell;

#[derive(Default)]
pub struct Application {
    pub auth_token: RefCell<Option<String>>,
}

#[glib::object_subclass]
impl ObjectSubclass for Application {
    const NAME: &'static str = "GellyApplication";
    type Type = super::Application;
    type ParentType = adw::Application;
}

impl ObjectImpl for Application {}
impl ApplicationImpl for Application {}
impl GtkApplicationImpl for Application {}
impl AdwApplicationImpl for Application {}
