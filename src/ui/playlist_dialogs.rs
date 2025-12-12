use crate::ui::window::Window;
use adw::prelude::*;
use gtk::glib;

pub fn new_playlist(parent: Option<&Window>, cb: impl Fn(String) + 'static) {
    let name_entry = adw::EntryRow::builder().title("Playlist name").build();
    let entry_box = gtk::ListBox::builder()
        .margin_top(12)
        .margin_bottom(12)
        .build();
    entry_box.add_css_class("boxed-list");
    entry_box.append(&name_entry);

    let dialog = adw::AlertDialog::builder()
        .heading("New Playlist")
        .extra_child(&entry_box)
        .build();

    dialog.add_responses(&[("cancel", "Cancel"), ("create", "Create")]);
    dialog.set_response_appearance("create", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("create"));
    dialog.set_close_response("cancel");

    dialog.connect_response(
        None,
        glib::clone!(
            #[weak]
            dialog,
            #[weak]
            name_entry,
            move |_, response| {
                if response == "create" {
                    let name = name_entry.text().to_string();
                    if !name.is_empty() {
                        cb(name);
                    }
                }
                dialog.close();
            }
        ),
    );

    dialog.present(parent);
}

pub fn confirm_delete(parent: Option<&Window>, cb: impl Fn(bool) + 'static) {
    let dialog = adw::AlertDialog::new(Some("Delete playlist?"), None);
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");
    dialog.add_responses(&[("cancel", "Cancel"), ("delete", "Delete")]);
    dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
    dialog.connect_response(
        None,
        glib::clone!(
            #[weak]
            dialog,
            move |_, response| {
                if response == "delete" {
                    cb(true);
                } else {
                    cb(false);
                }
                dialog.close();
            }
        ),
    );
    dialog.present(parent);
}
