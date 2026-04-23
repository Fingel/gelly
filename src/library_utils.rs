use crate::application::Application;
use crate::backend::BackendError;
use crate::models::{PlaylistModel, SongModel};

pub fn songs_for_playlist(
    playlist_model: &PlaylistModel,
    app: &Application,
    cb: impl Fn(Result<Vec<SongModel>, BackendError>) + 'static,
) {
    let library = app.library();
    let library_data = library.songs.borrow().clone();
    let jellyfin = app.jellyfin();
    let playlist_type = playlist_model.playlist_type();
    app.http_with_loading(
        async move { playlist_type.load_song_data(&jellyfin, &library_data).await },
        move |result| {
            cb(result.map(|music_data| {
                music_data
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
