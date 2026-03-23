use gtk::{
    glib::{self, clone, object::ObjectExt},
    prelude::WidgetExt,
};
use log::warn;

use crate::models::model_traits::ItemModel;

pub trait DetailPage {
    type Model: ItemModel;

    fn title(&self) -> String {
        self.get_model()
            .map(|m| m.display_name())
            .unwrap_or_default()
    }

    fn id(&self) -> String {
        self.get_model().map(|m| m.item_id()).unwrap_or_default()
    }

    fn set_model(&self, model: &Self::Model);
    fn get_model(&self) -> Option<Self::Model>;
}

pub trait TopPage {
    fn can_search(&self) -> bool;
    fn can_sort(&self) -> bool;
    fn can_new(&self) -> bool;
    fn toggle_search_bar(&self) {
        if let Some(b) = self.search_bar() {
            b.set_search_mode(!b.is_search_mode());
        }
    }
    fn toggle_sort_bar(&self) {
        if let Some(b) = self.sort_bar() {
            b.set_search_mode(!b.is_search_mode());
        }
    }
    fn play_selected(&self);
    fn create_new(&self) {
        warn!("New not implemented for this type");
    }
    fn search_bar(&self) -> Option<gtk::SearchBar>;
    fn sort_bar(&self) -> Option<gtk::SearchBar>;
    fn search_entry(&self) -> Option<gtk::SearchEntry>;
    fn bind_search_btn(
        &self,
        btn: &gtk::ToggleButton,
        stack: adw::ViewStack,
        stack_page: gtk::Widget,
    ) {
        bind_bar_generic(
            btn,
            self.search_bar(),
            self.sort_bar(),
            self.search_entry(),
            Some(stack),
            Some(stack_page),
        );
    }
    fn bind_sort_btn(&self, btn: &gtk::ToggleButton) {
        bind_bar_generic(
            btn,
            self.sort_bar(),
            self.search_bar(),
            self.search_entry(),
            None,
            None,
        );
    }
}

fn bind_bar_generic(
    btn: &gtk::ToggleButton,
    bar: Option<gtk::SearchBar>,
    other_bar: Option<gtk::SearchBar>,
    search_entry: Option<gtk::SearchEntry>,
    stack: Option<adw::ViewStack>,
    stack_page: Option<gtk::Widget>,
) {
    if let Some(bar) = bar {
        btn.bind_property("active", &bar, "search-mode-enabled")
            .bidirectional()
            .build();
        // make sure we make the 2 bars mutually exclusive
        bar.connect_search_mode_enabled_notify(move |bar| {
            if bar.is_search_mode() {
                if let Some(obar) = other_bar.as_ref() {
                    obar.set_search_mode(false);
                }
                if let Some(entry) = search_entry.as_ref()
                    && let Some(stack) = stack.as_ref()
                    && let Some(page) = stack_page.as_ref()
                    && stack.visible_child().is_some_and(|child| child == *page)
                {
                    glib::idle_add_local_once(clone!(
                        #[weak]
                        entry,
                        move || {
                            entry.grab_focus();
                        }
                    ));
                }
            }
        });
    }
}
