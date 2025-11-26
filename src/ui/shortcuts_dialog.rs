use crate::ui::window::Window;
use adw::{ShortcutsDialog, prelude::*};
use gtk::Builder;
use log::error;

pub fn show(parent: &Window) {
    let builder = Builder::from_resource("/io/m51/Gelly/ui/shortcuts_dialog.ui");
    if let Some(dialog) = builder.object::<ShortcutsDialog>("shortcuts-dialog") {
        dialog.present(Some(parent));
    } else {
        error!("Failed to get shortcuts dialog from UI file");
    }
}
