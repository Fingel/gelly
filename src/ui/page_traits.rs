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
    fn hide_search_bar(&self);
    fn hide_sort_bar(&self);
    fn toggle_search_bar(&self);
    fn toggle_sort_bar(&self);
    fn play_selected(&self);
    fn create_new(&self) {
        warn!("New not implemented for this type");
    }
    fn toggle_bar(&self, bar: &gtk::SearchBar) {
        bar.set_search_mode(!bar.is_search_mode());
    }
}
