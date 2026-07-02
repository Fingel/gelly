use adw::prelude::{AdwDialogExt, AlertDialogExt, AlertDialogExtManual};
use gtk::{self, gio, prelude::*};

use crate::i18n::tr;
use crate::jellyfin::api::PlaylistDto;

#[derive(Debug, Clone)]
pub struct ContextActions {
    pub can_remove_from_playlist: bool,
    pub in_queue: bool,
    pub action_prefix: String,
    pub go_to_artist: bool,
    pub go_to_album: bool,
    pub show_info_dialog: bool,
}

pub fn construct_menu(config: &ContextActions) -> gtk::PopoverMenu {
    let empty_menu = gio::Menu::new();
    let popover_menu = gtk::PopoverMenu::from_model(Some(&empty_menu));
    let config = config.clone();
    popover_menu.connect_show(move |popover| {
        let menu_model = create_menu_model(&config);
        popover.set_menu_model(Some(&menu_model));
    });
    popover_menu
}

pub fn add_to_playlist_dialog(
    window: Option<&gtk::Window>,
    playlists: Vec<PlaylistDto>,
    cb: impl Fn(Option<String>) + 'static,
) -> adw::AlertDialog {
    let dialog = adw::AlertDialog::new(Some(&tr("Add to Playlist")), None);
    dialog.add_responses(&[("cancel", &tr("Cancel")), ("add", &tr("Add"))]);
    dialog.set_response_appearance("add", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("add"));
    dialog.set_close_response("cancel");

    if !playlists.is_empty() {
        let strings: Vec<&str> = playlists.iter().map(|s| s.name.as_str()).collect();
        let expression = gtk::StringObject::this_expression("string");
        let playlists_dropdown = gtk::DropDown::from_strings(&strings);
        playlists_dropdown.set_enable_search(true);
        playlists_dropdown.set_search_match_mode(gtk::StringFilterMatchMode::Substring);
        playlists_dropdown.set_expression(Some(&expression));

        dialog.set_extra_child(Some(&playlists_dropdown));
        dialog.connect_response(Some("add"), move |_, response| {
            if response == "add" {
                let selected_index = playlists_dropdown.selected();
                if let Some(selected_playlist) = playlists.get(selected_index as usize) {
                    cb(Some(selected_playlist.id.clone()));
                }
            }
        });
    } else {
        dialog.remove_response("add");
        dialog.set_body(&tr(
            "No playlists available. Please create a playlist first.",
        ));
    }
    dialog.present(window);
    dialog
}

fn create_menu_model(config: &ContextActions) -> gio::Menu {
    let menu = gio::Menu::new();
    // Queue section
    if !config.in_queue {
        let queue_section = gio::Menu::new();
        queue_section.append(
            Some(&tr("Queue Next")),
            Some(&format!("{}.queue_next", config.action_prefix)),
        );
        queue_section.append(
            Some(&tr("Queue Last")),
            Some(&format!("{}.queue_last", config.action_prefix)),
        );
        menu.append_section(None, &queue_section);
    }
    // Playlist section
    let playlist_section = gio::Menu::new();
    playlist_section.append(
        Some(&tr("Add to Playlist")),
        Some(&format!("{}.add_to_playlist_dialog", config.action_prefix)),
    );
    if config.can_remove_from_playlist {
        playlist_section.append(
            Some(&tr("Remove from Playlist")),
            Some(&format!("{}.remove_playlist", config.action_prefix)),
        );
    }

    menu.append_section(None, &playlist_section);

    let navigation_section = gio::Menu::new();
    if config.go_to_album {
        navigation_section.append(
            Some(&tr("Go to Album")),
            Some(&format!("{}.go_to_album", config.action_prefix)),
        );
    }

    if config.go_to_artist {
        navigation_section.append(
            Some(&tr("Go to Artist")),
            Some(&format!("{}.go_to_artist", config.action_prefix)),
        );
    }
    menu.append_section(None, &navigation_section);

    let other_section = gio::Menu::new();
    if config.show_info_dialog {
        other_section.append(
            Some(&tr("Song Info")),
            Some(&format!("{}.show_info_dialog", config.action_prefix)),
        );
    } else {
        other_section.append(
            Some(&tr("Copy ID")),
            Some(&format!("{}.copy_id", config.action_prefix)),
        )
    }
    menu.append_section(None, &other_section);

    menu
}
