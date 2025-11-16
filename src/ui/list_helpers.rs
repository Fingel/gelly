use gtk::{
    AnyFilter, FilterListModel, ListItem, PropertyExpression, StringFilter, gio, glib, prelude::*,
};

/// Generic factory setup for media list items
pub fn setup_media_factory<T, W, F, C>(
    factory: &gtk::SignalListItemFactory,
    widget_constructor: C,
    bind_fn: F,
) where
    T: glib::object::IsA<glib::Object> + Clone + 'static,
    W: glib::object::IsA<gtk::Widget> + Clone + 'static,
    C: Fn() -> W + Clone + 'static,
    F: Fn(&T, &W) + 'static,
{
    let widget_constructor_setup = widget_constructor.clone();
    factory.connect_setup(move |_, list_item| {
        let placeholder = widget_constructor_setup();
        let item = list_item
            .downcast_ref::<ListItem>()
            .expect("Needs to be a ListItem");
        item.set_child(Some(&placeholder));
    });

    factory.connect_bind(move |_, list_item| {
        let list_item = list_item
            .downcast_ref::<ListItem>()
            .expect("Needs to be a ListItem");
        let model = list_item
            .item()
            .and_downcast::<T>()
            .expect("Item should be correct model type");
        let widget = list_item
            .child()
            .and_downcast::<W>()
            .expect("Child should be correct widget type");

        bind_fn(&model, &widget);
    });
}

/// Create a string filter for a given property
pub fn create_string_filter<T>(property: &str) -> gtk::StringFilter
where
    T: glib::object::IsA<glib::Object> + 'static,
{
    let expression = PropertyExpression::new(T::static_type(), None::<&gtk::Expression>, property);
    let filter = StringFilter::new(Some(expression));
    filter.set_ignore_case(true);
    filter.set_match_mode(gtk::StringFilterMatchMode::Substring);
    filter
}

/// Generic search implementation for single filter
pub fn apply_single_filter_search(
    query: &str,
    store: &gio::ListStore,
    filter: &gtk::StringFilter,
    grid_view: &gtk::GridView,
) {
    if query.is_empty() {
        let selection_model = gtk::SingleSelection::new(Some(store.clone()));
        grid_view.set_model(Some(&selection_model));
    } else {
        filter.set_search(Some(query));
        let filter_model = FilterListModel::new(Some(store.clone()), Some(filter.clone()));
        let selection_model = gtk::SingleSelection::new(Some(filter_model));
        grid_view.set_model(Some(&selection_model));
    }
}

/// Generic search implementation for multiple filters (like album name + artist)
pub fn apply_multi_filter_search(
    query: &str,
    store: &gio::ListStore,
    filters: &[gtk::StringFilter],
    grid_view: &gtk::GridView,
) {
    if query.is_empty() {
        let selection_model = gtk::SingleSelection::new(Some(store.clone()));
        grid_view.set_model(Some(&selection_model));
    } else {
        for filter in filters {
            filter.set_search(Some(query));
        }

        let any_filter = AnyFilter::new();
        for filter in filters {
            any_filter.append(filter.clone());
        }

        let filter_model = FilterListModel::new(Some(store.clone()), Some(any_filter));
        let selection_model = gtk::SingleSelection::new(Some(filter_model));
        grid_view.set_model(Some(&selection_model));
    }
}

/// Generic activation handler
pub fn handle_grid_activation<T, F>(grid_view: &gtk::GridView, position: u32, activation_fn: F)
where
    T: glib::object::IsA<glib::Object>,
    F: FnOnce(&T),
{
    let selection_model = grid_view
        .model()
        .expect("GridView should have a model")
        .downcast::<gtk::SingleSelection>()
        .expect("Model should be a SingleSelection");
    let current_model = selection_model
        .model()
        .expect("SelectionModel should have a model");
    let item = current_model
        .item(position)
        .expect("Item index invalid")
        .downcast_ref::<T>()
        .expect("Item should be correct type")
        .clone();

    activation_fn(&item);
}
