use std::{
    collections::HashSet,
    fs,
    io::{Read, Write},
    os::unix,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use flate2::{Compression, read::GzDecoder, write::GzEncoder};
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

    #[error("Jellyfin error: {0}")]
    Jellyfin(#[from] JellyfinError),
}

fn get_cache_directory(name: &str) -> Result<PathBuf, CacheError> {
    let cache_dir = if let Ok(xdg_cache) = std::env::var("XDG_CACHE_HOME") {
        PathBuf::from(xdg_cache)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".cache")
    } else {
        PathBuf::from("/tmp")
    };
    Ok(cache_dir.join(APP_ID).join(name))
}

#[derive(Debug, Clone)]
pub struct LibraryCache {
    cache_dir: PathBuf,
}

impl LibraryCache {
    pub fn new() -> Result<Self, CacheError> {
        let cache_dir = get_cache_directory("library")?;
        fs::create_dir_all(&cache_dir)?;
        Ok(Self { cache_dir })
    }

    // TODO: use enums instead of strings for fname here
    pub fn save_to_disk(&self, fname: &str, data: &[u8]) -> Result<(), CacheError> {
        let path = self.cache_dir.join(fname);
        let file = fs::File::create(path)?;
        let mut encoder = GzEncoder::new(file, Compression::default());
        encoder.write_all(data)?;
        encoder.finish()?;
        Ok(())
    }

    pub fn load_from_disk(&self, fname: &str) -> Result<Vec<u8>, CacheError> {
        let path = self.cache_dir.join(fname);
        let file = fs::File::open(path)?;
        let mut decoder = GzDecoder::new(file);
        let mut json_str = Vec::new();
        decoder.read_to_end(&mut json_str)?;
        Ok(json_str)
    }

    pub fn clear(&self) -> Result<(), CacheError> {
        fs::remove_dir_all(&self.cache_dir)?;
        fs::create_dir_all(&self.cache_dir)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ImageCache {
    pending_requests: Arc<Mutex<HashSet<String>>>,
    cache_dir: PathBuf,
}

impl ImageCache {
    pub fn new() -> Result<Self, CacheError> {
        let cache_dir = get_cache_directory("album-art")?;
        fs::create_dir_all(&cache_dir)?;
        Ok(Self {
            pending_requests: Arc::new(Mutex::new(HashSet::new())),
            cache_dir,
        })
    }

    pub async fn get_images(
        &self,
        primary: &str,
        fallback: Option<&str>,
        jellyfin: &Jellyfin,
    ) -> Result<Vec<u8>, CacheError> {
        match fallback {
            None => self.get_image(primary, jellyfin).await,
            Some(fallback) => {
                if let Ok(primary_image) = self.get_image(primary, jellyfin).await {
                    Ok(primary_image)
                } else {
                    let fallback_image = self.get_image(fallback, jellyfin).await;
                    if fallback_image.is_ok() {
                        let primary_path = self.get_cache_file_path(primary);
                        let fallback_path = self.get_cache_file_path(fallback);
                        unix::fs::symlink(&fallback_path, &primary_path)?;
                    }
                    fallback_image
                }
            }
        }
    }

    async fn get_image(&self, item_id: &str, jellyfin: &Jellyfin) -> Result<Vec<u8>, CacheError> {
        // We should probably be using hashes here, many IDs have the same image.
        loop {
            if let Ok(bytes) = self.load_from_disk(item_id) {
                return Ok(bytes);
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

            return result;
        }
    }

    async fn download_and_cache(
        &self,
        item_id: &str,
        jellyfin: &Jellyfin,
    ) -> Result<Vec<u8>, CacheError> {
        debug!("Downloading album art for {}", item_id);
        let image_data = jellyfin.get_image(item_id).await?;

        if let Err(e) = self.save_to_disk(item_id, &image_data) {
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

    fn save_to_disk(&self, item_id: &str, image_data: &[u8]) -> Result<(), CacheError> {
        let file_path = self.get_cache_file_path(item_id);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&file_path, image_data)?;
        Ok(())
    }

    pub fn get_cache_file_path(&self, item_id: &str) -> PathBuf {
        self.cache_dir.join(item_id)
    }

    pub fn clear_cache(&self) {
        _ = fs::remove_dir_all(&self.cache_dir);
        _ = fs::create_dir_all(&self.cache_dir);
    }
}
