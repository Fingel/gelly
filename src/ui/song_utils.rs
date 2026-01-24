use crate::{
    library_utils::{album_for_item, artist_for_item},
    ui::{song::Song, widget_ext::WidgetApplicationExt, window::Window},
};
use gtk::{glib, prelude::*};

/// Used for navigation on a song widget artist/album labels
pub fn connect_song_navigation(song: &Song, window: &Window) {
    song.connect_closure(
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

    song.connect_closure(
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
}
