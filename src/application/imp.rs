use adw::subclass::prelude::*;
use gtk::glib;
use std::cell::RefCell;

use crate::jellyfin::Jellyfin;

#[derive(Default)]
pub struct Application {
    pub jellyfin: RefCell<Option<Jellyfin>>,
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
