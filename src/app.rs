use crate::database;
use crate::event::{AppEvent, Event, EventHandler};
use crate::settings;
use ratatui::{
    DefaultTerminal,
    crossterm::event::{KeyCode, KeyEvent},
    widgets::ListState,
};
use std::collections::HashMap;
use std::fs;

pub struct App {
    pub bon_list: BonList,
    pub bon_summary: Vec<SummaryEntry>,
    pub current_state: AppState,
    events: EventHandler,
    pub import_list: FileList,
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

pub enum AppState {
    Home,
    Import,
}

pub struct SummaryEntry {
    pub category: String,
    pub total: f64,
}

impl Default for App {
    fn default() -> Self {
        let settings = settings::Settings::new();
        let database_exists = settings.database_exists();
        let database = database::Database::new(&settings.database_file);
        if !database_exists {
            database.create_database();
        }
        let bons = database.get_bons();
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
            events: EventHandler::new(),
            import_list: FileList {
                items: import_list,
                state: ListState::default(),
            },
            running: true,
        }
    }
}

impl App {
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
        match key_event.code {
            KeyCode::Char('i') => self.events.send(AppEvent::GoImportState),
            KeyCode::Char('j') => self.events.send(AppEvent::NextItem),
            KeyCode::Char('k') => self.events.send(AppEvent::PreviousItem),
            KeyCode::Char('q') => self.events.send(AppEvent::Quit),
            _ => {}
        }
        Ok(())
    }

    fn go_home_state(&mut self) {
        self.current_state = AppState::Home;
    }

    fn go_import_state(&mut self) {
        self.current_state = AppState::Import;
    }

    pub fn new() -> Self {
        Self::default()
    }

    fn next_item(&mut self) {
        if let Some(i) = self.bon_list.state.selected() {
            if i < self.bon_list.items.len() - 1 {
                self.bon_list.state.select_next();
                self.events.send(AppEvent::CalculateSummary);
            }
        }
    }

    fn previous_item(&mut self) {
        if let Some(i) = self.bon_list.state.selected() {
            if i > 0 {
                self.bon_list.state.select_previous();
                self.events.send(AppEvent::CalculateSummary);
            }
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
                    AppEvent::GoHomeState => self.go_home_state(),
                    AppEvent::GoImportState => self.go_import_state(),
                    AppEvent::NextItem => self.next_item(),
                    AppEvent::PreviousItem => self.previous_item(),
                    AppEvent::Quit => self.quit(),
                },
            }
        }
        Ok(())
    }

    pub fn tick(&self) {}

    pub fn quit(&mut self) {
        match self.current_state {
            AppState::Home => {
                self.running = false;
            }
            AppState::Import => {
                self.events.send(AppEvent::GoHomeState);
            }
        }
    }
}
