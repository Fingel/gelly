use crate::{
    audio::{model::AudioModel, stream_info::discover_stream_info},
    library_utils::{album_for_item, artist_for_item},
    ui::{
        album_art::AlbumArt, lyrics::Lyrics, stream_info_dialog, volume_button::VolumeButton,
        widget_ext::WidgetApplicationExt,
    },
};
use adw::prelude::*;
use glib::{WeakRef, object::ObjectSubclassIs, subclass::prelude::*};
use gtk::glib;
use log::warn;
use std::cell::RefCell;

pub fn format_time(seconds: u32) -> String {
    let minutes = seconds / 60;
    let seconds = seconds % 60;
    format!("{}:{:02}", minutes, seconds)
}

pub trait PlayerImp: ObjectSubclassExt + glib::clone::Downgrade + 'static
where
    Self::Type: IsA<gtk::Widget>,
    Self::Type: ObjectSubclassIs<Subclass = Self>,
{
    // state
    fn audio_model(&self) -> &AudioModel;
    fn lyrics_window(&self) -> &RefCell<Option<WeakRef<adw::Window>>>;
    fn seek_debounce_id(&self) -> &RefCell<Option<glib::SourceId>>;
    fn position_storage(&self) -> &RefCell<u32>;
    fn duration_storage(&self) -> &RefCell<u32>;

    // widgets
    fn play_pause_button(&self) -> &gtk::Button;
    fn next_button(&self) -> &gtk::Button;
    fn prev_button(&self) -> &gtk::Button;
    fn volume_control(&self) -> &VolumeButton;
    fn info_button(&self) -> &gtk::Button;
    fn mute_button(&self) -> &gtk::Button {
        self.volume_control().mute_button()
    }
    fn volume_scale(&self) -> &gtk::Scale {
        self.volume_control().scale()
    }
    fn position_scale(&self) -> &gtk::Scale;
    fn position_label(&self) -> &gtk::Label;
    fn duration_label(&self) -> &gtk::Label;
    fn lyrics_button(&self) -> &gtk::Button;
    fn artist_button(&self) -> &gtk::Button;
    fn album_button(&self) -> &gtk::Button;
    fn title_label(&self) -> &gtk::Label;
    fn artist_label(&self) -> &gtk::Label;
    fn album_label(&self) -> &gtk::Label;
    fn album_art(&self) -> &AlbumArt;

    fn update_play_pause_button(&self, playing: bool) {
        let btn = self.play_pause_button();
        if playing {
            btn.set_icon_name("media-playback-pause-symbolic");
            btn.set_tooltip_text(Some("Pause"));
        } else {
            btn.set_icon_name("media-playback-start-symbolic");
            btn.set_tooltip_text(Some("Play"));
        }
    }

    fn update_song_info(&self) {
        let audio_model = self.audio_model();
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
        if let Some(song) = audio_model.current_song() {
            self.toggle_lyrics(song.has_lyrics());
            self.load_album_art(&song.album_id(), &song.id());
        }
    }

    fn toggle_lyrics(&self, has_lyrics: bool) {
        self.lyrics_button().set_visible(has_lyrics);
    }

    fn load_album_art(&self, album_id: &str, song_id: &str) {
        self.album_art().set_item_id(song_id, Some(album_id));
    }

    // These functions are called after the common position and duration updates,
    // allowing subclasses to do additional updates or logic.
    fn extra_position_update(&self, _position: u32) {}
    fn extra_duration_update(&self, _duration: u32) {}

    fn update_volume_icon(&self, volume: f64) {
        let icon_name = if volume == 0.0 {
            "audio-volume-muted-symbolic"
        } else if volume < 0.33 {
            "audio-volume-low-symbolic"
        } else if volume < 0.66 {
            "audio-volume-medium-symbolic"
        } else {
            "audio-volume-high-symbolic"
        };
        self.volume_control().set_icon_name(icon_name);
    }

    fn show_info_dialog(&self) {
        if let Some(uri) = self.audio_model().get_uri()
            && let Some(song_model) = self.audio_model().current_song()
        {
            let song_id = song_model.id();
            let jellyfin = self.obj().get_application().jellyfin();
            let weak = self.obj().downgrade();
            discover_stream_info(&uri, &song_id, &jellyfin, move |info| {
                if let Some(obj) = weak.upgrade() {
                    stream_info_dialog::show(obj.get_gtk_window().as_ref(), info);
                }
            });
        } else {
            warn!("Could not get current stream URI");
        }
    }

    fn show_lyrics(&self) {
        if let Some(window) = self
            .lyrics_window()
            .borrow()
            .as_ref()
            .and_then(|w| w.upgrade())
        {
            window.present();
        } else {
            let lyrics_widget = Lyrics::new();
            let jellyfin = self.obj().get_application().jellyfin();
            lyrics_widget.set_jellyfin(&jellyfin);
            lyrics_widget.bind_to_audio_model(self.audio_model());

            let window = adw::Window::new();
            window.set_content(Some(&lyrics_widget));
            window.set_default_size(500, 600);

            if let Some(parent) = self.obj().get_gtk_window() {
                window.set_transient_for(Some(&parent));
            }

            let weak = self.obj().downgrade();
            window.connect_close_request(move |_| {
                if let Some(obj) = weak.upgrade() {
                    obj.imp().lyrics_window().replace(None);
                }
                glib::Propagation::Proceed
            });

            self.lyrics_window()
                .replace(Some(ObjectExt::downgrade(&window)));
            window.present();

            let item_id = self.audio_model().current_song_id();
            lyrics_widget.fetch_lyrics(&item_id);
        }
    }

    fn show_artist(&self) {
        let song_id = self.audio_model().current_song_id();
        let window = self.obj().get_root_window();
        let library = self.obj().get_application().library().clone();
        if let Some(artist_model) = artist_for_item(&song_id, &library.borrow()) {
            window.show_artist_detail(&artist_model);
        }
    }

    fn show_album(&self) {
        let song_id = self.audio_model().current_song_id();
        let window = self.obj().get_root_window();
        let library = self.obj().get_application().library().clone();
        if let Some(album_model) = album_for_item(&song_id, &library.borrow()) {
            window.show_album_detail(&album_model);
        }
    }

    fn setup_clickable_labels(&self) {
        self.artist_button().set_cursor_from_name(Some("pointer"));
        self.album_button().set_cursor_from_name(Some("pointer"));

        let weak = self.obj().downgrade();
        self.artist_button().connect_clicked({
            let weak = weak.clone();
            move |_| {
                if let Some(obj) = weak.upgrade() {
                    obj.imp().show_artist();
                }
            }
        });
        self.album_button().connect_clicked({
            let weak = weak.clone();
            move |_| {
                if let Some(obj) = weak.upgrade() {
                    obj.imp().show_album();
                }
            }
        });
    }

    fn setup_common_signals(&self) {
        let weak = self.obj().downgrade();

        self.play_pause_button().connect_clicked({
            let weak = weak.clone();
            move |_| {
                if let Some(obj) = weak.upgrade() {
                    obj.imp().audio_model().toggle_play_pause();
                }
            }
        });

        self.next_button().connect_clicked({
            let weak = weak.clone();
            move |_| {
                if let Some(obj) = weak.upgrade() {
                    obj.imp().audio_model().next();
                }
            }
        });

        self.prev_button().connect_clicked({
            let weak = weak.clone();
            move |_| {
                if let Some(obj) = weak.upgrade() {
                    obj.imp().audio_model().prev();
                }
            }
        });

        self.mute_button().connect_clicked({
            let weak = weak.clone();
            move |_| {
                if let Some(obj) = weak.upgrade() {
                    let imp = obj.imp();
                    imp.volume_scale().set_value(0.0);
                    imp.update_volume_icon(0.0);
                }
            }
        });

        self.info_button().connect_clicked({
            let weak = weak.clone();
            move |_| {
                if let Some(obj) = weak.upgrade() {
                    obj.imp().show_info_dialog();
                }
            }
        });

        self.position_scale().connect_change_value({
            let weak = weak.clone();
            move |_, _, value| {
                let Some(obj) = weak.upgrade() else {
                    return glib::Propagation::Proceed;
                };
                let imp = obj.imp();
                if let Some(source_id) = imp.seek_debounce_id().take() {
                    source_id.remove();
                }
                let position = value as u32;
                imp.position_label().set_text(&format_time(position));

                let source_id = glib::timeout_add_local(std::time::Duration::from_millis(150), {
                    let weak = weak.clone();
                    move || {
                        if let Some(obj) = weak.upgrade() {
                            let imp = obj.imp();
                            imp.audio_model().seek(position);
                            imp.seek_debounce_id().replace(None);
                        }
                        glib::ControlFlow::Break
                    }
                });
                imp.seek_debounce_id().replace(Some(source_id));
                glib::Propagation::Proceed
            }
        });

        self.volume_scale().connect_value_changed({
            let weak = weak.clone();
            move |scale| {
                if let Some(obj) = weak.upgrade() {
                    obj.imp().audio_model().imp().set_volume(scale.value());
                    obj.imp().update_volume_icon(scale.value());
                }
            }
        });

        self.lyrics_button().connect_clicked({
            let weak = weak.clone();
            move |_| {
                if let Some(obj) = weak.upgrade() {
                    obj.imp().show_lyrics();
                }
            }
        });

        self.obj().connect_notify_local(Some("position"), {
            let weak = weak.clone();
            move |_, _| {
                if let Some(obj) = weak.upgrade() {
                    let imp = obj.imp();
                    let position = *imp.position_storage().borrow();
                    let duration = *imp.duration_storage().borrow();
                    imp.position_label().set_text(&format_time(position));
                    imp.extra_position_update(position);
                    if duration > 0 {
                        imp.position_scale().set_value(position as f64);
                    }
                }
            }
        });

        self.obj().connect_notify_local(Some("duration"), {
            let weak = weak.clone();
            move |_, _| {
                if let Some(obj) = weak.upgrade() {
                    let imp = obj.imp();
                    let duration = *imp.duration_storage().borrow();
                    imp.duration_label().set_text(&format_time(duration));
                    imp.extra_duration_update(duration);
                    if duration > 0 {
                        imp.position_scale().adjustment().set_upper(duration as f64);
                    }
                }
            }
        });
    }
}
