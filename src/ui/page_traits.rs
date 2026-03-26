use gtk::glib::{self, prelude::ObjectExt};
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
    fn can_new(&self) -> bool;
    fn play_selected(&self);
    fn create_new(&self) {
        warn!("New not implemented for this type");
    }
    fn search_changed(&self, query: &str) -> ();
    fn sort_bar(&self) -> gtk::SearchBar;
    fn bind_sort_bar(&self, btn: &gtk::ToggleButton) {
        btn.bind_property("active", &self.sort_bar(), "search-mode-enabled")
            .bidirectional()
            .build();
    }
    fn setup_bar_mutual_exclusion(&self, bar: &gtk::SearchBar) {
        let sort_bar = self.sort_bar();
        close_on_open(&sort_bar, bar);
        close_on_open(bar, &sort_bar);
    }
}

fn close_on_open(bar1: &gtk::SearchBar, bar2: &gtk::SearchBar) {
    bar1.connect_search_mode_enabled_notify(glib::clone!(
        #[weak]
        bar2,
        move |bar1| {
            if bar1.is_search_mode() {
                bar2.set_search_mode(false);
            }
        }
    ));
}
