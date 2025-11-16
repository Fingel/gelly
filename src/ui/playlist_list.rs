use crate::{
    application::Application,
    models::PlaylistModel,
    ui::{list_helpers::*, playlist::Playlist, widget_ext::WidgetApplicationExt, window::Window},
};
use glib::Object;
use gtk::{
    gio,
    glib::{self},
    prelude::*,
    subclass::prelude::*,
};

glib::wrapper! {
    pub struct PlaylistList(ObjectSubclass<imp::PlaylistList>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl PlaylistList {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn pull_playlists(&self) {
        let playlists = self.get_application().playlists().borrow().clone();
        if playlists.is_empty() {
            self.set_empty(true);
        } else {
            self.set_empty(false);
            let store = self
                .imp()
                .store
                .get()
                .expect("PlaylistList store should be initialized");
            store.remove_all();
            for playlist in playlists {
                let playlist_obj = PlaylistModel::from(&playlist);
                store.append(&playlist_obj);
            }
        }
    }

    pub fn activate_playlist(&self, index: u32) {
        let window = self.get_root_window();
        handle_grid_activation::<PlaylistModel, _>(
            &self.imp().grid_view,
            index,
            |playlist_model| {
                window.show_playlist_detail(playlist_model);
            },
        );
    }

    pub fn setup_library_connection(&self) {
        let app = self.get_application();
        app.connect_closure(
            "library-refreshed",
            false,
            glib::closure_local!(
                #[weak(rename_to = playlist_list)]
                self,
                move |_app: Application, _total_record_count: u64| {
                    playlist_list.pull_playlists();
                }
            ),
        );
    }

    pub fn search_changed(&self, query: &str) {
        let imp = self.imp();
        let store = imp.store.get().expect("Store should be initialized");
        let name_filter = imp
            .name_filter
            .get()
            .expect("Name filter should be initialized");
        apply_single_filter_search(query, store, name_filter, &imp.grid_view);
    }

    pub fn setup_search_connection(&self) {
        let window = self.get_root_window();
        window.connect_closure(
            "search",
            false,
            glib::closure_local!(
                #[weak(rename_to = playlist_list)]
                self,
                move |_: Window| {
                    playlist_list.imp().search_bar.set_search_mode(true);
                }
            ),
        );
    }

    fn setup_model(&self) {
        let imp = self.imp();
        let store = gio::ListStore::new::<PlaylistModel>();
        let name_filter = create_string_filter::<PlaylistModel>("name");
        imp.store
            .set(store.clone())
            .expect("PlaylistList store should only be set once");
        imp.name_filter
            .set(name_filter)
            .expect("PlaylistList name filter should only be set once");

        let selection_model = gtk::SingleSelection::new(Some(store));
        let factory = gtk::SignalListItemFactory::new();

        factory.connect_setup(move |_, list_item| {
            let placeholder = Playlist::new();
            let item = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be a ListItem");
            item.set_child(Some(&placeholder));
        });

        factory.connect_bind(move |_, list_item| {
            let list_item = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be a ListItem");
            let playlist_model = list_item
                .item()
                .and_downcast::<PlaylistModel>()
                .expect("Item should be a PlaylistModel");
            let playlist_widget = list_item
                .child()
                .and_downcast::<Playlist>()
                .expect("child should be a Playlist");
            playlist_widget.set_playlist_model(&playlist_model);
        });

        imp.grid_view.set_model(Some(&selection_model));
        imp.grid_view.set_factory(Some(&factory));
    }

    fn set_empty(&self, empty: bool) {
        self.imp().empty.set_visible(empty);
        self.imp().grid_view.set_visible(!empty);
    }
}

impl Default for PlaylistList {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use std::cell::OnceCell;

    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{CompositeTemplate, gio, glib, prelude::*};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/playlist_list.ui")]
    pub struct PlaylistList {
        #[template_child]
        pub grid_view: TemplateChild<gtk::GridView>,
        #[template_child]
        pub empty: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub search_bar: TemplateChild<gtk::SearchBar>,
        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,

        pub store: OnceCell<gio::ListStore>,
        pub name_filter: OnceCell<gtk::StringFilter>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlaylistList {
        const NAME: &'static str = "GellyPlaylistList";
        type Type = super::PlaylistList;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PlaylistList {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_model();

            self.grid_view.connect_activate(glib::clone!(
                #[weak(rename_to = playlist_list)]
                self.obj(),
                move |_, position| {
                    playlist_list.activate_playlist(position);
                }
            ));

            self.search_entry.connect_search_changed(glib::clone!(
                #[weak(rename_to = playlist_list)]
                self.obj(),
                move |entry| {
                    playlist_list.search_changed(&entry.text());
                }
            ));

            self.obj().connect_realize(glib::clone!(
                #[weak (rename_to = playlist_list)]
                self.obj(),
                move |_| {
                    playlist_list.setup_search_connection();
                }
            ));
        }
    }
    impl WidgetImpl for PlaylistList {}
    impl BoxImpl for PlaylistList {}
}
