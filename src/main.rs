use gtk4::Application;
use gtk4::ApplicationWindow;
use gtk4::Orientation;
use gtk4::SearchEntry;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::Box as GtkBox;

const APP_ID: &str = "com.pietrotommasini.archspotlight";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);

    app.run()
}

fn build_ui(app: &Application) {
    
    let container = GtkBox::builder()
    .hexpand(true)
    .margin_bottom(2)
    .margin_top(2)
    .margin_start(2)
    .margin_end(2)
    .orientation(Orientation::Vertical)
    .build();

    let searchbar = SearchEntry::builder().build();
    searchbar.connect_search_changed(|entry| {
        let query = entry.text();
        println!("{query}");
    });

    container.append(&searchbar);
    
    let window = ApplicationWindow::builder().application(app)
    .default_width(700)
    .default_height(60)
    .build();

    window.set_child(Some(&container));
    window.present();


}