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
use std::cell::RefCell;
use std::path::Path;
use std::process::Command;
use std::rc::Rc;

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
    let a_clone = app.clone();
    let a_clone_refine = app.clone();
    let ec_clone_refine = entries_container.clone();

    let shared_box = Rc::new(RefCell::new(Vec::new()));
    let sb_clone = shared_box.clone();

    let searchbar = SearchEntry::builder().build();
    searchbar.connect_search_changed(move |entry| {
        let query = entry.text();
        ec_clone.remove_all();
        if query.is_empty() {
            sb_clone.replace(Vec::new());
            return;
        }
        let fuzzy_result = fuzzy_search(&query_result, &query);
        for item in &fuzzy_result {
            let single_result_container = GtkBox::builder()
                .orientation(Orientation::Vertical)
                .hexpand(true)
                .build();

            let main_result_row = Label::builder().label(&item.name).xalign(0.0).build();
            let main_path_row = Label::builder().label(&item.path).xalign(0.0).build();
            main_path_row.add_css_class("dim-label");

            single_result_container.append(&main_result_row);
            single_result_container.append(&main_path_row);

            ec_clone.append(&single_result_container);
        }
        sb_clone.replace(fuzzy_result);
    });

    entries_container.connect_row_activated(move |_list, row| {
        let index = row.index() as usize;
        let entries = shared_box.borrow();
        match entries.get(index) {
            Some(entry) => {
                let open_cmd = Command::new("xdg-open").arg(&entry.path).spawn();

                match open_cmd {
                    Ok(_res) => {
                        a_clone.quit();
                    }
                    Err(err) => {
                        println!("{err}")
                    }
                }
            }
            None => {
                println!("nothing")
            }
        }
    });

    searchbar.connect_activate(move |_entry|  {
        match ec_clone_refine.row_at_index(0) {
            Some(res) => {
                res.activate();
            },
            None => {
                println!("no item in the list")
            }
        }
    });

    searchbar.connect_stop_search(move |_searchbar| {
        a_clone_refine.quit();
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
