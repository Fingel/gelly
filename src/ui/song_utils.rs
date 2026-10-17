use crate::{
    Application,
    audio::model::AudioModel,
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
                if let Some(artist_model) =
                    window.get_application().library().artist_for_item(song_id)
                {
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
                if let Some(album_model) =
                    window.get_application().library().album_for_item(song_id)
                {
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

pub fn connect_favorite_indicator(song_widget: &Song, song_model: &SongModel, app: &Application) {
    song_widget.set_starred(song_model.favorite());

    let handler_id = app.connect_closure(
        "favorites-updated",
        false,
        glib::closure_local!(
            #[weak]
            song_widget,
            #[weak]
            song_model,
            move |app: Application| {
                let is_fav = app.library().song_is_favorite(&song_model.id());
                song_model.set_favorite(is_fav);
                song_widget.set_starred(is_fav);
            }
        ),
    );

    song_widget
        .imp()
        .favorite_indicator_handler
        .replace(Some((handler_id, app.downgrade())));
}

pub fn disconnect_favorite_indicator(song_widget: &Song) {
    if let Some((handler_id, app_weak)) = song_widget.imp().favorite_indicator_handler.take()
        && let Some(app) = app_weak.upgrade()
    {
        app.disconnect(handler_id);
    }
}

fn update_downloaded_indicator(song_widget: &Song, song_model: &SongModel, app: &Application) {
    let downloaded = app
        .media_cache()
        .is_some_and(|cache| cache.contains(&song_model.id()));
    song_widget.set_downloaded(downloaded);
}

pub fn connect_downloaded_indicator(song_widget: &Song, song_model: &SongModel, app: &Application) {
    disconnect_downloaded_indicator(song_widget);

    let init_handler = app.connect_closure(
        "media-cache-initialized",
        false,
        glib::closure_local!(
            #[weak]
            song_widget,
            #[weak]
            song_model,
            move |app: Application| {
                update_downloaded_indicator(&song_widget, &song_model, &app);
            }
        ),
    );

    let download_handler = app.connect_closure(
        "media-downloaded",
        false,
        glib::closure_local!(
            #[weak]
            song_widget,
            #[weak]
            song_model,
            move |app: Application, item_id: &str| {
                if item_id == song_model.id() {
                    update_downloaded_indicator(&song_widget, &song_model, &app);
                }
            }
        ),
    );

    song_widget
        .imp()
        .downloaded_indicator_handlers
        .replace(Some(([init_handler, download_handler], app.downgrade())));

    update_downloaded_indicator(song_widget, song_model, app);
}

pub fn disconnect_downloaded_indicator(song_widget: &Song) {
    if let Some((handlers, app_weak)) = song_widget.imp().downloaded_indicator_handlers.take()
        && let Some(app) = app_weak.upgrade()
    {
        for handler in handlers {
            app.disconnect(handler);
        }
    }

    song_widget.set_downloaded(false);
}

pub fn disconnect_signal_handlers(song_widget: &Song) {
    song_widget
        .imp()
        .signal_handlers
        .take()
        .into_iter()
        .for_each(|handler_id| song_widget.disconnect(handler_id));
}
