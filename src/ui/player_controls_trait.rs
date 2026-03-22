use crate::{audio::model::AudioModel, ui::album_art::AlbumArt};
use gtk::prelude::*;

pub trait PlayerControls {
    fn play_pause_btn(&self) -> &gtk::Button;
    fn title_label(&self) -> &gtk::Label;
    fn artist_label(&self) -> &gtk::Label;
    fn album_label(&self) -> &gtk::Label;
    fn lyrics_btn(&self) -> &gtk::Button;
    fn album_art(&self) -> &AlbumArt;
    fn audio_model(&self) -> &AudioModel;
    fn update_play_pause_button(&self, playing: bool) {
        let btn = self.play_pause_btn();
        if playing {
            btn.set_icon_name("media-playback-pause-symbolic");
            btn.set_tooltip_text(Some("Pause"));
        } else {
            btn.set_icon_name("media-playback-start-symbolic");
            btn.set_tooltip_text(Some("Play"));
        }
    }
    fn update_song_info(&self, audio_model: &AudioModel) {
        // Update title and artist
        let title = audio_model.current_song_title();
        let artists = audio_model.current_song_artists();
        let album = audio_model.current_song_album();
        let artist_str = if artists.is_empty() {
            "Unknown Artist".to_string()
        } else {
            artists.join(", ")
        };

        self.title_label().set_text(&title);
        self.artist_label().set_text(&artist_str);
        self.album_label().set_text(&album);

        // Load album art and lyrics if available
        if let Some(song) = audio_model.current_song() {
            self.toggle_lyrics(song.has_lyrics());
            self.load_album_art(&song.album_id(), &song.id());
        }
    }
    fn toggle_lyrics(&self, has_lyrics: bool) {
        self.lyrics_btn().set_visible(has_lyrics);
    }
    fn load_album_art(&self, album_id: &str, song_id: &str) {
        self.album_art().set_item_id(song_id, Some(album_id));
    }
    fn format_time(seconds: u32) -> String {
        let minutes = seconds / 60;
        let seconds = seconds % 60;
        format!("{}:{:02}", minutes, seconds)
    }
}
