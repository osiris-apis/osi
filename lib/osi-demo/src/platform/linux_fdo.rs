//! Platform Layer: Linux with freedesktop.org APIs
//!
//! Implement the application state and UI handling via GTK4. This uses
//! GObject, thus we have to follow GObject type+instance rules.
//!
//! The UI uses a simple text-view to show all output, and an input-entry
//! to accept commands from the user.

use adw::{
    self as adw,
    prelude as adwp,
    subclass::prelude as adwsp,
};
use gtk::{self, gio, glib};

const APP_ID: &str = "foo.osiris.demo.App";
const APP_TITLE: &str = "Osiris Demo Application";

#[derive(Default)]
pub struct AppState {
    log: gtk::TextBuffer,
}

glib::wrapper! {
    pub struct App(ObjectSubclass<AppState>)
        @extends adw::Application, gio::Application, gtk::Application,
        @implements gtk::Buildable;
}

#[glib::object_subclass]
impl adwsp::ObjectSubclass for AppState {
    const NAME: &'static str = "OsiDemoApplication";
    type Type = App;
    type ParentType = adw::Application;

    fn new() -> Self {
        Self {
            log: gtk::TextBuffer::new(None),
        }
    }
}

impl adwsp::ObjectImpl for AppState {}
impl adwsp::ApplicationImpl for AppState {}
impl adwsp::GtkApplicationImpl for AppState {}
impl adwsp::AdwApplicationImpl for AppState {}

impl App {
    fn build(app: &App) {
        let app_state = <AppState as adwsp::ObjectSubclassExt>::from_obj(app);

        <_ as adwp::TextBufferExt>::insert_at_cursor(&app_state.log, "platform: initialized\n");

        let log_view = gtk::TextView::builder()
            .hexpand(true)
            .vexpand(true)

            .margin_top(2)
            .margin_end(2)
            .margin_bottom(2)
            .margin_start(2)

            .accepts_tab(false)
            .can_focus(false)
            .editable(false)
            .monospace(true)
            .buffer(&app_state.log)

            .build();

        let log_scroll = gtk::ScrolledWindow::builder()
            .child(&log_view)
            .can_focus(false)
            .build();

        let log_input = gtk::Entry::builder()
            .margin_top(2)
            .margin_end(2)
            .margin_bottom(2)
            .margin_start(2)

            .build();

        let log_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        <_ as adwp::BoxExt>::append(&log_box, &log_scroll);
        <_ as adwp::BoxExt>::append(&log_box, &log_input);

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title(APP_TITLE)
            .default_width(640)
            .default_height(480)
            .content(&log_box)
            .build();

        <_ as adwp::GtkWindowExt>::present(&window);
    }

    pub fn new() -> Self {
        gtk::init().unwrap();

        let app: App = glib::Object::new();

        <_ as adwp::ApplicationExt>::set_application_id(&app, Some(APP_ID));
        <_ as adwp::ApplicationExt>::connect_activate(&app, Self::build);

        app
    }

    pub fn run(&self) -> std::process::ExitCode {
        let r = <_ as adwp::ApplicationExtManual>::run(self);
        <_ as std::process::Termination>::report(r)
    }
}
