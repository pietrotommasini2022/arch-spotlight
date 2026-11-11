use gtk4::Application;
use gtk4::ApplicationWindow;
use gtk4::glib;
use gtk4::prelude::*;

const APP_ID: &str = "com.pietrotommasini.archspotlight";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);

    app.run()
}

fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder().application(app)
    .default_width(700)
    .default_height(60)
    .build();
    window.present();
}