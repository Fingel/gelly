use gtk::{self, gio, glib, prelude::*};

use crate::jellyfin::api::PlaylistDto;

#[derive(Debug, Clone)]
pub struct ContextActions {
    pub can_remove_from_playlist: bool,
    pub in_queue: bool,
    pub action_prefix: String,
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

fn create_menu_model(config: &ContextActions, playlists: &[PlaylistDto]) -> gio::Menu {
    let menu = gio::Menu::new();
    // Queue section
    if !config.in_queue {
        let queue_section = gio::Menu::new();
        queue_section.append(
            Some("Queue Next"),
            Some(&format!("{}.queue_next", config.action_prefix)),
        );
        queue_section.append(
            Some("Queue Last"),
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
        playlist_section.append_submenu(Some("Add to Playlist"), &playlist_submenu);
    }
    if config.can_remove_from_playlist {
        playlist_section.append(
            Some("Remove from Playlist"),
            Some(&format!("{}.remove_playlist", config.action_prefix)),
        );
    }
    menu.append_section(None, &playlist_section);

    menu
}

pub fn create_actiongroup(
    on_add_to_playlist: Option<impl Fn(String) + 'static>,
    on_remove_from_playlist: Option<impl Fn() + 'static>,
    on_queue_next: Option<impl Fn() + 'static>,
    on_queue_last: Option<impl Fn() + 'static>,
) -> gio::SimpleActionGroup {
    let action_group = gio::SimpleActionGroup::new();

    if let Some(on_add_to_playlist) = on_add_to_playlist {
        let add_to_playlist_action =
            gio::SimpleAction::new("add_to_playlist", Some(glib::VariantTy::STRING));
        add_to_playlist_action.connect_activate(move |_, playlist_id| {
            if let Some(playlist_id) = playlist_id.and_then(|id| id.get::<String>()) {
                on_add_to_playlist(playlist_id);
            }
        });
        action_group.add_action(&add_to_playlist_action);
    }

    if let Some(on_remove_from_playlist) = on_remove_from_playlist {
        let remove_playlist_action = gio::SimpleAction::new("remove_playlist", None);
        remove_playlist_action.connect_activate(glib::clone!(move |_, _| {
            on_remove_from_playlist();
        }));
        action_group.add_action(&remove_playlist_action);
    }

    if let Some(on_queue_next) = on_queue_next {
        let queue_next_action = gio::SimpleAction::new("queue_next", None);
        queue_next_action.connect_activate(glib::clone!(move |_, _| {
            on_queue_next();
        }));
        action_group.add_action(&queue_next_action);
    }

    if let Some(on_queue_last) = on_queue_last {
        let queue_last_action = gio::SimpleAction::new("queue_last", None);
        queue_last_action.connect_activate(glib::clone!(move |_, _| {
            on_queue_last();
        }));
        action_group.add_action(&queue_last_action);
    }

    action_group
}
