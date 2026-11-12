use gtk4::Application;
use gtk4::ApplicationWindow;
use gtk4::Box as GtkBox;
use gtk4::Orientation;
use gtk4::SearchEntry;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Label, ListBox};
use ignore::WalkBuilder;
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher};
use std::path::Path;

const APP_ID: &str = "com.pietrotommasini.archspotlight";
const IGNORED_DIRS: [&str; 4] = [".cache", ".git", "target", "node_modules"];
const MAX_RESULTS: usize = 10;

#[derive(Debug, Clone)]
struct FileEntry {
    name: String,
    path: String,
}

impl AsRef<str> for FileEntry {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

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

    let entries_container = ListBox::builder().build();
    let ec_clone = entries_container.clone();

    let searchbar = SearchEntry::builder().build();
    searchbar.connect_search_changed(move |entry| {
        let query = entry.text();
        ec_clone.remove_all();
        if query.is_empty() {
            return;
        }
        let fuzzy_result = fuzzy_search(&query_result, &query);
        for item in fuzzy_result {
            ec_clone.append(&Label::new(Some(&item.name)));
        }
    });

    container.append(&searchbar);
    container.append(&entries_container);

    let window = ApplicationWindow::builder()
        .application(app)
        .default_width(700)
        .default_height(60)
        .build();

    window.set_child(Some(&container));
    window.present();
}

fn walk_directories(query_path: &Path) -> Vec<FileEntry> {
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
                let name = res.file_name().to_string_lossy().to_string();
                let path = res.path().display().to_string();
                let entry = FileEntry {
                    name: name,
                    path: path,
                };
                query_name_list.push(entry)
            }
            Err(err) => {
                println!("{err}")
            }
        }
    }
    query_name_list
}

fn fuzzy_search(paths: &Vec<FileEntry>, query: &str) -> Vec<FileEntry> {
    let mut matcher = Matcher::new(Config::DEFAULT);
    let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
    let mut items = pattern.match_list(paths, &mut matcher);
    items.truncate(MAX_RESULTS);
    let mut res = Vec::new();
    for (entry, _score) in items {
        res.push(entry.clone())
    }
    res
}
