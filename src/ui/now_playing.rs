use crate::ui::{song::Song, widget_ext::WidgetApplicationExt};
use glib::Object;
use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use log::warn;

glib::wrapper! {
    pub struct NowPlaying(ObjectSubclass<imp::NowPlaying>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup;
}

impl NowPlaying {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn display_playlist(&self) {
        if let Some(audio_model) = self.get_application().audio_model() {
            let tracks = audio_model.playlist();
            self.imp().track_list.remove_all();
            if tracks.is_empty() {
                self.set_empty(true);
            } else {
                self.set_empty(false);
                let current_track = audio_model.current_song_id();
                for track in &tracks {
                    let song_widget = Song::new();
                    song_widget.set_song_data(track);
                    self.imp().track_list.append(&song_widget);
                    if track.id() == current_track {
                        song_widget.set_playing(true);
                    }
                }
            }
        } else {
            self.toast("Audio model not initialized, please restart", None);
            warn!("No audio model found");
        }
    }

    fn set_empty(&self, empty: bool) {
        self.imp().empty.set_visible(empty);
        self.imp().track_list.set_visible(!empty);
    }

    pub fn song_selected(&self, index: usize) {
        if let Some(audio_model) = self.get_application().audio_model() {
            audio_model.play_song(index);
        }
    }
}

impl Default for NowPlaying {
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
        prelude::*,
    };

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/now_playing.ui")]
    pub struct NowPlaying {
        #[template_child]
        pub empty: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub track_list: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NowPlaying {
        const NAME: &'static str = "GellyNowPlaying";
        type Type = super::NowPlaying;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl BoxImpl for NowPlaying {}
    impl WidgetImpl for NowPlaying {}
    impl ObjectImpl for NowPlaying {
        fn constructed(&self) {
            self.parent_constructed();
            self.setup_signals();
            self.obj().connect_map(glib::clone!(
                #[weak(rename_to = now_playing)]
                self.obj(),
                move |_| {
                    now_playing.display_playlist();
                }
            ));
        }
    }

    impl NowPlaying {
        fn setup_signals(&self) {
            self.track_list.connect_row_activated(glib::clone!(
                #[weak(rename_to=imp)]
                self,
                move |_track_list, row| {
                    let index = row.index();
                    imp.obj().song_selected(index as usize);
                }
            ));
        }
    }
}
