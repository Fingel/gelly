use std::{collections::HashSet, fs, path::PathBuf, sync::Arc, time::Duration};

use gtk::{
    gdk_pixbuf::{Pixbuf, PixbufLoader, prelude::PixbufLoaderExt},
    glib,
};
use log::{debug, warn};
use thiserror::Error;
use tokio::sync::Mutex;

use crate::{
    config::APP_ID,
    jellyfin::{Jellyfin, JellyfinError},
};

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Glib/Pixbuf error")]
    GlibPixbuf(#[from] glib::Error),

    #[error("Jellyfin error: {0}")]
    Jellyfin(#[from] JellyfinError),
}

#[derive(Debug, Clone)]
pub struct ImageCache {
    pending_requests: Arc<Mutex<HashSet<String>>>,
    cache_dir: PathBuf,
}

impl ImageCache {
    pub fn new() -> Result<Self, CacheError> {
        let cache_dir = Self::get_cache_directory()?;
        fs::create_dir_all(&cache_dir)?;
        Ok(Self {
            pending_requests: Arc::new(Mutex::new(HashSet::new())),
            cache_dir,
        })
    }

    fn get_cache_directory() -> Result<PathBuf, CacheError> {
        // TODO: could use a crate like `directories` to make this cross platform
        // TODO: see if these directories are actually correct
        let cache_dir = if let Ok(xdg_cache) = std::env::var("XDG_CACHE_HOME") {
            PathBuf::from(xdg_cache)
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".cache")
        } else {
            PathBuf::from("/tmp")
        };
        Ok(cache_dir.join(APP_ID).join("album-art"))
    }

    pub async fn get_image(
        &self,
        item_id: &str,
        jellyfin: &Jellyfin,
    ) -> Result<Pixbuf, CacheError> {
        loop {
            if let Ok(bytes) = self.load_from_disk(item_id) {
                debug!("Image cache hit: {}", item_id);
                return Self::bytes_to_pixbuf(&bytes);
            }

            // Prevent duplicate requests
            {
                let mut pending = self.pending_requests.lock().await;
                if pending.contains(item_id) {
                    drop(pending);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
                pending.insert(item_id.to_string());
            }

            let result = self.download_and_cache(item_id, jellyfin).await;

            // Remove from pending requests
            {
                let mut pending = self.pending_requests.lock().await;
                pending.remove(item_id);
            }

            return result.and_then(|bytes| Self::bytes_to_pixbuf(&bytes));
        }
    }

    async fn download_and_cache(
        &self,
        item_id: &str,
        jellyfin: &Jellyfin,
    ) -> Result<Vec<u8>, CacheError> {
        debug!("Downloading album art for {}", item_id);
        let image_data = jellyfin.get_image(item_id).await?;

        // Save to disk
        if let Err(e) = self.save_to_disk(item_id, &image_data).await {
            warn!("Failed to save image to disk cache: {}", e);
        }

        Ok(image_data)
    }

    fn load_from_disk(&self, item_id: &str) -> Result<Vec<u8>, CacheError> {
        let file_path = self.cache_dir.join(item_id);
        if !file_path.exists() {
            return Err(CacheError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Cache file not found",
            )));
        }

        Ok(fs::read(&file_path)?)
    }

    async fn save_to_disk(&self, item_id: &str, image_data: &[u8]) -> Result<(), CacheError> {
        let file_path = self.get_cache_file_path(item_id);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&file_path, image_data)?;
        Ok(())
    }

    fn get_cache_file_path(&self, item_id: &str) -> PathBuf {
        self.cache_dir.join(item_id)
    }

    pub fn bytes_to_pixbuf(image_data: &[u8]) -> Result<Pixbuf, CacheError> {
        let loader = PixbufLoader::new();
        loader.write(image_data)?;
        loader.close()?;
        loader.pixbuf().ok_or_else(|| {
            glib::Error::new(
                glib::FileError::Failed,
                "Failed to create pixbuf from image data",
            )
            .into()
        })
    }
}
