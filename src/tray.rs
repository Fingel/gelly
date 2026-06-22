use ksni::blocking::{Handle, TrayMethods};
use ksni::menu::StandardItem;
use log::warn;

use crate::config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    ShowWindow,
    Quit,
}

pub struct TrayService {
    handle: Handle<GellyTray>,
}

impl TrayService {
    pub fn spawn(sender: async_channel::Sender<TrayCommand>) -> Option<Self> {
        let tray = GellyTray { sender };
        let handle = match tray.assume_sni_available(true).spawn() {
            Ok(handle) => handle,
            Err(error) => {
                warn!("Failed to start tray service: {}", error);
                return None;
            }
        };

        Some(Self { handle })
    }
}

impl Drop for TrayService {
    fn drop(&mut self) {
        self.handle.shutdown().wait();
    }
}

struct GellyTray {
    sender: async_channel::Sender<TrayCommand>,
}

impl GellyTray {
    fn send(&self, command: TrayCommand) {
        let _ = self.sender.send_blocking(command);
    }

    fn menu_item(label: &str, command: TrayCommand) -> ksni::MenuItem<Self> {
        StandardItem {
            label: label.to_string(),
            activate: Box::new(move |tray: &mut GellyTray| {
                println!(
                    "menu item activated on thread: {:?}",
                    std::thread::current().id()
                );
                tray.send(command);
            }),
            ..Default::default()
        }
        .into()
    }
}

impl ksni::Tray for GellyTray {
    fn id(&self) -> String {
        config::APP_ID.to_string()
    }

    fn icon_name(&self) -> String {
        config::APP_ID.to_string()
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        self.send(TrayCommand::ShowWindow);
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        vec![
            Self::menu_item("Show Gelly", TrayCommand::ShowWindow),
            Self::menu_item("Quit", TrayCommand::Quit),
        ]
    }
}
