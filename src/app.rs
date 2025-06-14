use crate::database;
use crate::event::{AppEvent, Event, EventHandler};
use crate::settings;
use float_cmp::{ApproxEq, F64Margin};
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
use textdistance::str::damerau_levenshtein;
use tui_textarea::{CursorMove, TextArea};

pub struct App<'a> {
    pub bon_list: BonList,
    pub bon_summary: Vec<SummaryEntry>,
    pub category_list: CategoryList,
    pub current_state: AppState,
    database: database::Database,
    pub edit_field: TextArea<'a>,
    events: EventHandler,
    pub import_list: FileList,
    import_path: String,
    pub new_bon_list: NewBonList,
    ocr_blacklist: Vec<String>,
    pub ocr_list: OcrList,
    pub ocr_file: String,
    running: bool,
}

pub struct BonList {
    pub items: Vec<database::Bon>,
    pub state: ListState,
}

pub struct CategoryList {
    pub items: Vec<database::Category>,
    pub state: ListState,
}

pub struct FileList {
    pub items: Vec<String>,
    pub state: ListState,
}

pub struct NewBonList {
    pub date: String,
    pub items: Vec<database::Entry>,
    pub price_calc: f64,
    pub price_eq: bool,
    pub price_ocr: f64,
    pub state: ListState,
}

#[derive(Clone)]
pub struct OcrEntry {
    pub name: String,
    pub ocr_type: OcrType,
}

pub struct OcrList {
    pub items: Vec<OcrEntry>,
    pub state: ListState,
}

pub enum AppState {
    Blacklist,
    Category,
    ConvertBon,
    EditBonPrice,
    EditCategory,
    EditName,
    EditPrice,
    Home,
    Import,
    OCR,
}

#[derive(Clone)]
pub enum OcrType {
    Date,
    Entry,
    Sum,
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
        let processed = database.get_processed();
        let import_list = read_ocr_files(&processed);
        let category_list = database.get_categories();
        Self {
            bon_list: BonList {
                items: bons,
                state: ListState::default(),
            },
            category_list: CategoryList {
                items: category_list,
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
            new_bon_list: NewBonList {
                date: String::new(),
                items: Vec::new(),
                price_calc: 0.0,
                price_eq: false,
                price_ocr: 0.0,
                state: ListState::default(),
            },
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
        if matches!(self.current_state, AppState::Home) {
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
        } else if matches!(self.current_state, AppState::ConvertBon)
            | matches!(self.current_state, AppState::EditPrice)
        {
            self.new_bon_list.price_calc = self
                .new_bon_list
                .items
                .iter()
                .map(|entry| entry.price)
                .sum();
            self.new_bon_list.price_eq = self.new_bon_list.price_ocr.approx_eq(
                self.new_bon_list.price_calc,
                F64Margin {
                    ulps: 2,
                    epsilon: 1.0,
                },
            );
        }
    }

    fn convert_to_bon(&mut self) {
        self.new_bon_list.date = String::new();
        self.new_bon_list.items.clear();
        self.new_bon_list.price_calc = 0.0;
        self.new_bon_list.price_ocr = 0.0;

        self.ocr_list
            .items
            .iter()
            .for_each(|elem| match elem.ocr_type {
                OcrType::Date => {
                    if let Some(date) = Self::extract_date(&elem.name) {
                        self.new_bon_list.date = date;
                    }
                }
                OcrType::Entry => {
                    if let Some(name) = Self::extract_name(&elem.name) {
                        if let Some(price) = Self::extract_price(&elem.name) {
                            let db_products = self.database.get_products();
                            let db_product = db_products
                                .iter()
                                .min_by_key(|elem| damerau_levenshtein(&name, &elem.product));
                            let distance = if let Some(product) = &db_product {
                                damerau_levenshtein(&name, &product.product)
                            } else {
                                usize::MAX
                            };
                            let mut category = String::new();
                            let mut product = name;
                            if distance < 4 {
                                let db_product = db_product.unwrap();
                                product = db_product.product.clone();
                                let db_categories = self.database.get_categories();
                                category = db_categories
                                    .iter()
                                    .find(|category| category.category_id == db_product.category_id)
                                    .map(|category| category.category.clone())
                                    .unwrap_or_else(|| "".to_string());
                            }
                            self.new_bon_list.items.push(database::Entry {
                                category,
                                product,
                                price,
                            });
                        }
                    }
                }
                OcrType::Sum => {
                    if let Some(sum) = Self::extract_price(&elem.name) {
                        self.new_bon_list.price_ocr = sum;
                    }
                }
            });

        if !self.new_bon_list.items.is_empty() {
            self.new_bon_list.state.select_first();
        }
        self.events.send(AppEvent::GoConvertBonState);
        self.events.send(AppEvent::CalculateSummary);
    }

    fn extract_date(line: &str) -> Option<String> {
        let re = Regex::new(r"\d{2}[\.,]\d{2}[\.,]\d{4}").expect("Could not compile regex");
        re.find(line).map(|m| m.as_str().to_string())
    }

    fn extract_name(line: &str) -> Option<String> {
        let name = line.rsplit_once(' ');
        name.map(|name| name.0.to_string())
    }

    fn extract_price(line: &str) -> Option<f64> {
        let re = Regex::new(r"(\d+[.,]\d+)").expect("Could not compile regex");
        re.find(line)
            .and_then(|m| m.as_str().replace(',', ".").parse::<f64>().ok())
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
        } else if matches!(self.current_state, AppState::EditBonPrice)
            | matches!(self.current_state, AppState::EditName)
            | matches!(self.current_state, AppState::EditPrice)
        {
            match key_event.code {
                KeyCode::Enter => {
                    match self.current_state {
                        AppState::EditBonPrice => {
                            self.new_bon_list.price_ocr = self
                                .edit_field
                                .lines()
                                .first()
                                .and_then(|line| {
                                    let repl = line.replace(",", ".");
                                    repl.parse::<f64>().ok()
                                })
                                .unwrap_or(0.0);
                        }
                        AppState::EditCategory => {
                            if let Some(i) = self.new_bon_list.state.selected() {
                                if let Some(entry) = self.new_bon_list.items.get_mut(i) {
                                    entry.category = self.edit_field.lines()[0].clone();
                                }
                            }
                        }
                        AppState::EditName => {
                            if let Some(i) = self.new_bon_list.state.selected() {
                                if let Some(entry) = self.new_bon_list.items.get_mut(i) {
                                    entry.product = self.edit_field.lines()[0].clone();
                                }
                            }
                        }
                        AppState::EditPrice => {
                            if let Some(i) = self.new_bon_list.state.selected() {
                                if let Some(entry) = self.new_bon_list.items.get_mut(i) {
                                    entry.price = self
                                        .edit_field
                                        .lines()
                                        .first()
                                        .and_then(|line| {
                                            let repl = line.replace(",", ".");
                                            repl.parse::<f64>().ok()
                                        })
                                        .unwrap_or(0.0);
                                }
                            }
                        }
                        _ => {}
                    }
                    self.events.send(AppEvent::GoConvertBonState);
                    self.events.send(AppEvent::CalculateSummary);
                }
                KeyCode::Esc => self.events.send(AppEvent::GoConvertBonState),
                _ => _ = self.edit_field.input(key_event),
            }
        } else if matches!(self.current_state, AppState::EditCategory) {
            match key_event.code {
                KeyCode::Enter => {
                    let category = self.edit_field.lines()[0].as_str();
                    let category_exists = self
                        .category_list
                        .items
                        .iter()
                        .any(|elem| elem.category == category);
                    if !category_exists {
                        self.database
                            .create_category(self.edit_field.lines()[0].as_str());
                    }
                    self.events.send(AppEvent::GoCategoryState);
                    self.events.send(AppEvent::UpdateFromDatabase);
                }
                KeyCode::Esc => self.events.send(AppEvent::GoCategoryState),
                _ => _ = self.edit_field.input(key_event),
            }
        } else {
            match key_event.code {
                KeyCode::Char('a') => {
                    if matches!(self.current_state, AppState::Category) {
                        self.edit_field.move_cursor(CursorMove::End);
                        self.edit_field.delete_line_by_head();
                        self.events.send(AppEvent::GoEditCategoryState);
                    }
                }
                KeyCode::Char('b') => {
                    if matches!(self.current_state, AppState::OCR) {
                        self.edit_field.move_cursor(CursorMove::End);
                        self.edit_field.delete_line_by_head();
                        if let Some(i) = self.ocr_list.state.selected() {
                            self.edit_field
                                .insert_str(self.ocr_list.items[i].name.as_str());
                        }
                        self.events.send(AppEvent::GoBlacklistState);
                    }
                }
                KeyCode::Char('c') => self.events.send(AppEvent::GoCategoryState),
                KeyCode::Char('d') => self.events.send(AppEvent::OcrMarkDate),
                KeyCode::Char('h') => self.events.send(AppEvent::HideItem),
                KeyCode::Char('i') => self.events.send(AppEvent::GoImportState),
                KeyCode::Char('j') => self.events.send(AppEvent::NextItem),
                KeyCode::Char('k') => self.events.send(AppEvent::PreviousItem),
                KeyCode::Char('n') => {
                    self.edit_field.move_cursor(CursorMove::End);
                    self.edit_field.delete_line_by_head();
                    if let Some(i) = self.new_bon_list.state.selected() {
                        self.edit_field
                            .insert_str(self.new_bon_list.items[i].product.as_str());
                    }
                    self.events.send(AppEvent::GoEditNameState);
                }
                KeyCode::Char('o') => {
                    self.edit_field.move_cursor(CursorMove::End);
                    self.edit_field.delete_line_by_head();
                    self.edit_field
                        .insert_str(self.new_bon_list.price_ocr.to_string());
                    self.events.send(AppEvent::GoEditBonPriceState);
                }
                KeyCode::Char('p') => {
                    self.edit_field.move_cursor(CursorMove::End);
                    self.edit_field.delete_line_by_head();
                    if let Some(i) = self.new_bon_list.state.selected() {
                        self.edit_field
                            .insert_str(self.new_bon_list.items[i].price.to_string());
                    }
                    self.events.send(AppEvent::GoEditPriceState);
                }
                KeyCode::Char('q') => self.events.send(AppEvent::Quit),
                KeyCode::Char('s') => self.events.send(AppEvent::OcrMarkSum),
                KeyCode::Char('x') => {
                    if matches!(self.current_state, AppState::OCR) {
                        if let Some(i) = self.ocr_list.state.selected() {
                            self.ocr_list.items.remove(i);
                        }
                    } else if matches!(self.current_state, AppState::ConvertBon) {
                        if let Some(i) = self.new_bon_list.state.selected() {
                            self.new_bon_list.items.remove(i);
                        }
                        self.events.send(AppEvent::CalculateSummary);
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
                    } else if matches!(self.current_state, AppState::OCR) {
                        self.events.send(AppEvent::ConvertToBon);
                    } else if matches!(self.current_state, AppState::ConvertBon) {
                        self.events.send(AppEvent::ImportBon);
                    } else if matches!(self.current_state, AppState::Category) {
                        if let Some(i) = self.category_list.state.selected() {
                            if let Some(category) = self.category_list.items.get(i) {
                                if let Some(j) = self.new_bon_list.state.selected() {
                                    if let Some(item) = self.new_bon_list.items.get_mut(j) {
                                        item.category = category.category.clone();
                                    }
                                }
                            }
                        }
                        self.events.send(AppEvent::GoConvertBonState);
                    }
                }
                KeyCode::Esc => {
                    if matches!(self.current_state, AppState::Category) {
                        self.events.send(AppEvent::GoConvertBonState)
                    } else {
                        self.events.send(AppEvent::GoHomeState)
                    };
                }
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

    fn go_category_state(&mut self) {
        if matches!(self.current_state, AppState::ConvertBon)
            | matches!(self.current_state, AppState::EditCategory)
        {
            if !self.category_list.items.is_empty() {
                self.category_list.state.select_first();
            }
            self.current_state = AppState::Category;
        }
    }

    fn go_convert_bon_state(&mut self) {
        self.current_state = AppState::ConvertBon;
    }

    fn go_edit_bon_price_state(&mut self) {
        if matches!(self.current_state, AppState::ConvertBon) {
            self.current_state = AppState::EditBonPrice;
        }
    }

    fn go_edit_category_state(&mut self) {
        if matches!(self.current_state, AppState::Category) {
            self.current_state = AppState::EditCategory;
        }
    }

    fn go_edit_name_state(&mut self) {
        if matches!(self.current_state, AppState::ConvertBon) {
            self.current_state = AppState::EditName;
        }
    }

    fn go_edit_price_state(&mut self) {
        if matches!(self.current_state, AppState::ConvertBon) {
            self.current_state = AppState::EditPrice;
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
            self.ocr_list.items = vec![OcrEntry {
                name: "Processing..".to_string(),
                ocr_type: OcrType::Entry,
            }];
            self.events.send(AppEvent::PerformOCR);
        }
    }

    fn hide_item(&mut self) {
        if matches!(self.current_state, AppState::Home) {
            if let Some(i) = self.bon_list.state.selected() {
                if let Some(entry) = self.bon_list.items.get(i) {
                    self.database.hide_bon(entry.bon_id);
                    self.events.send(AppEvent::UpdateFromDatabase);
                }
            }
        }
    }

    fn import_bon(&mut self) {
        let mut split = self.new_bon_list.date.split(".").collect::<Vec<&str>>();
        split.reverse();
        let date = split.join("-");
        self.database
            .create_bon(date.as_str(), self.new_bon_list.price_ocr);
        let bon_id = self.database.get_last_bon_id();
        self.new_bon_list.items.iter().for_each(|entry| {
            let categories = self.database.get_categories();
            let category_id = categories
                .iter()
                .find(|cat| cat.category == entry.category)
                .map_or_else(
                    || {
                        self.database.create_category(entry.category.as_str());
                        self.database.get_last_category_id()
                    },
                    |cat| cat.category_id,
                );
            let products = self.database.get_products();
            let product_id = products
                .iter()
                .find(|prod| prod.product == entry.product)
                .map_or_else(
                    || {
                        self.database
                            .create_product(category_id, entry.product.as_str());
                        self.database.get_last_product_id()
                    },
                    |cat| cat.category_id,
                );
            self.database.create_entry(bon_id, product_id, entry.price);
        });
        let ocr_file = self.ocr_file.clone();
        self.ocr_file = String::new();
        let file_name = Path::new(&ocr_file)
            .file_name()
            .expect("Could not get file name")
            .to_str()
            .expect("Could not convert file name to string");
        self.database.add_processed_entry(file_name);
        self.events.send(AppEvent::GoHomeState);
        self.events.send(AppEvent::UpdateFromDatabase);
        self.events.send(AppEvent::CalculateSummary);
    }

    pub fn new() -> Self {
        Self::default()
    }

    fn next_item(&mut self) {
        match self.current_state {
            AppState::Category => {
                if let Some(i) = self.category_list.state.selected() {
                    if i < self.category_list.items.len() - 1 {
                        self.category_list.state.select_next();
                    }
                }
            }
            AppState::ConvertBon => {
                if let Some(i) = self.new_bon_list.state.selected() {
                    if i < self.new_bon_list.items.len() - 1 {
                        self.new_bon_list.state.select_next();
                    }
                }
            }
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

    pub fn ocr_mark_date(&mut self) {
        let dates = self
            .ocr_list
            .items
            .iter()
            .filter(|elem| matches!(elem.ocr_type, OcrType::Date))
            .count();
        if let Some(i) = self.ocr_list.state.selected() {
            if let Some(entry) = self.ocr_list.items.get_mut(i) {
                if dates == 0 && matches!(entry.ocr_type, OcrType::Entry) {
                    entry.ocr_type = OcrType::Date;
                } else if matches!(entry.ocr_type, OcrType::Date) {
                    entry.ocr_type = OcrType::Entry;
                }
            }
        }
    }

    pub fn ocr_mark_sum(&mut self) {
        let sums = self
            .ocr_list
            .items
            .iter()
            .filter(|elem| matches!(elem.ocr_type, OcrType::Sum))
            .count();
        if let Some(i) = self.ocr_list.state.selected() {
            if let Some(entry) = self.ocr_list.items.get_mut(i) {
                if sums == 0 && matches!(entry.ocr_type, OcrType::Entry) {
                    entry.ocr_type = OcrType::Sum;
                } else if matches!(entry.ocr_type, OcrType::Sum) {
                    entry.ocr_type = OcrType::Entry;
                }
            }
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
            .map(|line| OcrEntry {
                name: line,
                ocr_type: OcrType::Entry,
            })
            .collect::<Vec<OcrEntry>>();

        if !self.ocr_list.items.is_empty() {
            self.ocr_list.state.select_first();
        }
    }

    fn previous_item(&mut self) {
        match self.current_state {
            AppState::Category => {
                if let Some(i) = self.category_list.state.selected() {
                    if i > 0 {
                        self.category_list.state.select_previous();
                    }
                }
            }
            AppState::ConvertBon => {
                if let Some(i) = self.new_bon_list.state.selected() {
                    if i > 0 {
                        self.new_bon_list.state.select_previous();
                    }
                }
            }
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
        if !self.category_list.items.is_empty() {
            self.category_list.state.select_first();
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
                    AppEvent::ConvertToBon => self.convert_to_bon(),
                    AppEvent::GoBlacklistState => self.go_blacklist_state(),
                    AppEvent::GoCategoryState => self.go_category_state(),
                    AppEvent::GoConvertBonState => self.go_convert_bon_state(),
                    AppEvent::GoEditBonPriceState => self.go_edit_bon_price_state(),
                    AppEvent::GoEditCategoryState => self.go_edit_category_state(),
                    AppEvent::GoEditNameState => self.go_edit_name_state(),
                    AppEvent::GoEditPriceState => self.go_edit_price_state(),
                    AppEvent::GoHomeState => self.go_home_state(),
                    AppEvent::GoImportState => self.go_import_state(),
                    AppEvent::GoOcrState => self.go_ocr_state(),
                    AppEvent::HideItem => self.hide_item(),
                    AppEvent::ImportBon => self.import_bon(),
                    AppEvent::NextItem => self.next_item(),
                    AppEvent::PerformOCR => self.perform_ocr(),
                    AppEvent::PreviousItem => self.previous_item(),
                    AppEvent::OcrMarkDate => self.ocr_mark_date(),
                    AppEvent::OcrMarkSum => self.ocr_mark_sum(),
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
                .filter(|line| {
                    !self
                        .ocr_blacklist
                        .iter()
                        .any(|elem| line.name.contains(elem))
                })
                .collect::<Vec<OcrEntry>>();
        } else if matches!(self.current_state, AppState::Home) {
            self.bon_list.items = self.database.get_bons();
            if !self.bon_list.items.is_empty() {
                self.bon_list.state.select_first();
            }
            self.import_list.items = read_ocr_files(&self.database.get_processed());
            if !self.import_list.items.is_empty() {
                self.import_list.state.select_first();
            }
        } else if matches!(self.current_state, AppState::Category) {
            self.category_list.items = self.database.get_categories();
            if !self.category_list.items.is_empty() {
                self.category_list.state.select_first();
            }
        }
    }

    pub fn tick(&self) {}

    pub fn quit(&mut self) {
        self.running = false;
    }
}

fn read_ocr_files(processed: &[String]) -> Vec<String> {
    let settings = settings::Settings::new();
    fs::read_dir(settings.import_path())
        .expect("Couldn't read bons directory")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|entry| !processed.iter().any(|elem| entry.contains(elem)))
        .filter(|entry| entry.contains("jpg") || entry.contains("png") || entry.contains("jpeg"))
        .collect::<Vec<String>>()
}
