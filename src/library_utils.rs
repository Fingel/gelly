use crate::application::Application;
use crate::backend::BackendError;
use crate::models::{PlaylistModel, SongModel};

pub fn songs_for_playlist(
    playlist_model: &PlaylistModel,
    app: &Application,
    cb: impl Fn(Result<Vec<SongModel>, BackendError>) + 'static,
) {
    let library = app.library();
    let playlist_type = playlist_model.playlist_type();

    // Smart playlists work on the main thread - return immediately
    if playlist_type.is_smart() {
        cb(Ok(playlist_type.smart_songs(&library)));
        return;
    }

    // Regular playlists are fetched from the backend
    let id = playlist_type.to_id();
    let jellyfin = app.jellyfin();
    app.http_with_loading(
        async move { jellyfin.get_playlist_items(&id).await },
        move |result| {
            cb(result.map(|items| {
                items
                    .items
                    .iter()
                    .map(|dto| SongModel::new(dto, library.song_is_favorite(&dto.id)))
                    .collect()
            }))
        },
    );
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
