use std::{
    collections::HashSet, fs, num::NonZeroUsize, os::unix, path::PathBuf, sync::Arc, time::Duration,
};

use gtk::gdk;
use log::{debug, warn};
use lru::LruCache;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;
use tokio::sync::{Mutex, Semaphore};

use crate::{
    async_utils::run_on_tokio,
    backend::{Backend, BackendError},
    config::APP_ID,
    jellyfin::api::{
        FavoriteDto, FavoriteDtoList, ImageType, MusicDto, MusicDtoList, PlaylistDto,
        PlaylistDtoList,
    },
    ui::image_utils::bytes_to_texture,
};

// Cache versions of the library structs that fail on deserialization errors instead of skipping.
// We need this so that we fail reading from the cache if any item fails to deserialize.
// This is opposed to the behavior of reading from the server where we want to skip items.
// The application will refresh from the server (its probably because structs are out of date).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MusicDtoListCache {
    pub items: Vec<MusicDto>,
    pub total_record_count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PlaylistDtoListCache {
    pub items: Vec<PlaylistDto>,
    pub total_record_count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct FavoritesDtoListCache {
    pub items: Vec<FavoriteDto>,
}

impl From<MusicDtoListCache> for MusicDtoList {
    fn from(cache: MusicDtoListCache) -> Self {
        Self {
            items: cache.items,
            total_record_count: cache.total_record_count,
        }
    }
}

impl From<PlaylistDtoListCache> for PlaylistDtoList {
    fn from(cache: PlaylistDtoListCache) -> Self {
        Self {
            items: cache.items,
            total_record_count: cache.total_record_count,
        }
    }
}

impl From<FavoritesDtoListCache> for FavoriteDtoList {
    fn from(cache: FavoritesDtoListCache) -> Self {
        Self { items: cache.items }
    }
}

pub trait Cacheable: DeserializeOwned + Serialize {
    type Loader: DeserializeOwned + Into<Self>;
    const CACHE_FILE_NAME: &'static str;
}

impl Cacheable for MusicDtoList {
    type Loader = MusicDtoListCache;
    const CACHE_FILE_NAME: &'static str = "library.json";
}

impl Cacheable for PlaylistDtoList {
    type Loader = PlaylistDtoListCache;
    const CACHE_FILE_NAME: &'static str = "playlists.json";
}

impl Cacheable for FavoriteDtoList {
    type Loader = FavoritesDtoListCache;
    const CACHE_FILE_NAME: &'static str = "favorites.json";
}

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Jellyfin error: {0}")]
    Jellyfin(#[from] BackendError),

    #[error("Deserialization error: {0}")]
    Deserialize(#[from] serde_json::Error),

    #[error("Image decode error: {0}")]
    Decode(String),
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

    fn save_to_disk(&self, fname: &str, data: &[u8]) -> Result<(), CacheError> {
        let path = self.cache_dir.join(fname);
        fs::write(path, data)?;
        Ok(())
    }

    fn load_from_disk(&self, fname: &str) -> Result<Vec<u8>, CacheError> {
        let path = self.cache_dir.join(fname);
        Ok(fs::read(path)?)
    }

    pub fn clear(&self) -> Result<(), CacheError> {
        fs::remove_dir_all(&self.cache_dir)?;
        fs::create_dir_all(&self.cache_dir)?;
        Ok(())
    }

    pub fn load<T: Cacheable>(&self) -> Result<T, CacheError> {
        let fname = T::CACHE_FILE_NAME;
        let data = self.load_from_disk(fname)?;
        let parsed: T::Loader = serde_json::from_slice(&data)?;
        Ok(parsed.into())
    }

    pub fn save<T: Cacheable>(&self, data: &T) -> Result<(), CacheError> {
        let data = serde_json::to_string(data)?;
        self.save_to_disk(T::CACHE_FILE_NAME, data.as_bytes())?;
        Ok(())
    }
}

const MAX_TEXTURE_CACHE_ENTRIES: usize = 10_000;

type TextureCache = LruCache<String, gdk::Texture>;

#[derive(Debug, Clone)]
pub struct ImageCache {
    pending_requests: Arc<Mutex<HashSet<String>>>,
    download_semaphore: Arc<Semaphore>,
    texture_cache: Arc<std::sync::Mutex<TextureCache>>,
    cache_dir: PathBuf,
}

impl ImageCache {
    // TODO: move the jellyfin logic into an image service or something
    pub fn new() -> Result<Self, CacheError> {
        const MAX_CONCURRENT_DOWNLOADS: usize = 4;
        let cache_dir = get_cache_directory("album-art")?;
        fs::create_dir_all(&cache_dir)?;
        Ok(Self {
            pending_requests: Arc::new(Mutex::new(HashSet::new())),
            download_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_DOWNLOADS)),
            texture_cache: Arc::new(std::sync::Mutex::new(LruCache::new(
                NonZeroUsize::new(MAX_TEXTURE_CACHE_ENTRIES).unwrap(),
            ))),
            cache_dir,
        })
    }

    /// Retrieve the main album art for an item.
    /// Fallback is an id (usually a parent item) that is more likely to have an image.
    async fn get_primary_images(
        &self,
        primary: &str,
        fallback: Option<&str>,
        jellyfin: &Backend,
    ) -> Result<Vec<u8>, CacheError> {
        match fallback {
            None => self.get_image(primary, ImageType::Primary, jellyfin).await,
            Some(fallback) => {
                if let Ok(primary_image) =
                    self.get_image(primary, ImageType::Primary, jellyfin).await
                {
                    Ok(primary_image)
                } else {
                    let fallback_image =
                        self.get_image(fallback, ImageType::Primary, jellyfin).await;
                    if fallback_image.is_ok() {
                        let primary_path = self.get_cache_file_path(primary, ImageType::Primary);
                        let fallback_path = self.get_cache_file_path(fallback, ImageType::Primary);
                        unix::fs::symlink(&fallback_path, &primary_path)?;
                    }
                    fallback_image
                }
            }
        }
    }

    async fn get_image(
        &self,
        item_id: &str,
        image_type: ImageType,
        jellyfin: &Backend,
    ) -> Result<Vec<u8>, CacheError> {
        loop {
            if let Ok(bytes) = self.load_from_disk(item_id, image_type) {
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

            // Acquire semaphore permit to limit concurrent downloads
            let _permit = self.download_semaphore.acquire().await.unwrap();
            let result = self.download_and_cache(item_id, image_type, jellyfin).await;

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
        image_type: ImageType,
        jellyfin: &Backend,
    ) -> Result<Vec<u8>, CacheError> {
        debug!("Downloading album art for {}", item_id);
        let image_data = jellyfin.get_image(item_id, image_type).await?;

        if let Err(e) = self.save_to_disk(item_id, image_type, &image_data) {
            warn!("Failed to save image to disk cache: {}", e);
        }

        Ok(image_data)
    }

    fn load_from_disk(&self, item_id: &str, image_type: ImageType) -> Result<Vec<u8>, CacheError> {
        let file_path = self.get_cache_file_path(item_id, image_type);
        if !file_path.exists() {
            return Err(CacheError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Cache file not found",
            )));
        }

        Ok(fs::read(&file_path)?)
    }

    fn save_to_disk(
        &self,
        item_id: &str,
        image_type: ImageType,
        image_data: &[u8],
    ) -> Result<(), CacheError> {
        let file_path = self.get_cache_file_path(item_id, image_type);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&file_path, image_data)?;
        Ok(())
    }

    pub fn get_cache_file_path(&self, item_id: &str, image_type: ImageType) -> PathBuf {
        match image_type {
            ImageType::Primary => self.cache_dir.join(item_id),
            _ => self
                .cache_dir
                .join(format!("{}/{}", image_type.as_str(), item_id)),
        }
    }

    fn cached_texture(&self, item_id: &str, image_type: ImageType) -> Option<gdk::Texture> {
        let key = format!("{}:{}", image_type.as_str(), item_id);
        self.texture_cache.lock().unwrap().get(&key).cloned()
    }

    /// Decode raw image bytes into a Texture using glycin, cache it, and return it.
    async fn decode_and_cache_texture(
        &self,
        item_id: &str,
        image_type: ImageType,
        image_data: &[u8],
    ) -> Result<gdk::Texture, CacheError> {
        let texture = bytes_to_texture(image_data)
            .await
            .map_err(|e| CacheError::Decode(e.to_string()))?;
        let key = format!("{}:{}", image_type.as_str(), item_id);
        self.texture_cache.lock().unwrap().put(key, texture.clone());
        Ok(texture)
    }

    /// Get or fetch a texture for a specific image type.
    pub async fn get_texture(
        &self,
        item_id: &str,
        image_type: ImageType,
        jellyfin: &Backend,
    ) -> Result<gdk::Texture, CacheError> {
        if let Some(texture) = self.cached_texture(item_id, image_type) {
            return Ok(texture);
        }
        let image_data = run_on_tokio({
            let cache = self.clone();
            let item_id = item_id.to_string();
            let jellyfin = jellyfin.clone();
            async move { cache.get_image(&item_id, image_type, &jellyfin).await }
        })
        .await?;
        self.decode_and_cache_texture(item_id, image_type, &image_data)
            .await
    }

    /// Get or fetch the primary texture for an item, with optional fallback id.
    pub async fn get_primary_texture(
        &self,
        primary: &str,
        fallback: Option<&str>,
        jellyfin: &Backend,
    ) -> Result<gdk::Texture, CacheError> {
        if let Some(texture) = self.cached_texture(primary, ImageType::Primary) {
            return Ok(texture);
        }
        let image_data = run_on_tokio({
            let cache = self.clone();
            let primary = primary.to_string();
            let fallback = fallback.map(str::to_string);
            let jellyfin = jellyfin.clone();
            async move {
                cache
                    .get_primary_images(&primary, fallback.as_deref(), &jellyfin)
                    .await
            }
        })
        .await?;
        self.decode_and_cache_texture(primary, ImageType::Primary, &image_data)
            .await
    }

    pub fn clear_cache(&self) {
        self.texture_cache.lock().unwrap().clear();
        _ = fs::remove_dir_all(&self.cache_dir);
        _ = fs::create_dir_all(&self.cache_dir);
    }
}
