use crate::{
    async_utils::spawn_tokio,
    audio::{model::AudioModel, stream_info::discover_stream_info},
    jellyfin::api::ItemType,
    ui::{
        album_art::AlbumArt, album_art_background::draw_background, lyrics::Lyrics,
        stream_info_dialog, widget_ext::WidgetApplicationExt,
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
    fn favorite_binding(&self) -> &RefCell<Option<glib::Binding>>;

    // widgets
    fn play_pause_button(&self) -> &gtk::Button;
    fn next_button(&self) -> &gtk::Button;
    fn prev_button(&self) -> &gtk::Button;
    fn volume_control(&self) -> &gtk::ScaleButton;
    fn info_button(&self) -> &gtk::Button;
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
    fn favorite_button(&self) -> &gtk::ToggleButton;

    fn snapshot_background(&self, snapshot: &gtk::Snapshot) {
        let obj = self.obj();
        let root = obj.get_root_window();
        if let Some(p) = root.blurred_paintable() {
            let root_w = root.width();
            let root_h = root.height();
            draw_background(
                snapshot,
                &p,
                root_w as f64,
                root_h as f64,
                Some((
                    (obj.width() - root_w) as f32,
                    (obj.height() - root_h) as f32,
                )),
            );
        }
    }

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
        self.update_favorite_binding();
    }

    fn update_favorite_binding(&self) {
        if let Some(song) = self.audio_model().current_song() {
            let binding = song
                .bind_property("favorite", self.favorite_button(), "active")
                .sync_create()
                .build();
            self.favorite_binding().replace(Some(binding));
        } else {
            self.favorite_binding().replace(None);
            self.favorite_button().set_active(false);
            self.favorite_button().set_icon_name("non-starred-symbolic");
        }
    }

    fn toggle_favorite(&self, is_favorite: bool) {
        let Some(song) = self.audio_model().current_song() else {
            return;
        };
        song.set_favorite(is_favorite);
        let item_id = song.id();
        let app = self.obj().get_application();
        let backend = app.jellyfin();
        let weak = self.obj().downgrade();
        spawn_tokio(
            async move { backend.set_favorite(&item_id, &ItemType::Audio, is_favorite).await },
            move |result| {
                let Some(obj) = weak.upgrade() else { return };
                let imp = obj.imp();
                if let Err(err) = result {
                    warn!("Failed to set favorite: {err}");
                    if let Some(song) = imp.audio_model().current_song() {
                        song.set_favorite(!is_favorite);
                    }
                } else {
                    if let Some(song) = imp.audio_model().current_song() {
                        song.set_favorite(is_favorite);
                    }
                    obj.get_application().refresh_favorites(true);
                }
            },
        );
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
        if let Some(artist_model) = self
            .obj()
            .get_application()
            .library()
            .artist_for_item(&song_id)
        {
            window.show_artist_detail(&artist_model);
        }
    }

    fn show_album(&self) {
        let song_id = self.audio_model().current_song_id();
        let window = self.obj().get_root_window();
        if let Some(album_model) = self
            .obj()
            .get_application()
            .library()
            .album_for_item(&song_id)
        {
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

    fn setup_volume_icons(&self) {
        self.volume_control()
            .minus_button()
            .set_icon_name("audio-volume-low-symbolic");
        self.volume_control()
            .plus_button()
            .set_icon_name("audio-volume-high-symbolic");
        self.volume_control().set_icons(&[
            "audio-volume-muted-symbolic",
            "audio-volume-high-symbolic",
            "audio-volume-low-symbolic",
            "audio-volume-medium-symbolic",
        ]);
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

        let middle_click = gtk::GestureClick::new();
        middle_click.set_button(gtk::gdk::BUTTON_MIDDLE);
        middle_click.connect_pressed({
            let weak = weak.clone();
            move |_, _, _, _| {
                if let Some(obj) = weak.upgrade() {
                    let vol = obj.imp().volume_control();
                    vol.set_value(if vol.value() == 0.0 { 1.0 } else { 0.0 });
                }
            }
        });
        self.volume_control().add_controller(middle_click);

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
                            imp.position_scale()
                                .set_value(imp.audio_model().position() as f64);
                            imp.seek_debounce_id().replace(None);
                        }
                        glib::ControlFlow::Break
                    }
                });
                imp.seek_debounce_id().replace(Some(source_id));
                glib::Propagation::Proceed
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

        self.favorite_button().connect_notify_local(
            Some("active"),
            move |btn, _| {
                btn.set_icon_name(if btn.is_active() {
                    "starred-symbolic"
                } else {
                    "non-starred-symbolic"
                });
            },
        );

        self.favorite_button().connect_clicked({
            let weak = weak.clone();
            move |btn| {
                if let Some(obj) = weak.upgrade() {
                    obj.imp().toggle_favorite(btn.is_active());
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
