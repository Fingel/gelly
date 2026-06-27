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
}

pub fn construct_menu(
    config: &ContextActions,
    get_playlists: impl Fn() -> Vec<PlaylistDto> + 'static,
) -> gtk::PopoverMenu {
    // We want to populate the menu lazily so playlists are always up to date
    let empty_menu = gio::Menu::new();
    let popover_menu = gtk::PopoverMenu::from_model(Some(&empty_menu));
    let config = config.clone();
    popover_menu.connect_show(move |popover| {
        let playlists = get_playlists();
        let menu_model = create_menu_model(&config, &playlists);
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

    let strings: Vec<&str> = playlists.iter().map(|s| s.name.as_str()).collect();
    let model = gtk::StringList::new(&strings);
    let expression = gtk::PropertyExpression::new(
        gtk::StringObject::static_type(),
        None::<gtk::Expression>,
        "string",
    );
    let playlists_dropdown = gtk::DropDown::builder()
        .model(&model)
        .expression(&expression)
        .enable_search(true)
        .search_match_mode(gtk::StringFilterMatchMode::Substring)
        .build();

    dialog.set_extra_child(Some(&playlists_dropdown));
    dialog.connect_response(Some("add"), move |_, response| {
        if response == "add" {
            let selected_index = playlists_dropdown.selected();
            if let Some(selected_playlist) = playlists.get(selected_index as usize) {
                cb(Some(selected_playlist.id.clone()));
            }
        }
    });
    dialog.present(window);
    dialog
}

fn create_menu_model(config: &ContextActions, playlists: &[PlaylistDto]) -> gio::Menu {
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
    if !playlists.is_empty() {
        let playlist_submenu = gio::Menu::new();
        for playlist in playlists.iter() {
            let playlist_name = playlist.name.clone();
            let playlist_id = playlist.id.clone();
            let menu_item = gio::MenuItem::new(
                Some(&playlist_name),
                Some(&format!("{}.add_to_playlist", config.action_prefix)),
            );
            menu_item.set_action_and_target_value(
                Some(&format!("{}.add_to_playlist", config.action_prefix)),
                Some(&playlist_id.to_variant()),
            );
            playlist_submenu.append_item(&menu_item);
        }
        playlist_section.append_submenu(Some(&tr("Add to Playlist")), &playlist_submenu);
    }
    if config.can_remove_from_playlist {
        playlist_section.append(
            Some(&tr("Remove from Playlist")),
            Some(&format!("{}.remove_playlist", config.action_prefix)),
        );
    }
    playlist_section.append(
        Some(&tr("Add to Playlist...")),
        Some(&format!("{}.add_to_playlist_dialog", config.action_prefix)),
    );

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
    other_section.append(
        Some(&tr("Copy ID")),
        Some(&format!("{}.copy_id", config.action_prefix)),
    );
    menu.append_section(None, &other_section);

    menu
}
