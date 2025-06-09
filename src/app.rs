use crate::database;
use crate::event::{AppEvent, Event, EventHandler};
use crate::settings;
use ratatui::{
    DefaultTerminal,
    crossterm::event::{KeyCode, KeyEvent},
    widgets::ListState,
};
use regex::Regex;
use rusty_tesseract::{Args, Image};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tui_textarea::{CursorMove, TextArea};

pub struct App<'a> {
    pub bon_list: BonList,
    pub bon_summary: Vec<SummaryEntry>,
    pub current_state: AppState,
    database: database::Database,
    pub edit_field: TextArea<'a>,
    events: EventHandler,
    pub import_list: FileList,
    import_path: String,
    pub ocr_blacklist: Vec<String>,
    pub ocr_list: OcrList,
    pub ocr_file: String,
    running: bool,
}

pub struct BonList {
    pub items: Vec<database::Bon>,
    pub state: ListState,
}

pub struct FileList {
    pub items: Vec<String>,
    pub state: ListState,
}

pub struct OcrList {
    pub items: Vec<String>,
    pub state: ListState,
}

pub enum AppState {
    Blacklist,
    Home,
    Import,
    OCR,
}

pub struct SummaryEntry {
    pub category: String,
    pub total: f64,
}

impl Default for App<'_> {
    fn default() -> Self {
        let settings = settings::Settings::new();
        let database_exists = settings.database_exists();
        let database = database::Database::new(&settings.database_file);
        if !database_exists {
            database.create_database();
        }
        let bons = database.get_bons();
        let blacklist = database.get_blacklist();
        let import_list = fs::read_dir(settings.import_path())
            .expect("Couldn't read bons directory")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect::<Vec<String>>();
        Self {
            bon_list: BonList {
                items: bons,
                state: ListState::default(),
            },
            bon_summary: Vec::new(),
            current_state: AppState::Home,
            database,
            edit_field: TextArea::default(),
            events: EventHandler::new(),
            import_list: FileList {
                items: import_list,
                state: ListState::default(),
            },
            import_path: settings.import_path(),
            ocr_blacklist: blacklist,
            ocr_list: OcrList {
                items: Vec::new(),
                state: ListState::default(),
            },
            ocr_file: String::new(),
            running: true,
        }
    }
}

impl App<'_> {
    fn calculate_summary(&mut self) {
        if let Some(i) = self.bon_list.state.selected() {
            let bon = &self.bon_list.items[i];
            self.bon_summary.clear();
            let mut summary_map: HashMap<String, f64> = HashMap::new();
            bon.entries.iter().for_each(|entry| {
                summary_map
                    .entry(entry.category.clone())
                    .and_modify(|value| *value += entry.price)
                    .or_insert(entry.price);
            });
            summary_map.iter().for_each(|(category, total)| {
                self.bon_summary.push(SummaryEntry {
                    category: category.clone(),
                    total: *total,
                });
            });
            let total_sum: f64 = self.bon_summary.iter().map(|e| e.total).sum();
            self.bon_summary.push(SummaryEntry {
                category: "total".to_string(),
                total: total_sum,
            });
        }
    }

    pub fn handle_key_events(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        if matches!(self.current_state, AppState::Blacklist) {
            match key_event.code {
                KeyCode::Enter => {
                    self.database
                        .add_blacklist_entry(self.edit_field.lines()[0].as_str());
                    self.events.send(AppEvent::GoOcrState);
                    self.events.send(AppEvent::UpdateFromDatabase);
                }
                KeyCode::Esc => self.events.send(AppEvent::GoOcrState),
                _ => _ = self.edit_field.input(key_event),
            }
        } else {
            match key_event.code {
                KeyCode::Char('b') => {
                    if matches!(self.current_state, AppState::OCR) {
                        self.edit_field.move_cursor(CursorMove::End);
                        self.edit_field.delete_line_by_head();
                        if let Some(i) = self.ocr_list.state.selected() {
                            self.edit_field.insert_str(self.ocr_list.items[i].as_str());
                        }
                        self.events.send(AppEvent::GoBlacklistState);
                    }
                }
                KeyCode::Char('i') => self.events.send(AppEvent::GoImportState),
                KeyCode::Char('j') => self.events.send(AppEvent::NextItem),
                KeyCode::Char('k') => self.events.send(AppEvent::PreviousItem),
                KeyCode::Char('q') => self.events.send(AppEvent::Quit),
                KeyCode::Char('x') => {
                    if matches!(self.current_state, AppState::OCR) {
                        if let Some(i) = self.ocr_list.state.selected() {
                            self.ocr_list.items.remove(i);
                        }
                    }
                }
                KeyCode::Enter => {
                    if matches!(self.current_state, AppState::Import) {
                        let file_path = Path::new(&self.import_path);
                        if let Some(i) = self.import_list.state.selected() {
                            let file_name = self.import_list.items[i].clone();
                            self.ocr_file = file_path
                                .join(file_name)
                                .to_str()
                                .expect("Couldn't convert path to string")
                                .to_string();
                        }
                        self.events.send(AppEvent::GoOcrState);
                    }
                }
                KeyCode::Esc => self.events.send(AppEvent::GoHomeState),
                _ => {}
            }
        }
        Ok(())
    }

    fn go_blacklist_state(&mut self) {
        if matches!(self.current_state, AppState::OCR) {
            self.current_state = AppState::Blacklist;
        }
    }

    fn go_home_state(&mut self) {
        self.ocr_list.items.clear();
        self.ocr_list.state = ListState::default();
        self.current_state = AppState::Home;
    }

    fn go_import_state(&mut self) {
        if matches!(self.current_state, AppState::Home) {
            self.current_state = AppState::Import;
        }
    }

    fn go_ocr_state(&mut self) {
        self.current_state = AppState::OCR;
        if self.ocr_list.items.is_empty() {
            self.ocr_list.items = vec!["Processing..".to_string()];
            self.events.send(AppEvent::PerformOCR);
        }
    }

    pub fn new() -> Self {
        Self::default()
    }

    fn next_item(&mut self) {
        match self.current_state {
            AppState::Home => {
                if let Some(i) = self.bon_list.state.selected() {
                    if i < self.bon_list.items.len() - 1 {
                        self.bon_list.state.select_next();
                        self.events.send(AppEvent::CalculateSummary);
                    }
                }
            }
            AppState::Import => {
                if let Some(i) = self.import_list.state.selected() {
                    if i < self.import_list.items.len() - 1 {
                        self.import_list.state.select_next();
                    }
                }
            }
            AppState::OCR => {
                if let Some(i) = self.ocr_list.state.selected() {
                    if i < self.ocr_list.items.len() - 1 {
                        self.ocr_list.state.select_next();
                    }
                }
            }
            _ => {}
        }
    }

    pub fn perform_ocr(&mut self) {
        let img = Image::from_path(&self.ocr_file).expect("Failed to load image for OCR");

        let args = Args {
            lang: "deu".to_string(),
            config_variables: HashMap::from([(
                "tessedit_char_whitelist".into(),
                "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZöäüÖÄÜß1234567890., &-%$@€:"
                    .into(),
            )]),
            dpi: Some(150),
            psm: Some(6),
            oem: Some(3),
        };

        let ocr_text =
            rusty_tesseract::image_to_string(&img, &args).expect("Could not perform OCR");

        self.ocr_list.items = ocr_text
            .split('\n')
            .map(|line| line.trim().to_string())
            .filter(|line| line.len() > 1)
            .map(|line| {
                // delete the last element, when it's a single character
                let re = Regex::new(r" \w$").expect("Could not compile regex");
                if let Some(found) = re.find(&line) {
                    line[..found.start()].to_string()
                } else {
                    line.to_string()
                }
            })
            .filter(|line| {
                // the last element of the line must contain a digit
                let elems = line.split(" ").collect::<Vec<&str>>();
                let re = Regex::new(r"\d").expect("Could not compile regex");
                re.is_match(elems[elems.len() - 1])
            })
            .filter(|line| {
                // the line must contain some sort of delimiter
                let re = Regex::new(r"[,.:-]").expect("Could not compile regex");
                re.is_match(line)
            })
            .filter(|line| !self.ocr_blacklist.iter().any(|elem| line.contains(elem)))
            .collect::<Vec<String>>();

        if !self.ocr_list.items.is_empty() {
            self.ocr_list.state.select_first();
        }
    }

    fn previous_item(&mut self) {
        match self.current_state {
            AppState::Home => {
                if let Some(i) = self.bon_list.state.selected() {
                    if i > 0 {
                        self.bon_list.state.select_previous();
                        self.events.send(AppEvent::CalculateSummary);
                    }
                }
            }
            AppState::Import => {
                if let Some(i) = self.import_list.state.selected() {
                    if i > 0 {
                        self.import_list.state.select_previous();
                    }
                }
            }
            AppState::OCR => {
                if let Some(i) = self.ocr_list.state.selected() {
                    if i > 0 {
                        self.ocr_list.state.select_previous();
                    }
                }
            }
            _ => {}
        }
    }

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        if !self.bon_list.items.is_empty() {
            self.bon_list.state.select_first();
            self.events.send(AppEvent::CalculateSummary);
        }
        if !self.import_list.items.is_empty() {
            self.import_list.state.select_first();
        }
        if !self.ocr_list.items.is_empty() {
            self.ocr_list.state.select_first();
        }
        while self.running {
            terminal.draw(|frame| frame.render_widget(&mut self, frame.area()))?;
            match self.events.next().await? {
                Event::Tick => self.tick(),
                Event::Crossterm(event) => {
                    if let crossterm::event::Event::Key(key_event) = event {
                        self.handle_key_events(key_event)?
                    }
                }
                Event::App(app_event) => match app_event {
                    AppEvent::CalculateSummary => self.calculate_summary(),
                    AppEvent::GoBlacklistState => self.go_blacklist_state(),
                    AppEvent::GoHomeState => self.go_home_state(),
                    AppEvent::GoImportState => self.go_import_state(),
                    AppEvent::GoOcrState => self.go_ocr_state(),
                    AppEvent::NextItem => self.next_item(),
                    AppEvent::PerformOCR => self.perform_ocr(),
                    AppEvent::PreviousItem => self.previous_item(),
                    AppEvent::UpdateFromDatabase => self.update_from_database(),
                    AppEvent::Quit => self.quit(),
                },
            }
        }
        Ok(())
    }

    pub fn update_from_database(&mut self) {
        if matches!(self.current_state, AppState::OCR) {
            self.ocr_blacklist = self.database.get_blacklist();
            let ocr_list = self.ocr_list.items.clone();
            self.ocr_list.items = ocr_list
                .into_iter()
                .filter(|line| !self.ocr_blacklist.iter().any(|elem| line.contains(elem)))
                .collect::<Vec<String>>();
        }
    }

    pub fn tick(&self) {}

    pub fn quit(&mut self) {
        self.running = false;
    }
}
