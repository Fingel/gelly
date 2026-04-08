use gtk::{glib::prelude::*, prelude::*};
use log::warn;

use crate::models::model_traits::ItemModel;

#[derive(Debug, Clone, Copy)]
pub enum SortType {
    DateAdded,
    Name,
    Artist,
    Year,
    PlayCount,
    NumSongs,
    Album,
}

impl SortType {
    pub fn as_str(&self) -> &str {
        match self {
            SortType::DateAdded => "Recently Added",
            SortType::Name => "Name",
            SortType::Artist => "Album Artist",
            SortType::Year => "Year",
            SortType::PlayCount => "Play Count",
            SortType::NumSongs => "Num. Songs",
            SortType::Album => "Album",
        }
    }
}

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
    fn sort_options(&self) -> &[SortType];
    fn current_sort_by(&self) -> u32;
    fn current_sort_direction(&self) -> u32;
    fn apply_sort(&self, sort_by: u32, direction: u32);
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
}
