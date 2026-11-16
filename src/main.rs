// =====================================================================
// arch-spotlight: barra di ricerca file in stile Spotlight (Rust + GTK4)
//
// Flusso generale:
//   1. all'avvio carica la lista file dalla CACHE su disco (veloce)
//   2. apre subito la finestra con la barra di ricerca
//   3. in un thread separato scansiona davvero la home e riscrive la cache
//   4. a ogni tasto premuto fa una ricerca fuzzy sui nomi dei file
//   5. Invio / click apre il file con xdg-open, Esc chiude l'app
// =====================================================================

use gtk4::Application;
use gtk4::ApplicationWindow;
use gtk4::Box as GtkBox; // rinominato: `Box` esiste già in Rust (è un'altra cosa)
use gtk4::Orientation;
use gtk4::SearchEntry;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Label, ListBox};
use gtk4_layer_shell::{LayerShell, KeyboardMode, Layer, Edge};
use ignore::WalkBuilder;
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher};
use serde::Deserialize;
use std::cell::RefCell;
use std::path::Path;
use std::process::Command;
use std::rc::Rc;

// ---------------------------------------------------------------------
// Costanti (diventeranno configurabili nella tappa 9)
// ---------------------------------------------------------------------

/// Identificativo univoco dell'applicazione per GTK.
const APP_ID: &str = "com.pietrotommasini.archspotlight";

/// Cartelle da NON esplorare durante la scansione (denylist).
/// Non saltiamo tutte le cartelle nascoste perché, per esempio, .config ci interessa.
const IGNORED_DIRS: [&str; 4] = [".cache", ".git", "target", "node_modules"];

/// Quanti risultati mostrare al massimo nella lista.
const MAX_RESULTS: usize = 10;

// ---------------------------------------------------------------------
// Tipi
// ---------------------------------------------------------------------

/// Un file trovato sul disco.
/// - `name`: solo il nome (es. "note.txt"), usato per la ricerca fuzzy
/// - `path`: percorso completo, usato per mostrarlo e per aprire il file
#[derive(Debug, Clone)]
struct FileEntry {
    name: String,
    path: String,
    is_dir: bool,
}

/// nucleo-matcher ha bisogno di "vedere" ogni elemento come testo (&str).
/// Qui gli diciamo: per una FileEntry, il testo su cui cercare è il NOME.
impl AsRef<str> for FileEntry {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct ConfigVar {
    ignored_dirs: Vec<String>,
    max_results: usize,
    show_hidden: bool,
    main_dir: String,
    style_path: String,
    icon_theme: String,
}

impl Default for ConfigVar {
    fn default() -> Self {
        ConfigVar {
            ignored_dirs: IGNORED_DIRS.iter().map(|item| item.to_string()).collect(),
            max_results: MAX_RESULTS,
            show_hidden: true,
            main_dir: home_dir(),
            style_path: style_path(),
            icon_theme: "Adwaita".to_string(),
        }
    }
}

// ---------------------------------------------------------------------
// Avvio
// ---------------------------------------------------------------------

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    // Quando l'app è pronta, GTK chiama build_ui per costruire la finestra.
    app.connect_activate(build_ui);
    // Avvia il main loop: da qui in poi il programma reagisce agli eventi.
    app.run()
}

/// Costruisce la finestra e collega tutti i segnali (eventi).
fn build_ui(app: &Application) {
    if app.active_window().is_some() {
        app.quit();
        return
    }
    let config = load_config();
    // -----------------------------------------------------------------
    // Stato condiviso
    // -----------------------------------------------------------------
    // Rc<RefCell<...>> = una scatola condivisa tra più closure (Rc)
    // il cui contenuto si può modificare anche se condivisa (RefCell).

    // TUTTI i file conosciuti: parte dalla cache, poi viene sostituita
    // dal risultato della scansione in background.
    let all_files = Rc::new(RefCell::new(load_cache()));

    // Solo i file attualmente MOSTRATI nella lista dei risultati,
    // nello stesso ordine delle righe: la riga N corrisponde a visible_results[N].
    let visible_results = Rc::new(RefCell::new(Vec::new()));

    // -----------------------------------------------------------------
    // Widget
    // -----------------------------------------------------------------

    // Contenitore principale verticale: sopra la barra, sotto i risultati.
    // È lei l'"isola" nera arrotondata: margini, sfondo e bordi stanno in style.css.
    let main_layout = GtkBox::builder()
        .hexpand(true)
        .orientation(Orientation::Vertical)
        .build();
    main_layout.add_css_class("island");

    // Barra di ricerca.
    let searchbar = SearchEntry::builder().build();
    searchbar.add_css_class("search_bar");

    // Lista verticale dei risultati (una riga per file trovato).
    let results_list = ListBox::builder().build();
    results_list.add_css_class("results_list");

    // -----------------------------------------------------------------
    // Segnale: il testo della barra è cambiato -> nuova ricerca
    // -----------------------------------------------------------------
    // Ogni closure `move` si prende i propri "telecomandi" (clone),
    // creati subito prima della closure che li usa.
    let results_list_for_search = results_list.clone();
    let all_files_for_search = all_files.clone();
    let visible_results_for_search = visible_results.clone();
    searchbar.connect_search_changed(move |entry| {
        let query = entry.text();

        // Svuota la lista grafica prima di riempirla con i nuovi risultati.
        results_list_for_search.remove_all();

        // Barra vuota: nessun risultato da mostrare.
        if query.is_empty() {
            visible_results_for_search.replace(Vec::new());
            return;
        }

        // borrow() = guardo dentro la scatola senza modificarla.
        let files = all_files_for_search.borrow();
        let matches = fuzzy_search(&files, &query, config.max_results);

        // Per ogni risultato crea una riga: nome sopra, path (grigio) sotto.
        for file in &matches {
            let file_type = if file.is_dir {
                "inode/directory".to_string()
            } else {
                let guess = gio::content_type_guess(Some(&file.path), None::<&[u8]>);
                guess.0.to_string()
            };
            let icon = gio::content_type_get_icon(&file_type);
            let icon_image = gtk4::Image::builder().gicon(&icon).pixel_size(32).build();

            let row_container = GtkBox::builder()
                .orientation(Orientation::Horizontal)
                .hexpand(true)
                .spacing(10)
                .build();
            let row_layout = GtkBox::builder()
                .orientation(Orientation::Vertical)
                .hexpand(true)
                .build();

            let name_label = Label::builder()
                .label(&file.name)
                .xalign(0.0)
                .ellipsize(gtk4::pango::EllipsizeMode::End)
                .build();
            let path_label = Label::builder()
                .label(&file.path)
                .xalign(0.0)
                .ellipsize(gtk4::pango::EllipsizeMode::Start)
                .build();
            path_label.add_css_class("path_label"); // stile GTK per testo attenuato
            name_label.add_css_class("name_label");

            row_layout.append(&name_label);
            row_layout.append(&path_label);
            row_container.append(&icon_image);
            row_container.append(&row_layout);

            results_list_for_search.append(&row_container);
        }

        // Ricorda quali file sono in lista, così il click sa cosa aprire.
        visible_results_for_search.replace(matches);
    });

    // -----------------------------------------------------------------
    // Segnale: una riga è stata attivata (click o Invio) -> apri il file
    // -----------------------------------------------------------------
    let app_for_open = app.clone();
    results_list.connect_row_activated(move |_list, row| {
        // Posizione della riga cliccata (GTK usa i32, Vec::get vuole usize).
        let row_index = row.index() as usize;
        let results = visible_results.borrow();

        // get() restituisce Option: None se l'indice non esiste.
        match results.get(row_index) {
            Some(file) => {
                // Lancia xdg-open, che apre il file con il programma predefinito.
                let open_result = Command::new("xdg-open").arg(&file.path).spawn();

                match open_result {
                    Ok(_child_process) => {
                        app_for_open.quit(); // file aperto: chiudiamo la barra
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

    // -----------------------------------------------------------------
    // Segnale: Invio nella barra -> attiva il primo risultato
    // -----------------------------------------------------------------
    // Riusa il segnale row_activated qui sopra, senza duplicare codice.
    let results_list_for_enter = results_list.clone();
    searchbar.connect_activate(move |_entry| match results_list_for_enter.row_at_index(0) {
        Some(first_row) => {
            first_row.activate();
        }
        None => {
            println!("no item in the list")
        }
    });

    // -----------------------------------------------------------------
    // Segnale: Esc nella barra -> chiudi l'app
    // -----------------------------------------------------------------
    let app_for_escape = app.clone();
    searchbar.connect_stop_search(move |_searchbar| {
        app_for_escape.quit();
    });

    // -----------------------------------------------------------------
    // Montaggio della finestra
    // -----------------------------------------------------------------
    main_layout.append(&searchbar);
    main_layout.append(&results_list);

    let window = ApplicationWindow::builder()
        .application(app)
        .default_width(700)
        .default_height(60)
        .build();

    window.set_child(Some(&main_layout));
    // Qualsiasi tasto premuto nella finestra finisce nella barra di ricerca.
    searchbar.set_key_capture_widget(Some(&window));
    // present() non disegna subito: mette in coda, il main loop la mostrerà.
    load_css(&window, &config);
    let icon_theme = gtk4::Settings::default();
        match icon_theme {
            Some(theme) => {
                theme.set_gtk_icon_theme_name(Some(&config.icon_theme.as_str()));
            }
            None => {

            }
        }
    window.init_layer_shell();
    window.set_keyboard_mode(KeyboardMode::Exclusive);
    window.set_layer(Layer::Overlay);
    window.set_anchor(Edge::Top, true);
    window.set_margin(Edge::Top, 100);
    window.present();

    // -----------------------------------------------------------------
    // Scansione in background + aggiornamento della cache
    // -----------------------------------------------------------------
    // spawn_blocking: la closure gira in un ALTRO thread (la "cucina"),
    // così la finestra resta reattiva. Restituisce subito uno "scontrino".
    // Qui dentro NON si possono usare Rc/RefCell né widget: vivono solo
    // sul thread della UI.
    let scan_task = gio::spawn_blocking(move || {
        let scanned_files = walk_directories(
            Path::new(&config.main_dir),
            config.show_hidden,
            config.ignored_dirs,
        );

        // Salva la cache per il prossimo avvio. &scanned_files = in prestito,
        // così la lista resta nostra e possiamo restituirla alla fine.
        let cache_text = build_cache_text(&scanned_files);
        match std::fs::write(cache_path(), cache_text) {
            Ok(_) => {}
            Err(err) => {
                println!("{err}")
            }
        };

        // Ultima espressione senza `;` = valore restituito dalla closure.
        scanned_files
    });

    // spawn_future_local: codice async sul thread della UI ("local").
    // .await aspetta lo scontrino SENZA bloccare l'interfaccia.
    // Tutto quello che viene dopo .await gira sul thread della UI.
    let all_files_for_scan = all_files.clone();
    let searchbar_for_scan = searchbar.clone();
    glib::spawn_future_local(async move {
        let scan_result = scan_task.await;
        match scan_result {
            Ok(scanned_files) => {
                // Sostituisce la lista vecchia (dalla cache) con quella fresca.
                all_files_for_scan.replace(scanned_files);
                // Rifà la ricerca, nel caso l'utente abbia già scritto qualcosa.
                searchbar_for_scan.emit_by_name("search-changed", &[])
            }
            Err(_) => {
                println!("scan failed")
            }
        }
    });
}

// ---------------------------------------------------------------------
// Funzioni di supporto
// ---------------------------------------------------------------------

/// Esplora ricorsivamente `root` e restituisce tutti i file e le cartelle
/// trovati, saltando quelle in IGNORED_DIRS.
fn walk_directories(root: &Path, show_hidden: bool, ignored_dirs: Vec<String>) -> Vec<FileEntry> {
    let walker = WalkBuilder::new(root)
        .hidden(!show_hidden) // includi anche i file nascosti (.config ecc.)
        // filter_entry: se la closure restituisce false, la cartella
        // (e tutto il suo contenuto) viene saltata.
        .filter_entry(move |dir_entry| {
            !ignored_dirs
                .iter()
                .any(|ignored| *dir_entry.file_name() == **ignored)
        })
        .build();

    let mut found_files = Vec::new();
    for walk_result in walker {
        // Ogni elemento è un Result: può fallire (es. permessi negati).
        match walk_result {
            Ok(dir_entry) => {
                let name = dir_entry.file_name().to_string_lossy().to_string();
                let path = dir_entry.path().display().to_string();
                let is_dir: bool = match dir_entry.file_type() {
                    Some(res) => res.is_dir(),
                    None => false,
                };
                found_files.push(FileEntry { name, path, is_dir })
            }
            Err(err) => {
                println!("{err}")
            }
        }
    }
    found_files
}

/// Ricerca fuzzy di `query` sui nomi dei file.
/// Restituisce al massimo MAX_RESULTS file, dal più al meno pertinente.
fn fuzzy_search(files: &Vec<FileEntry>, query: &str, max_results: usize) -> Vec<FileEntry> {
    let mut matcher = Matcher::new(Config::DEFAULT);
    // Smart: maiuscole/minuscole e accenti ignorati, a meno che la query li usi.
    let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);

    // match_list restituisce coppie (file, punteggio), già ordinate per punteggio.
    let mut scored_matches = pattern.match_list(files, &mut matcher);
    scored_matches.truncate(max_results);

    // Teniamo solo i file (senza punteggio), clonati: match_list ci dà
    // riferimenti alla lista originale, ma a noi servono copie nostre.
    let mut best_matches = Vec::new();
    for (file, _score) in scored_matches {
        best_matches.push(file.clone())
    }
    best_matches
}

/// Trasforma la lista di file nel testo da salvare in cache:
/// un path per riga.
fn build_cache_text(files: &Vec<FileEntry>) -> String {
    let mut cache_text = String::new();
    for file in files {
        cache_text.push_str(&file.path);
        if file.is_dir {
            cache_text.push('/');
        }
        cache_text.push('\n');
    }
    cache_text
}

/// Legge la cache dal disco e ricostruisce la lista di file.
/// Se la cache non esiste (es. primo avvio) restituisce una lista vuota.
fn load_cache() -> Vec<FileEntry> {
    match std::fs::read_to_string(cache_path()) {
        Ok(cache_text) => {
            let mut loaded_files = Vec::new();
            for line in cache_text.lines() {
                // Il nome è l'ultimo pezzo del path. file_name() restituisce
                // Option perché alcuni path (es. "/") non ne hanno uno.
                let (clean_path, is_dir) = match line.strip_suffix('/') {
                    Some(clean_line) => (clean_line.to_string(), true),
                    None => (line.to_string(), false),
                };
                let name = match Path::new(&clean_path).file_name() {
                    Some(file_name) => file_name.to_string_lossy().to_string(),
                    None => continue, // riga strana: saltala
                };
                loaded_files.push(FileEntry {
                    name: name,
                    path: clean_path,
                    is_dir: is_dir,
                });
            }
            loaded_files
        }
        // Nessuna cache: va bene, ci penserà la scansione in background.
        Err(_) => Vec::new(),
    }
}

fn load_config() -> ConfigVar {
    let config_file = std::fs::read_to_string(config_path());
    match config_file {
        Ok(config_text) => {
            let read_text = toml::from_str::<ConfigVar>(&config_text);
            match read_text {
                Ok(config) => config,
                Err(err) => {
                    println!("{err}");
                    ConfigVar::default()
                }
            }
        }
        Err(err) => {
            println!("{err}");
            ConfigVar::default()
        }
    }
}

fn home_dir() -> String {
    match std::env::var("HOME") {
        Ok(home) => home,
        Err(_) => "/tmp".to_string(),
    }
}

fn config_path() -> String {
    let config_path = format!("{}/.config/arch-spotlight/config.toml", home_dir());
    return config_path;
}

fn cache_path() -> String {
    let cache_path = format!("{}/.cache/arch-spotlight-index.txt", home_dir());
    return cache_path;
}

fn style_path() -> String {
    let style_path = format!("{}/.config/arch-spotlight/style.css", home_dir());
    return style_path;
}

fn load_css(window: &ApplicationWindow, config: &ConfigVar) {
    let css_provider = gtk4::CssProvider::new();
    let css_path = &config.style_path;
    match std::fs::read_to_string(css_path) {
        Ok(css_info) => {
            css_provider.load_from_string(&css_info);
            gtk4::style_context_add_provider_for_display(
                &RootExt::display(window),
                &css_provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            )
        }
        Err(_) => {
            println!("couldn't find the style.css file")
        }
    }
}
