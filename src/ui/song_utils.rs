use crate::{
    audio::model::AudioModel,
    library_utils::{album_for_item, artist_for_item},
    models::SongModel,
    ui::{song::Song, widget_ext::WidgetApplicationExt, window::Window},
};
use gtk::{glib, prelude::*, subclass::prelude::*};

/// Used for navigation on a song widget artist/album labels
pub fn connect_song_navigation(song: &Song, window: &Window) -> Vec<glib::SignalHandlerId> {
    let handler1 = song.connect_closure(
        "artist-clicked",
        false,
        glib::closure_local!(
            #[weak]
            window,
            move |_song: Song, song_id: &str| {
                let library = window.get_application().library().clone();
                if let Some(artist_model) = artist_for_item(song_id, &library.borrow()) {
                    window.show_artist_detail(&artist_model);
                }
            }
        ),
    );

    let handler2 = song.connect_closure(
        "album-clicked",
        false,
        glib::closure_local!(
            #[weak]
            window,
            move |_song: Song, song_id: &str| {
                let library = window.get_application().library().clone();
                if let Some(album_model) = album_for_item(song_id, &library.borrow()) {
                    window.show_album_detail(&album_model);
                }
            }
        ),
    );

    vec![handler1, handler2]
}

pub fn connect_playing_indicator(
    song_widget: &Song,
    song_model: &SongModel,
    audio_model: &AudioModel,
) {
    // Set initial playing indicator state
    let current_track = audio_model.current_song_id();
    song_widget.set_playing(song_model.id() == current_track);

    let handler_id = audio_model.connect_closure(
        "song-changed",
        false,
        glib::closure_local!(
            #[weak]
            song_widget,
            #[weak]
            song_model,
            move |_: AudioModel, song_id: &str| {
                song_widget.set_playing(song_id == song_model.id());
            }
        ),
    );

    song_widget
        .imp()
        .playing_indicator_handler
        .replace(Some(handler_id));
}

pub fn disconnect_playing_indicator(song_widget: &Song, audio_model: &AudioModel) {
    if let Some(handler_id) = song_widget.imp().playing_indicator_handler.take() {
        audio_model.disconnect(handler_id);
    }
}

pub fn disconnect_signal_handlers(song_widget: &Song) {
    song_widget
        .imp()
        .signal_handlers
        .take()
        .into_iter()
        .for_each(|handler_id| song_widget.disconnect(handler_id));
}
