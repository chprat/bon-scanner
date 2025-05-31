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
            KeyCode::Char('q') => self.events.send(AppEvent::Quit),
            _ => {}
        }
        Ok(())
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
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
