use adw::{AboutDialog, prelude::AdwDialogExt};

use crate::{config, ui::window::Window};

pub fn show(parent: &Window) {
    let dialog = AboutDialog::from_appdata("/io/m51/Gelly/metainfo.xml", Some(config::VERSION));
    #[cfg(debug_assertions)]
    {
        dialog.set_version("Devel");
    }

    dialog.set_developers(&["Austin Riba <austin@m51.io>"]);
    dialog.present(Some(parent));
}
