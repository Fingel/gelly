use crate::config::APP_ID;

use chacha20poly1305::{
    ChaCha20Poly1305, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use hkdf::Hkdf;
use log::debug;
use rand::Rng;
use sha2::Sha256;
use std::{collections::HashMap, fs, path::PathBuf};

// Bumping this version will invalidate all existing derived keys
const KEY_SCHEME_VERSION: u8 = 1;

// Retrieves master secret, defined by app id
pub async fn retrieve_secret() -> ashpd::Result<Vec<u8>> {
    ashpd::desktop::secret::retrieve().await
}

// Derives a 32-byte encryption key from the master secret using HKDF-SHA256
pub fn derive_secret(master_secret: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(None, master_secret);
    let mut key = [0u8; 32];

    // Info format should be change via KEY_SCHEME_VERSION
    let info = format!("keystore-{}", KEY_SCHEME_VERSION);
    hk.expand(info.as_bytes(), &mut key)
        .expect("HKDF expand should not fail for 32 bytes");
    key
}

// Returns the path to the encrypted store file, following XDG_DATA_DIR
// e.g. ~/.local/share/gelly/store.json.enc
fn store_path() -> PathBuf {
    // Split app id
    let appid: Vec<&str> = APP_ID.split('.').collect();
    let proj = directories::ProjectDirs::from(appid[0], appid[1], appid[2])
        .expect("Data directory could not be determined");

    proj.data_dir().join("token.json.enc")
}

// File layout: [1-byte version][12-byte nonce][ciphertext+tag]
pub async fn save_secrets(map: HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>> {
    let master = retrieve_secret()
        .await
        .map_err(|e| format!("Couldn't retrieve secret, check XDG Portal availability: ({e})"))?;

    let key = derive_secret(&master);
    let plaintext = serde_json::to_vec(&map)?;
    let cipher = ChaCha20Poly1305::new(chacha20poly1305::Key::from_slice(&key));

    let mut nonce_bytes = [0u8; 12];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Bind ciphertext to this app and scheme version — prevents swapping
    // store files between apps or versions from decrypting successfully
    let aad = format!("{}|{}", APP_ID, KEY_SCHEME_VERSION);
    let ciphertext = cipher
        .encrypt(
            nonce,
            Payload {
                msg: &plaintext,
                aad: aad.as_bytes(),
            },
        )
        .map_err(|e| format!("Encryption failed: {e}"))?;

    let path = store_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write header + nonce + ciphertext
    let mut out = Vec::with_capacity(1 + 12 + ciphertext.len());
    out.push(KEY_SCHEME_VERSION);
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    fs::write(path, out)?;

    debug!("Secrets saved (version={KEY_SCHEME_VERSION}, app={APP_ID})");
    Ok(())
}

pub async fn load_secrets() -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    debug!("Loading secrets...");
    let bytes = match fs::read(store_path()) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            debug!("No secrets store found, returning empty map.");
            return Ok(HashMap::new());
        }
        Err(e) => return Err(e.into()),
    };

    // Parse header
    let (&version, rest) = bytes.split_first().ok_or("Store file corrupt: empty")?;
    if version != KEY_SCHEME_VERSION {
        return Err(format!("Unsupported store version: {version}").into());
    }

    let (nonce_bytes, ciphertext) = rest
        .split_at_checked(12)
        .ok_or("Store file corrupt: too short")?;

    let master = retrieve_secret().await?;
    let key = derive_secret(&master);
    let cipher = ChaCha20Poly1305::new((&key).into());

    // AAD must match exactly what was used during encryption
    let aad = format!("{}|{}", APP_ID, KEY_SCHEME_VERSION);
    let plaintext = cipher
        .decrypt(
            Nonce::from_slice(nonce_bytes),
            Payload {
                msg: ciphertext,
                aad: aad.as_bytes(),
            },
        )
        .map_err(|_| "Decryption failed: corrupt file, wrong key, or app/version mismatch")?;

    let map: HashMap<String, String> = serde_json::from_slice(&plaintext)?;
    Ok(map)
}

pub fn clear_secrets() {
    let path = store_path();
    match fs::remove_file(&path) {
        Ok(_) => debug!("Secrets store cleared."),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            debug!("No secrets store to clear.");
        }
        Err(e) => log::error!("Failed to clear secrets store: {e}"),
    }
}
