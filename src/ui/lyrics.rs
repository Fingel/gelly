use glib::Object;
use gtk::{gdk, gio, glib, prelude::*, subclass::prelude::*};
use log::debug;

use crate::{
    async_utils::spawn_tokio, audio::model::AudioModel, backend::Backend, jellyfin::api::Lyric,
};

glib::wrapper! {
    pub struct Lyrics(ObjectSubclass<imp::Lyrics>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

// Since we only have seconds precision on audio position, but lyrics are synced to ms,
// we add a small fudge because being slightly early feels better than being slightly late.
const LYRICS_FUDGE: u64 = 10_000_000;

impl Lyrics {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn set_jellyfin(&self, backend: &Backend) {
        let imp = self.imp();
        if let Err(e) = imp.jellyfin.set(backend.clone()) {
            debug!("Backend client already set: {:?}", e);
        }
    }

    pub fn bind_to_audio_model(&self, audio_model: &AudioModel) {
        let imp = self.imp();

        if let Err(e) = imp.audio_model.set(audio_model.clone()) {
            debug!("Audio model already set: {:?}", e);
        }

        audio_model.connect_notify_local(
            Some("queue-index"),
            glib::clone!(
                #[weak(rename_to = lyrics)]
                self,
                move |audio_model, _| {
                    if audio_model.queue_index() >= 0 {
                        let item_id = audio_model.current_song_id();
                        lyrics.set_song_info();
                        lyrics.fetch_lyrics(&item_id);
                    }
                }
            ),
        );

        audio_model.connect_notify_local(
            Some("position"),
            glib::clone!(
                #[weak(rename_to = lyrics)]
                self,
                move |audio_model, _| {
                    lyrics.update_lyrics_position(audio_model.position());
                }
            ),
        );
    }

    fn set_song_info(&self) {
        if let Some(audio_model) = self.imp().audio_model.get() {
            let title = audio_model.current_song_title();
            let artist = audio_model.current_song_artists().join(", ");
            self.imp().artist_label.set_text(&artist);
            self.imp().song_label.set_text(&title);
        }
    }

    pub fn fetch_lyrics(&self, item_id: &str) {
        let imp = self.imp();
        self.set_song_info();

        let Some(jellyfin) = imp.jellyfin.get() else {
            debug!("Jellyfin client not set, cannot fetch lyrics");
            return;
        };

        let item_id = item_id.to_string();
        let jellyfin = jellyfin.clone();

        spawn_tokio(
            async move { jellyfin.fetch_lyrics(&item_id).await },
            glib::clone!(
                #[weak (rename_to = obj)]
                self,
                move |result| {
                    match result {
                        Ok(lyrics_resp) => {
                            obj.create_lyrics_widgets(lyrics_resp.lyrics);
                            obj.update_lyrics_position(0u32);
                        }
                        Err(_) => obj.create_lyrics_widgets(vec![]),
                    }
                }
            ),
        );
    }

    fn create_lyrics_widgets(&self, lyrics: Vec<Lyric>) {
        let imp = self.imp();

        // Clear existing widgets
        while let Some(child) = imp.lyrics_box.first_child() {
            imp.lyrics_box.remove(&child);
        }

        // With Subsonic, we don't know if lyrics exist until we fetch them
        // so show this label if there are no lyrics.
        imp.lyrics_label_empty.set_visible(lyrics.is_empty());

        let mut labels = Vec::new();

        for lyric in lyrics.iter() {
            let label = gtk::Label::new(Some(&lyric.text));
            label.set_wrap(true);
            label.set_justify(gtk::Justification::Center);
            label.set_valign(gtk::Align::Start);
            label.set_halign(gtk::Align::Center);

            if let Some(ticks) = lyric.start {
                label.set_cursor(gdk::Cursor::from_name("pointer", None).as_ref());

                let gesture = gtk::GestureClick::new();
                gesture.connect_pressed(glib::clone!(
                    #[weak(rename_to = lyrics)]
                    self,
                    move |_, _, _, _| {
                        let seconds = (ticks / 10_000_000) as u32;
                        if let Some(audio_model) = lyrics.imp().audio_model.get() {
                            audio_model.seek(seconds);
                        }
                    }
                ));
                label.add_controller(gesture);
            }

            imp.lyrics_box.append(&label);
            labels.push(label);
        }

        imp.lyrics.replace(lyrics);
        imp.lyrics_labels.replace(labels);
    }

    fn update_lyrics_position(&self, position: u32) {
        let ticks = u64::from(position).saturating_mul(10_000_000); // Jellyfin ticks
        let lyrics = self.imp().lyrics.borrow();
        let labels = self.imp().lyrics_labels.borrow();

        // Find the index of the currently playing lyric (last lyric that has started)
        let current_lyric_index = lyrics.iter().rposition(|lyric| {
            lyric.start.unwrap_or(u64::MAX) <= ticks.saturating_add(LYRICS_FUDGE)
        });

        for (i, label) in labels.iter().enumerate() {
            let is_current = Some(i) == current_lyric_index;
            let is_past = current_lyric_index.is_some_and(|idx| i < idx);

            if is_current {
                label.add_css_class("current-lyric");
                label.remove_css_class("dimmed");
            } else {
                label.remove_css_class("current-lyric");

                if is_past {
                    label.add_css_class("dimmed");
                } else {
                    label.remove_css_class("dimmed");
                }
            }
        }
    }
}

impl Default for Lyrics {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib::{self},
    };

    use crate::{audio::model::AudioModel, backend::Backend, jellyfin::api::Lyric};
    use std::cell::{OnceCell, RefCell};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/lyrics.ui")]
    pub struct Lyrics {
        #[template_child]
        pub toolbar_view: TemplateChild<adw::ToolbarView>,
        #[template_child]
        pub song_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub artist_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub lyrics_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub lyrics_label_empty: TemplateChild<gtk::Label>,
        pub audio_model: OnceCell<AudioModel>,
        pub jellyfin: OnceCell<Backend>,
        pub lyrics: RefCell<Vec<Lyric>>,
        pub lyrics_labels: RefCell<Vec<gtk::Label>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Lyrics {
        const NAME: &'static str = "GellyLyrics";
        type Type = super::Lyrics;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Lyrics {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for Lyrics {}
    impl BoxImpl for Lyrics {}
}
