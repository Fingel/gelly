use super::PlaybackEvent;
use log::info;

/// Jellyfin reporter stub - to be implemented later
#[derive(Debug)]
pub struct JellyfinReporter {
    enabled: bool,
}

impl JellyfinReporter {
    pub fn new() -> Self {
        Self {
            enabled: false, // Disabled by default until implemented
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub async fn handle_event(
        &mut self,
        event: PlaybackEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Stub - do nothing
        Ok(())
    }
}
