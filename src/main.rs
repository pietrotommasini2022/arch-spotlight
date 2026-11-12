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

    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);

    app.run()
}

fn build_ui(app: &Application) {
    let query_result = walk_directories(Path::new("/home/pietro/prova"));
    let container = GtkBox::builder()
        .hexpand(true)
        .margin_bottom(2)
        .margin_top(2)
        .margin_start(2)
        .margin_end(2)
        .orientation(Orientation::Vertical)
        .build();

    let searchbar = SearchEntry::builder().build();
    searchbar.connect_search_changed(move |entry| {
        let query = entry.text();
        fuzzy_test(&query_result, &query);
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

fn walk_directories(query_path: &Path) -> Vec<String> {
    let walker = WalkBuilder::new(query_path)
        .hidden(false)
        .filter_entry(|entry| {
            !IGNORED_DIRS
                .iter()
                .any(|ignored| entry.file_name() == *ignored)
        })
        .build();
    let mut query_name_list = Vec::new();
    for item in walker {
        match item {
            Ok(res) => {
                let res_name: String = res.path().display().to_string();
                query_name_list.push(res_name)
            }
            Err(err) => {
                println!("{err}")
            }
        }
    }
    query_name_list
}

fn fuzzy_test(paths:&Vec<String>, query:&str) {
    let mut matcher = Matcher::new(Config::DEFAULT);
    let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
    let items = pattern.match_list(paths, &mut matcher);
    for (name, value) in items {
        println!("name: {name} score: {value}");
    }
}
