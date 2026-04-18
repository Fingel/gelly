use crate::application::Application;
use crate::backend::BackendError;
use crate::jellyfin::api::MusicDto;
use crate::models::PlaylistModel;

pub fn songs_for_playlist(
    playlist_model: &PlaylistModel,
    app: &Application,
    cb: impl Fn(Result<Vec<MusicDto>, BackendError>) + 'static,
) {
    let library_data = app.library().songs.borrow().clone();
    let jellyfin = app.jellyfin();
    let playlist_type = playlist_model.playlist_type();
    app.http_with_loading(
        async move { playlist_type.load_song_data(&jellyfin, &library_data).await },
        cb,
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
