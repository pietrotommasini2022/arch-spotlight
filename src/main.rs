use gtk4::Application;
use gtk4::ApplicationWindow;
use gtk4::Orientation;
use gtk4::SearchEntry;
use std::path::Path;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::Box as GtkBox;

const APP_ID: &str = "com.pietrotommasini.archspotlight";

fn main() -> glib::ExitCode {

    query_directories(Path::new("/home/pietro/Progetti/arch-spotlight"));
    
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

fn query_directories(query_path: &Path) {
    let results = std::fs::read_dir(query_path);
    match results {
        Ok(res) => {
            for item in res {
                match item {
                    Ok(item_res) => {
                        let item_type = item_res.file_type();
                        match item_type {
                            Ok(type_res) => {
                                let is_dir = type_res.is_dir();
                                let file_path = item_res.path();
                                let file_path_name = file_path.display();
                                println!("{file_path_name} cartella? {is_dir}")
                            },
                            Err(type_err) => {
                                println!("{type_err}")
                            }
                        }
                    },
                    Err(item_err) => {
                        println!("{item_err}")
                    }
                }
            }
        },
        Err(err) => {println!("{err}")}
    }
}