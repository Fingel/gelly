use crate::{
    application::Application,
    library_utils::albums_from_library,
    models::AlbumModel,
    ui::{album::Album, list_helpers::*, widget_ext::WidgetApplicationExt, window::Window},
};
use glib::Object;
use gtk::{
    gio,
    glib::{self},
    prelude::*,
    subclass::prelude::*,
};

glib::wrapper! {
    pub struct AlbumList(ObjectSubclass<imp::AlbumList>)
    @extends gtk::Widget, gtk::Box,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

#[derive(Debug, Clone, Copy)]
enum SortOption {
    AlbumName,
    AlbumArtist,
    DateAdded,
}

impl AlbumList {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn pull_albums(&self) {
        let library = self.get_application().library().clone();
        let albums = albums_from_library(&library.borrow());
        if albums.is_empty() {
            self.set_empty(true);
        } else {
            self.set_empty(false);
            let store = self
                .imp()
                .store
                .get()
                .expect("AlbumList store should be initialized.");
            store.remove_all();
            for album in albums {
                store.append(&album);
            }
        }
    }

    pub fn activate_album(&self, index: u32) {
        let window = self.get_root_window();
        handle_grid_activation::<AlbumModel, _>(&self.imp().grid_view, index, |album_model| {
            window.show_album_detail(album_model);
        });
    }

    pub fn setup_library_connection(&self) {
        let app = self.get_application();
        app.connect_closure(
            "library-refreshed",
            false,
            glib::closure_local!(
                #[weak(rename_to = album_list)]
                self,
                move |_app: Application, _total_record_count: u64| {
                    album_list.pull_albums();
                }
            ),
        );
    }

    pub fn search_changed(&self, query: &str) {
        let imp = self.imp();
        let store = imp.store.get().expect("Store should be initialized");

        let filters = vec![
            imp.name_filter
                .get()
                .expect("Name filter should be initialized")
                .clone(),
            imp.artists_filter
                .get()
                .expect("Artists filter should be initialized")
                .clone(),
        ];
        apply_multi_filter_search(query, store, &filters, &imp.grid_view);
    }

    fn sort_changed(&self, sort: SortOption) {
        dbg!(sort);
    }

    pub fn setup_search_sort_connection(&self) {
        let window = self.get_root_window();

        window.connect_closure(
            "search",
            false,
            glib::closure_local!(
                #[weak(rename_to = album_list)]
                self,
                move |_: Window| {
                    album_list.imp().search_bar.set_search_mode(true);
                    album_list.imp().sort_bar.set_search_mode(false);
                }
            ),
        );

        window.connect_closure(
            "sort",
            false,
            glib::closure_local!(
                #[weak(rename_to = album_list)]
                self,
                move |_: Window| {
                    album_list.imp().sort_bar.set_search_mode(true);
                    album_list.imp().search_bar.set_search_mode(false);
                }
            ),
        );
    }

    fn setup_model(&self) {
        let imp = self.imp();
        let store = gio::ListStore::new::<AlbumModel>();
        let name_filter = create_string_filter::<AlbumModel>("name");
        let artists_filter = create_string_filter::<AlbumModel>("artists-string");
        imp.store
            .set(store.clone())
            .expect("Store should only be set once");
        imp.name_filter
            .set(name_filter)
            .expect("Name filter should only be set once");
        imp.artists_filter
            .set(artists_filter)
            .expect("Artists filter should only be set once");

        let selection_model = gtk::SingleSelection::new(Some(store));
        let factory = gtk::SignalListItemFactory::new();

        factory.connect_setup(move |_, list_item| {
            let placeholder = Album::new();
            let item = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be a ListItem");
            item.set_child(Some(&placeholder))
        });

        factory.connect_bind(move |_, list_item| {
            let list_item = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be a ListItem");
            let album_model = list_item
                .item()
                .and_downcast::<AlbumModel>()
                .expect("Item should be an AlbumData");
            let album_widget = list_item
                .child()
                .and_downcast::<Album>()
                .expect("Child has to be an Album");

            album_widget.set_album_model(&album_model);
        });

        imp.grid_view.set_model(Some(&selection_model));
        imp.grid_view.set_factory(Some(&factory));
    }

    fn set_empty(&self, empty: bool) {
        self.imp().empty.set_visible(empty);
        self.imp().grid_view.set_visible(!empty);
    }
}

impl Default for AlbumList {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {

    use std::cell::OnceCell;

    use super::SortOption;
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{CompositeTemplate, gio, glib, prelude::*};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/album_list.ui")]
    pub struct AlbumList {
        #[template_child]
        pub grid_view: TemplateChild<gtk::GridView>,
        #[template_child]
        pub empty: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub search_bar: TemplateChild<gtk::SearchBar>,
        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub sort_bar: TemplateChild<gtk::SearchBar>,
        #[template_child]
        pub sort_dropdown: TemplateChild<gtk::DropDown>,

        pub store: OnceCell<gio::ListStore>,
        pub name_filter: OnceCell<gtk::StringFilter>,
        pub artists_filter: OnceCell<gtk::StringFilter>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AlbumList {
        const NAME: &'static str = "GellyAlbumList";
        type Type = super::AlbumList;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AlbumList {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_model();

            self.grid_view.connect_activate(glib::clone!(
                #[weak(rename_to = album_list)]
                self.obj(),
                move |_, position| {
                    album_list.activate_album(position);
                }
            ));

            self.search_entry.connect_search_changed(glib::clone!(
                #[weak(rename_to = album_list)]
                self.obj(),
                move |entry| {
                    album_list.search_changed(&entry.text());
                }
            ));

            self.sort_dropdown.connect_selected_notify(glib::clone!(
                #[weak(rename_to = album_list)]
                self.obj(),
                move |drop_down| {
                    let sort_option = match drop_down.selected() {
                        0 => SortOption::AlbumName,
                        1 => SortOption::AlbumArtist,
                        2 => SortOption::DateAdded,
                        _ => SortOption::AlbumName,
                    };
                    album_list.sort_changed(sort_option);
                }
            ));

            self.obj().connect_realize(glib::clone!(
                #[weak (rename_to = album_list)]
                self.obj(),
                move |_| {
                    album_list.setup_search_sort_connection();
                }
            ));
        }
    }

    impl WidgetImpl for AlbumList {}
    impl BoxImpl for AlbumList {}
}
