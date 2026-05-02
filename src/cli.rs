use gtk::{
    glib::{self, Char},
    prelude::*,
};

use crate::{
    application::Application,
    library_utils::{play_album, play_artist, play_song},
};

pub fn add_cli_options(app: &Application) {
    app.add_main_option(
        "next",
        Char::from(b'n'),
        glib::OptionFlags::NONE,
        glib::OptionArg::None,
        "Skip to the next track",
        None,
    );
    app.add_main_option(
        "prev",
        Char::from(b'p'),
        glib::OptionFlags::NONE,
        glib::OptionArg::None,
        "Go to the previous track",
        None,
    );
    app.add_main_option(
        "play-pause",
        Char::from(b't'),
        glib::OptionFlags::NONE,
        glib::OptionArg::None,
        "Toggle play/pause",
        None,
    );
    app.add_main_option(
        "stop",
        Char::from(b's'),
        glib::OptionFlags::NONE,
        glib::OptionArg::None,
        "Stop playback",
        None,
    );
    app.add_main_option(
        "play-album",
        Char::from(b'\0'),
        glib::OptionFlags::NONE,
        glib::OptionArg::String,
        "Play an album by ID",
        Some("ALBUM_ID"),
    );
    app.add_main_option(
        "play-artist",
        Char::from(b'\0'),
        glib::OptionFlags::NONE,
        glib::OptionArg::String,
        "Play all songs by an artist ID",
        Some("ARTIST_ID"),
    );
    app.add_main_option(
        "play-song",
        Char::from(b'\0'),
        glib::OptionFlags::NONE,
        glib::OptionArg::String,
        "Play a specific song by ID",
        Some("SONG_ID"),
    );

    app.connect_command_line(|app, command_line| {
        let options = command_line.options_dict();

        // In application::new() we set HANDLES_COMMAND_LINE ApplicationFlag
        // which suppresses the automatic activate signal
        // check if this is the first instance and launch normally if so
        if !command_line.is_remote() {
            app.activate();
            return glib::ExitCode::SUCCESS;
        }

        let Some(audio_model) = app.audio_model() else {
            log::warn!("No audio model found, cannot handle CLI commands");
            return glib::ExitCode::FAILURE;
        };

        let lookup_bool = |name: &str| options.lookup::<bool>(name).ok().flatten().unwrap_or(false);
        let lookup_value = |name: &str| {
            options
                .lookup_value(name, None)
                .and_then(|v| v.str().map(|s| Some(s.to_string())))
                .flatten()
        };

        if lookup_bool("next") {
            audio_model.next();
        } else if lookup_bool("prev") {
            audio_model.prev();
        } else if lookup_bool("play-pause") {
            audio_model.toggle_play_pause();
        } else if lookup_bool("stop") {
            audio_model.stop();
        }

        if let Some(song_id) = lookup_value("play-song") {
            play_song(&song_id, app);
        } else if let Some(album_id) = lookup_value("play-album") {
            play_album(&album_id, app);
        } else if let Some(artist_id) = lookup_value("play-artist") {
            play_artist(&artist_id, app);
        }

        glib::ExitCode::SUCCESS
    });
}
