use glib::Object;
use gtk::{gio, glib, prelude::*, subclass::prelude::*};

use crate::{
    audio::model::AudioModel, jellyfin::utils::format_duration, models::SongModel,
    ui::widget_ext::WidgetApplicationExt,
};

glib::wrapper! {
    pub struct Song(ObjectSubclass<imp::Song>)
    @extends gtk::Widget, gtk::ListBoxRow,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Actionable;
}

impl Song {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn set_song_data(&self, song: &SongModel) {
        let imp = self.imp();
        imp.item_id.replace(Some(song.id()));
        imp.title_label.set_label(&song.title());
        imp.album_label.set_label(&song.album());
        imp.artist_label.set_label(&song.artists_string());
        imp.number_label.set_label(&song.track_number().to_string());
        imp.duration_label
            .set_label(&format_duration(song.duration()));
    }

    pub fn set_playing(&self, playing: bool) {
        self.imp().playing_icon.set_visible(playing);
    }

    pub fn show_details(&self) {
        let imp = self.imp();
        imp.artist_label.set_visible(true);
        imp.album_label.set_visible(true);
        imp.drag_handle_box.set_visible(true);
        imp.song_actions_box.set_visible(true);
    }

    fn listen_for_song_changes(&self) {
        if let Some(audio_model) = self.get_application().audio_model() {
            audio_model.connect_closure(
                "song-changed",
                false,
                glib::closure_local!(
                    #[weak(rename_to = song)]
                    self,
                    move |_audio_model: AudioModel, song_id: &str| {
                        let my_id = song.imp().item_id.borrow().clone().unwrap_or_default();
                        song.set_playing(song_id == my_id);
                    }
                ),
            );
        }
    }
}

impl Default for Song {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use std::cell::RefCell;

    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{CompositeTemplate, glib, prelude::*};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/song.ui")]
    pub struct Song {
        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub number_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub artist_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub album_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub duration_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub playing_icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub drag_handle_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub song_actions_box: TemplateChild<gtk::Box>,

        pub item_id: RefCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Song {
        const NAME: &'static str = "GellySong";
        type Type = super::Song;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }
    impl ObjectImpl for Song {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().connect_map(glib::clone!(
                #[weak(rename_to = song)]
                self.obj(),
                move |_| {
                    song.listen_for_song_changes();
                }
            ));
        }
    }
    impl ListBoxRowImpl for Song {}
    impl WidgetImpl for Song {}
}
