use crate::config::settings;
use crate::models::{AlbumModel, ArtistModel, PlaylistModel};
use crate::ui::page_traits::{DetailPage, TopPage};
use crate::ui::preferences::Preferences;
use crate::ui::{about_dialog, shortcuts_dialog};
use crate::{application::Application, ui::widget_ext::WidgetApplicationExt};
use adw::{prelude::*, subclass::prelude::ObjectSubclassIsExt};
use glib::Object;
use gtk::{
    gio,
    glib::{self},
};
use log::error;

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
    @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
            @implements gio::ActionMap, gio::ActionGroup, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;

}

impl Window {
    pub fn blurred_paintable(&self) -> Option<gtk::gdk::Paintable> {
        self.imp().blurred_paintable.borrow().clone()
    }

    pub fn new(app: &Application) -> Self {
        let window: Self = Object::builder().property("application", app).build();
        if !app.setup_complete() {
            window.show_server_setup();
        } else {
            window.show_main_page();
        }
        window
    }

    pub fn toast(&self, title: &str, timeout: Option<u32>) {
        let mut toast = adw::Toast::builder().title(title);
        if let Some(timeout) = timeout {
            toast = toast.timeout(timeout);
        }
        self.imp().toaster.add_toast(toast.build());
    }

    pub fn go_back(&self) {
        self.imp().main_navigation.pop();
    }

    pub fn show_server_setup(&self) {
        let imp = self.imp();
        imp.setup_stack.set_visible_child(&imp.setup.get());
    }

    pub fn show_main_page(&self) {
        let imp = self.imp();
        imp.setup_stack.set_visible_child(&imp.bottom_sheet.get());
        imp.main_navigation.replace(&[imp.main_window.get()]);
        imp.album_list.setup_library_connection();
        imp.artist_list.setup_library_connection();
        imp.playlist_list.setup_library_connection();
        imp.song_list.setup_library_connection();
        // Library is refreshed down at the end of the connect_map signal

        // Initialize player bar with audio model
        if let Some(audio_model) = self.get_application().audio_model() {
            imp.player_bar
                .bind_to_audio_model(&audio_model, &imp.bottom_sheet);
            imp.big_player.bind_to_audio_model(&audio_model);
        }

        // Populate sort bar for the initially visible page
        self.show_visible_page();
    }

    fn show_visible_page(&self) {
        if let Some(visible_child) = self.imp().stack.visible_child() {
            let imp = self.imp();
            if visible_child == imp.album_list.get().upcast::<gtk::Widget>() {
                self.show_page(&imp.album_list.get());
            } else if visible_child == imp.artist_list.get().upcast::<gtk::Widget>() {
                self.show_page(&imp.artist_list.get());
            } else if visible_child == imp.playlist_list.get().upcast::<gtk::Widget>() {
                self.show_page(&imp.playlist_list.get());
            } else if visible_child == imp.song_list.get().upcast::<gtk::Widget>() {
                self.show_page(&imp.song_list.get());
            } else {
                error!("Unknown page widget");
            }
        }
    }

    pub fn show_page<T>(&self, page: &T)
    where
        T: IsA<gtk::Widget>,
        T: TopPage,
    {
        let imp = self.imp();
        imp.main_navigation.replace(&[imp.main_window.get()]);
        imp.new_button.set_visible(page.can_new());
        let sort_model = gtk::StringList::new(
            &page
                .sort_options()
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>(),
        );
        imp.sort_changing.set(true);
        imp.sort_dropdown.set_model(Some(&sort_model));
        imp.sort_dropdown.set_selected(page.current_sort_by());
        imp.sort_direction.set_active(page.current_sort_direction());
        imp.sort_changing.set(false);
    }

    pub fn show_detail_page<T: DetailPage>(
        &self,
        page: &impl IsA<adw::NavigationPage>,
        widget: &T,
        model: &T::Model,
    ) {
        let nav = &self.imp().main_navigation;
        let stack = nav.navigation_stack();
        let page_ref = page.upcast_ref::<adw::NavigationPage>();

        // Check if page is already in nav stack so we can just go to it instead
        let page_in_stack = stack
            .iter::<adw::NavigationPage>()
            .any(|p| p.ok().as_ref() == Some(page_ref));

        widget.set_model(model);
        page.set_title(&widget.title());

        if page_in_stack {
            nav.pop_to_page(page);
        } else {
            nav.push(page);
        }
    }

    pub fn show_album_detail(&self, album_model: &AlbumModel) {
        self.show_detail_page(
            &self.imp().album_detail_page.get(),
            &self.imp().album_detail.get(),
            album_model,
        );
    }

    pub fn show_artist_detail(&self, artist_model: &ArtistModel) {
        self.show_detail_page(
            &self.imp().artist_detail_page.get(),
            &self.imp().artist_detail.get(),
            artist_model,
        );
    }

    pub fn show_playlist_detail(&self, playlist_model: &PlaylistModel) {
        self.show_detail_page(
            &self.imp().playlist_detail_page.get(),
            &self.imp().playlist_detail.get(),
            playlist_model,
        );
    }

    pub fn show_about_dialog(&self) {
        about_dialog::show(self);
    }

    pub fn show_shortcuts_dialog(&self) {
        shortcuts_dialog::show(self);
    }

    pub fn show_preferences_dialog(&self) {
        let preferences_dialog = Preferences::new();
        preferences_dialog.present(Some(self));
    }

    pub fn logout(&self) {
        self.get_application().logout();
        self.show_server_setup();
        self.toast("Logged out", None);
    }

    pub fn save_window_size(&self) -> Result<(), glib::BoolError> {
        let size = self.default_size();
        let settings = settings();
        settings.set_int("window-width", size.0)?;
        settings.set_int("window-height", size.1)?;
        settings.set_boolean("window-maximized", self.is_maximized())?;

        Ok(())
    }

    fn load_window_size(&self) {
        let settings = settings();
        let width = settings.int("window-width");
        let height = settings.int("window-height");
        let maximized = settings.boolean("window-maximized");
        let width = if width < 0 { 875 } else { width };
        let height = if height < 0 { 900 } else { height };
        self.set_default_size(width, height);
        if maximized {
            self.maximize();
        }
    }

    pub fn loading_visible(&self, visible: bool) {
        self.imp().progress_bar.set_visible(visible);
        if visible {
            self.start_pulse_timer();
        }
    }

    fn start_pulse_timer(&self) {
        glib::timeout_add_local(
            std::time::Duration::from_millis(100),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or_else]
                || glib::ControlFlow::Break,
                move || {
                    let imp = window.imp();
                    if imp.progress_bar.is_visible() {
                        imp.progress_bar.pulse();
                        glib::ControlFlow::Continue
                    } else {
                        glib::ControlFlow::Break
                    }
                }
            ),
        );
    }

    fn call_on_visible_page<F>(&self, action: F)
    where
        F: Fn(&dyn TopPage),
    {
        if let Some(visible_child) = self.imp().stack.visible_child() {
            let imp = self.imp();
            if visible_child == imp.album_list.get().upcast::<gtk::Widget>() {
                action(&imp.album_list.get());
            } else if visible_child == imp.artist_list.get().upcast::<gtk::Widget>() {
                action(&imp.artist_list.get());
            } else if visible_child == imp.playlist_list.get().upcast::<gtk::Widget>() {
                action(&imp.playlist_list.get());
            } else if visible_child == imp.song_list.get().upcast::<gtk::Widget>() {
                action(&imp.song_list.get());
            } else {
                error!("Unknown page widget");
            }
        }
    }
}

mod imp {
    use std::cell::{Cell, RefCell};
    use std::sync::OnceLock;

    use adw::subclass::prelude::*;
    use glib::subclass::{InitializingObject, Signal};
    use gtk::{
        CompositeTemplate,
        gio::{ActionEntry, prelude::ActionMapExtManual},
        glib::{self, clone},
        prelude::*,
    };
    use log::{debug, warn};

    use crate::ui::{
        album_art_background::{create_blur_paintable, draw_background},
        artist_detail::ArtistDetail,
        page_traits::TopPage,
        player_bar::{big_player::BigPlayer, mini_player::MiniPlayerBar},
        song_list::SongList,
    };
    use crate::ui::{playlist_list::PlaylistList, widget_ext::WidgetApplicationExt};
    use crate::ui::{queue::Queue, setup::Setup};
    use crate::{application::Application, ui::album_detail::AlbumDetail};
    use crate::{
        config,
        ui::{album_list::AlbumList, artist_list::ArtistList, playlist_detail::PlaylistDetail},
    };

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/io/m51/Gelly/ui/window.ui")]
    pub struct Window {
        #[template_child]
        pub toaster: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub setup_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub stack: TemplateChild<adw::ViewStack>,
        #[template_child]
        pub setup: TemplateChild<Setup>,
        #[template_child]
        pub main_navigation: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub main_window: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub album_list: TemplateChild<AlbumList>,
        #[template_child]
        pub album_detail_page: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub album_detail: TemplateChild<AlbumDetail>,
        #[template_child]
        pub artist_list: TemplateChild<ArtistList>,
        #[template_child]
        pub artist_detail_page: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub artist_detail: TemplateChild<ArtistDetail>,
        #[template_child]
        pub playlist_list: TemplateChild<PlaylistList>,
        #[template_child]
        pub playlist_detail: TemplateChild<PlaylistDetail>,
        #[template_child]
        pub playlist_detail_page: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub song_list: TemplateChild<SongList>,
        #[template_child]
        pub player_bar: TemplateChild<MiniPlayerBar>,
        #[template_child]
        pub big_player: TemplateChild<BigPlayer>,
        #[template_child]
        pub queue: TemplateChild<Queue>,
        #[template_child]
        pub progress_bar: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub sort_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub search_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub new_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub split_view: TemplateChild<adw::OverlaySplitView>,
        #[template_child]
        pub bottom_sheet: TemplateChild<adw::BottomSheet>,
        #[template_child]
        pub search_bar: TemplateChild<gtk::SearchBar>,
        #[template_child]
        pub sort_bar: TemplateChild<gtk::SearchBar>,
        #[template_child]
        pub sort_dropdown: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub sort_direction: TemplateChild<adw::ToggleGroup>,
        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,

        pub blurred_paintable: RefCell<Option<gtk::gdk::Paintable>>,
        pub sort_changing: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "GellyApplicationWindow";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            crate::ui::auto_scroll_window::AutoScrollWindow::static_type();
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Window {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().load_window_size();

            let action_logout = ActionEntry::builder("logout")
                .activate(glib::clone!(
                    #[weak(rename_to=window)]
                    self,
                    move |_, _, _| {
                        window.obj().logout();
                    }
                ))
                .build();

            let action_clear_cache = ActionEntry::builder("clear-cache")
                .activate(glib::clone!(
                    #[weak(rename_to=window)]
                    self,
                    move |_, _, _| {
                        if let Some(image_cache) = window.obj().get_application().image_cache() {
                            image_cache.clear_cache();
                            debug!("Image cache cleared");
                        } else {
                            warn!("No image cache found");
                        }
                    }
                ))
                .build();

            let action_refresh_library = ActionEntry::builder("refresh-library")
                .activate(glib::clone!(
                    #[weak(rename_to=window)]
                    self,
                    move |_, _, _| {
                        let app = window.obj().get_application();
                        app.refresh_all(true);
                    }
                ))
                .build();

            let action_request_library_rescan = ActionEntry::builder("request-library-rescan")
                .activate(glib::clone!(
                    #[weak(rename_to=window)]
                    self,
                    move |_, _, _| {
                        let app = window.obj().get_application();
                        app.request_library_rescan();
                    }
                ))
                .build();

            let action_sidebar = ActionEntry::builder("toggle-queue")
                .activate(glib::clone!(
                    #[weak(rename_to=window)]
                    self,
                    move |_, _, _| {
                        window
                            .split_view
                            .set_show_sidebar(!window.split_view.shows_sidebar());
                    }
                ))
                .build();

            let action_search = ActionEntry::builder("search")
                .activate(glib::clone!(
                    #[weak(rename_to=window)]
                    self,
                    move |_, _, _| {
                        window
                            .search_bar
                            .set_search_mode(!window.search_bar.is_search_mode());
                    }
                ))
                .build();

            self.search_button
                .bind_property("active", &self.search_bar.get(), "search-mode-enabled")
                .bidirectional()
                .build();

            self.sort_button
                .bind_property("active", &self.sort_bar.get(), "search-mode-enabled")
                .bidirectional()
                .build();

            self.search_button.connect_active_notify(glib::clone!(
                #[weak(rename_to = sort_button)]
                self.sort_button,
                move |search_button| {
                    if search_button.is_active() {
                        sort_button.set_active(false);
                    }
                }
            ));

            self.sort_button.connect_active_notify(glib::clone!(
                #[weak(rename_to = search_button)]
                self.search_button,
                move |sort_button| {
                    if sort_button.is_active() {
                        search_button.set_active(false);
                    }
                }
            ));

            self.sort_dropdown.connect_selected_notify(glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_| {
                    if !window.sort_changing.get() {
                        window.obj().call_on_visible_page(|page| {
                            page.apply_sort(
                                window.sort_dropdown.selected(),
                                window.sort_direction.active(),
                            );
                        });
                    }
                }
            ));

            self.sort_direction.connect_active_notify(glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_| {
                    if !window.sort_changing.get() {
                        window.obj().call_on_visible_page(|page| {
                            page.apply_sort(
                                window.sort_dropdown.selected(),
                                window.sort_direction.active(),
                            );
                        });
                    }
                }
            ));

            self.album_list.setup_search_connection(&self.search_entry);
            self.artist_list.setup_search_connection(&self.search_entry);
            self.playlist_list
                .setup_search_connection(&self.search_entry);
            self.song_list.setup_search_connection(&self.search_entry);

            let action_new = ActionEntry::builder("new")
                .activate(glib::clone!(
                    #[weak(rename_to=window)]
                    self,
                    move |_, _, _| {
                        window.obj().call_on_visible_page(|page| {
                            page.create_new();
                        });
                    }
                ))
                .build();

            let action_play_selected = ActionEntry::builder("play-selected")
                .activate(glib::clone!(
                    #[weak(rename_to=window)]
                    self,
                    move |_, _, _| {
                        window.obj().call_on_visible_page(|page| {
                            page.play_selected();
                        });
                    }
                ))
                .build();

            let action_about = ActionEntry::builder("about")
                .activate(glib::clone!(
                    #[weak(rename_to=window)]
                    self,
                    move |_, _, _| {
                        window.obj().show_about_dialog();
                    }
                ))
                .build();

            let action_shortcuts = ActionEntry::builder("shortcuts")
                .activate(glib::clone!(
                    #[weak(rename_to=window)]
                    self,
                    move |_, _, _| {
                        window.obj().show_shortcuts_dialog();
                    }
                ))
                .build();

            let action_preferences = ActionEntry::builder("preferences")
                .activate(glib::clone!(
                    #[weak(rename_to=window)]
                    self,
                    move |_, _, _| {
                        window.obj().show_preferences_dialog();
                    }
                ))
                .build();

            let action_album_list = ActionEntry::builder("show-album-list")
                .activate(glib::clone!(
                    #[weak(rename_to=window)]
                    self,
                    move |_, _, _| {
                        window.stack.set_visible_child(&window.album_list.get());
                    }
                ))
                .build();

            let action_artist_list = ActionEntry::builder("show-artist-list")
                .activate(glib::clone!(
                    #[weak(rename_to=window)]
                    self,
                    move |_, _, _| {
                        window.stack.set_visible_child(&window.artist_list.get());
                    }
                ))
                .build();

            let action_playlist_list = ActionEntry::builder("show-playlist-list")
                .activate(glib::clone!(
                    #[weak(rename_to=window)]
                    self,
                    move |_, _, _| {
                        window.stack.set_visible_child(&window.playlist_list.get());
                    }
                ))
                .build();

            let action_song_list = ActionEntry::builder("show-song-list")
                .activate(glib::clone!(
                    #[weak(rename_to=window)]
                    self,
                    move |_, _, _| {
                        window.stack.set_visible_child(&window.song_list.get());
                    }
                ))
                .build();

            self.obj().add_action_entries([
                action_logout,
                action_clear_cache,
                action_refresh_library,
                action_request_library_rescan,
                action_search,
                action_new,
                action_play_selected,
                action_about,
                action_shortcuts,
                action_preferences,
                action_album_list,
                action_artist_list,
                action_playlist_list,
                action_song_list,
                action_sidebar,
            ]);

            self.stack.connect_notify_local(
                Some("visible-child"),
                glib::clone!(
                    #[weak(rename_to = window)]
                    self,
                    move |_, _| {
                        window.obj().show_visible_page();
                    }
                ),
            );

            self.obj().connect_map(glib::clone!(
                #[weak(rename_to = window)]
                self.obj(),
                move |_| {
                    let app = window.get_application();
                    app.connect_closure(
                        "global-error",
                        false,
                        glib::closure_local!(
                            #[weak]
                            window,
                            move |_app: Application, title: &str| {
                                window.toast(title, None);
                            }
                        ),
                    );

                    app.connect_closure(
                        "library-refreshed",
                        false,
                        glib::closure_local!(
                            #[weak]
                            window,
                            move |_app: Application, total_record_count: u64| {
                                // Makes sure if this signal is on another thread that the toast
                                // is created on the main thread (other thread dies)
                                glib::spawn_future_local(glib::clone!(
                                    #[weak]
                                    window,
                                    async move {
                                        window.toast(
                                            &format!("{} items added to library", total_record_count),
                                            Some(2),
                                        );
                                    }
                                ));
                            }
                        ),
                    );

                    app.connect_closure(
                        "library-rescan-requested",
                        false,
                        glib::closure_local!(
                            #[weak]
                            window,
                            move |_app: Application| {
                                window.toast(
                                    "Library rescan requested. Wait a few seconds and then use the \"Refresh Library\" option.",
                                    None,
                                );
                            }
                        ),
                    );

                    app.connect_closure(
                        "force-logout",
                        false,
                        glib::closure_local!(
                            #[weak]
                            window,
                            move |_app: Application| {
                                window.logout();
                            }
                        ),
                    );

                    app.connect_closure("http-request-start", false, glib::closure_local!(
                        #[weak]
                        window,
                        move |_app: Application| {
                            window.loading_visible(true);
                        }
                    ));

                    app.connect_closure("http-request-end", false, glib::closure_local!(
                        #[weak]
                        window,
                        move |_app: Application| {
                            window.loading_visible(false);
                        }
                    ));

                    // Refresh library once all signals are connected
                    app.refresh_all(config::get_refresh_on_startup());
                }
            ));

            let update_margin = clone!(
                #[weak(rename_to = main)]
                self.main_navigation,
                move |bs: &adw::BottomSheet| {
                    main.set_margin_bottom(if bs.reveals_bottom_bar() {
                        bs.bottom_bar_height()
                    } else {
                        0
                    });
                }
            );
            self.bottom_sheet
                .connect_reveal_bottom_bar_notify(update_margin.clone());
            self.bottom_sheet
                .connect_bottom_bar_height_notify(update_margin);

            self.big_player.connect_album_art_paintable_notify(clone!(
                #[weak(rename_to = this)]
                self,
                move |_| {
                    this.update_blurred_paintable();
                }
            ));

            config::settings().connect_changed(
                Some("album-art-window-background"),
                clone!(
                    #[weak(rename_to = this)]
                    self,
                    move |_, _| {
                        this.update_blurred_paintable();
                    }
                ),
            );
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("search").build(),
                    Signal::builder("sort").build(),
                    Signal::builder("play-selected").build(),
                ]
            })
        }
    }

    impl Window {
        fn update_blurred_paintable(&self) {
            let blurred = if config::settings().boolean("album-art-window-background") {
                self.big_player.album_art_paintable().and_then(|p| {
                    let obj = self.obj();
                    create_blur_paintable(
                        obj.upcast_ref::<gtk::Widget>(),
                        &p,
                        obj.width(),
                        obj.height(),
                    )
                })
            } else {
                None
            };
            if blurred.is_some() {
                self.obj().add_css_class("album-art-background");
            } else {
                self.obj().remove_css_class("album-art-background");
            }
            *self.blurred_paintable.borrow_mut() = blurred;
            self.obj().queue_draw();
        }
    }

    impl WidgetImpl for Window {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            if let Some(p) = self.blurred_paintable.borrow().as_ref() {
                let obj = self.obj();
                draw_background(snapshot, p, obj.width() as f64, obj.height() as f64, None);
            }
            self.parent_snapshot(snapshot);
        }
    }

    impl WindowImpl for Window {
        fn close_request(&self) -> glib::Propagation {
            self.obj()
                .save_window_size()
                .expect("Could not save window size");
            glib::Propagation::Proceed
        }
    }

    impl AdwApplicationWindowImpl for Window {}

    impl ApplicationWindowImpl for Window {}
}
