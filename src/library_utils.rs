use crate::application::Application;
use crate::config;
use crate::i18n::tr;
use crate::library::Library;
use crate::models::{PlaylistModel, SongModel};

pub fn songs_for_playlist(model: &PlaylistModel, library: &Library) -> Vec<SongModel> {
    let playlist_type = model.playlist_type();
    if playlist_type.is_smart() {
        playlist_type.smart_songs(library)
    } else {
        library.songs_for_playlist(&playlist_type.to_id())
    }
}

pub fn play_album(id: &str, app: &Application) {
    let songs = app.library().songs_for_album(id);
    if let Some(audio_model) = app.audio_model() {
        audio_model.set_queue(songs, 0, false);
    } else {
        log::warn!("No audio model found");
    }
}

pub fn play_artist(id: &str, app: &Application) {
    let songs = app.library().songs_for_artist(id);
    if let Some(audio_model) = app.audio_model() {
        audio_model.set_queue(songs, 0, false);
    } else {
        log::warn!("No audio model found");
    }
}

pub fn play_song(id: &str, app: &Application) {
    let songs = app.library().all_songs();
    let song = songs.iter().find(|s| s.id() == id);
    if let Some(audio_model) = app.audio_model() {
        if let Some(song) = song {
            audio_model.set_queue(vec![song.clone()], 0, true);
        }
    } else {
        log::warn!("No audio model found");
    }
}

pub fn play_similar(id: &str, app: &Application) {
    if !app.require_online(Some(&tr("Must be online to fetch similar music."))) {
        return;
    }
    let library = app.library();
    let backend = app.backend();
    let id = id.to_string();
    if let Some(audio_model) = app.audio_model() {
        app.http_with_loading(
            async move { backend.get_similar_items(&id, 50).await },
            move |result| {
                if config::get_offline() {
                    return;
                }
                if let Ok(plist_items) = result {
                    let songs: Vec<SongModel> = plist_items
                        .items
                        .iter()
                        .map(|dto| SongModel::new(dto, library.song_is_favorite(&dto.id)))
                        .collect();
                    if songs.len() <= 20 {
                        let msg = tr("The backend did not return many songs, it may require a sonic analysis plugin");
                        audio_model.emit_error(&msg);
                    }
                    if !songs.is_empty() {
                        audio_model.set_queue(songs, 0, true);
                    }
                } else {
                    log::warn!("Failed to get similar items")
                }
            },
        );
    } else {
        log::warn!("No audio model found");
    }
}
