use glib::Object;
use gtk::{gio, glib, subclass::prelude::*};

use crate::jellyfin::api::MusicDto;

glib::wrapper! {
    pub struct Song(ObjectSubclass<imp::Song>)
    @extends gtk::Widget, gtk::ListBoxRow,
        @implements gio::ActionMap, gio::ActionGroup;
}

impl Song {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn set_song_data(&self, song: &MusicDto) {
        let imp = self.imp();
        imp.item_id.replace(Some(song.id.clone()));
        imp.title_label.set_label(&song.name);
        imp.number_label.set_label(&song.index_number.to_string());
        imp.duration_label
            .set_label(&format_duration(song.run_time_ticks));
    }
}

impl Default for Song {
    fn default() -> Self {
        Self::new()
    }
}

fn format_duration(ticks: u64) -> String {
    // Jellyfin ticks are in 100-nanosecond intervals
    // 1 second = 10,000,000 ticks
    let seconds = ticks / 10_000_000;
    let minutes = seconds / 60;
    let remaining_seconds = seconds % 60;
    format!("{}:{:02}", minutes, remaining_seconds)
}

mod imp {
    use std::cell::RefCell;

    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{CompositeTemplate, glib};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/song.ui")]
    pub struct Song {
        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub number_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub duration_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub play_button: TemplateChild<gtk::Button>,

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
    impl ObjectImpl for Song {}
    impl ListBoxRowImpl for Song {}
    impl WidgetImpl for Song {}
}
