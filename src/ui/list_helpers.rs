use gtk::{PropertyExpression, glib, prelude::*};

pub fn create_string_filter<T: glib::object::IsA<glib::Object> + 'static>(
    property: &str,
) -> gtk::StringFilter {
    let expr = PropertyExpression::new(T::static_type(), None::<&gtk::Expression>, property);
    let filter = gtk::StringFilter::new(Some(expr));
    filter.set_ignore_case(true);
    filter.set_match_mode(gtk::StringFilterMatchMode::Substring);
    filter
}

pub fn handle_grid_activation<T: glib::object::IsA<glib::Object>, F: FnOnce(&T)>(
    grid_view: &gtk::GridView,
    position: u32,
    f: F,
) {
    if let Some(item) = grid_view
        .model()
        .and_then(|m| m.item(position))
        .and_downcast::<T>()
    {
        f(&item);
    }
}
