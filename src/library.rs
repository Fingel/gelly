use crate::jellyfin::{Jellyfin, api::LibraryDto};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct Library {
    jellyfin: Jellyfin,
    library_id: String,
    albums: Arc<Mutex<Vec<LibraryDto>>>,
}

impl Library {
    pub fn new(jellyfin: Jellyfin, library_id: String) -> Self {
        let albums = Arc::new(Mutex::new(vec![]));
        Self {
            jellyfin,
            library_id,
            albums,
        }
    }

    pub async fn refresh(&self) {
        self.fetch_albums().await;
    }

    pub fn get_albums(&self) -> Arc<Mutex<Vec<LibraryDto>>> {
        self.albums.clone()
    }

    pub async fn fetch_albums(&self) {
        let client = self.jellyfin.clone();
        let albums = client.get_albums(&self.library_id).await;
        let parsed = match albums {
            Ok(albums) => albums.items,
            Err(_) => vec![],
        };
        *self.albums.lock().await = parsed;
    }
}
