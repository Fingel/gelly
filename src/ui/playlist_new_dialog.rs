use crate::ui::window::Window;
use adw::prelude::*;
use gtk::glib;

pub fn show(parent: Option<&Window>, cb: impl Fn(String) + 'static) {
    let header_bar = adw::HeaderBar::new();
    header_bar.set_title_widget(Some(&adw::WindowTitle::new("New Playlist", "")));
    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&header_bar);
    let dialog_box = gtk::ListBox::builder()
        .margin_bottom(12)
        .margin_top(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    dialog_box.add_css_class("boxed-list");
    let name_entry = adw::EntryRow::builder().title("Playlist name").build();
    let submit_button = adw::ButtonRow::builder().title("Create").build();
    let cancel_button = adw::ButtonRow::builder().title("Cancel").build();
    cancel_button.add_css_class("destructive-action");
    dialog_box.append(&name_entry);
    dialog_box.append(&submit_button);
    dialog_box.append(&cancel_button);
    toolbar_view.set_content(Some(&dialog_box));

    let dialog = adw::Dialog::builder()
        .can_close(true)
        .child(&toolbar_view)
        .build();

    cancel_button.connect_activated(glib::clone!(
        #[weak]
        dialog,
        move |_| {
            dialog.close();
        }
    ));

    submit_button.connect_activated(glib::clone!(
        #[weak]
        dialog,
        #[weak]
        name_entry,
        move |_| {
            let name = name_entry.text().to_string();
            if !name.is_empty() {
                dialog.close();
                cb(name);
            }
        }
    ));

    dialog.present(parent);
}
