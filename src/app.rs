use crate::database;
use crate::event::{AppEvent, Event, EventHandler};
use crate::settings;
use ratatui::{
    DefaultTerminal,
    crossterm::event::{KeyCode, KeyEvent},
    widgets::ListState,
};

pub struct App {
    pub bon_list: BonList,
    events: EventHandler,
    running: bool,
}

pub struct BonList {
    pub items: Vec<database::Bon>,
    pub state: ListState,
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
        Self {
            bon_list: BonList {
                items: bons,
                state: ListState::default(),
            },
            events: EventHandler::new(),
            running: true,
        }
    }
}

impl App {
    pub fn handle_key_events(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Char('j') => self.events.send(AppEvent::NextItem),
            KeyCode::Char('k') => self.events.send(AppEvent::PreviousItem),
            KeyCode::Char('q') => self.events.send(AppEvent::Quit),
            _ => {}
        }
        Ok(())
    }

    pub fn new() -> Self {
        Self::default()
    }

    fn next_item(&mut self) {
        if let Some(i) = self.bon_list.state.selected() {
            if i < self.bon_list.items.len() - 1 {
                self.bon_list.state.select_next();
            }
        }
    }

    fn previous_item(&mut self) {
        if let Some(i) = self.bon_list.state.selected() {
            if i > 0 {
                self.bon_list.state.select_previous();
            }
        }
    }

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        if !self.bon_list.items.is_empty() {
            self.bon_list.state.select_first();
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
        self.running = false;
    }
}
