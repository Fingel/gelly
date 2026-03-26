use gtk::{glib::prelude::*, prelude::*};
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
    fn search_changed(&self, query: &str);
    fn sort_bar(&self) -> gtk::SearchBar;
    fn setup_search_connection(&self, search_entry: &gtk::SearchEntry)
    where
        Self: gtk::prelude::ObjectType,
    {
        let weak_self = self.downgrade();
        search_entry.connect_search_changed(move |entry| {
            if let Some(list_view) = weak_self.upgrade() {
                list_view.search_changed(&entry.text());
            }
        });
    }
    fn bind_sort_bar(&self, btn: &gtk::ToggleButton) {
        btn.bind_property("active", &self.sort_bar(), "search-mode-enabled")
            .bidirectional()
            .build();
    }
}
