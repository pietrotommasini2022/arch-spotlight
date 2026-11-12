use gtk4::Application;
use gtk4::ApplicationWindow;
use gtk4::Box as GtkBox;
use gtk4::Orientation;
use gtk4::SearchEntry;
use gtk4::glib;
use gtk4::prelude::*;
use ignore::WalkBuilder;
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher};
use std::path::Path;

const APP_ID: &str = "com.pietrotommasini.archspotlight";
const IGNORED_DIRS: [&str; 4] = [".cache", ".git", "target", "node_modules"];

fn main() -> glib::ExitCode {
    // walk_directories(Path::new("/home/pietro/prova"));
    fuzzy_test();

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

    let window = ApplicationWindow::builder()
        .application(app)
        .default_width(700)
        .default_height(60)
        .build();

    window.set_child(Some(&container));
    window.present();
}

fn walk_directories(query_path: &Path) {
    let walker = WalkBuilder::new(query_path)
        .hidden(false)
        .filter_entry(|entry| {
            !IGNORED_DIRS
                .iter()
                .any(|ignored| entry.file_name() == *ignored)
        })
        .build();
    for item in walker {
        match item {
            Ok(res) => {
                let res_path = res.path();
                let res_name = res_path.display();
                println!("{res_name}")
            }
            Err(err) => {
                println!("{err}")
            }
        }
    }
}

fn fuzzy_test() {
    let list = ["hyprland.conf", "hyprpaper.conf", "photo_cyprus.jpg", "why_pc.txt", "chyp.txt", "README.md", "readme_vecchio.txt", "città.txt", "config.toml"];
    let mut matcher = Matcher::new(Config::DEFAULT);
    let pattern = Pattern::parse("citta", CaseMatching::Smart, Normalization::Smart);
    let items = pattern.match_list(list, &mut matcher);
    for (name, value) in items {
        println!("name: {name} score: {value}");
    }
}
