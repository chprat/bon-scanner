pub mod app;
pub mod database;
pub mod event;
pub mod settings;
pub mod ui;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    let settings = settings::Settings::new();
    if !settings.settings_exists() {
        println!(
            "Settings file {} does not exist, using defaults",
            &settings.settings_file
        );
    }
    if !settings.database_exists() {
        println!(
            "Database {} does not exist, creating it",
            &settings.database_file
        );
        let database = database::Database::new(&settings.database_file);
        database.create_database();
    }
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = app::App::new().run(terminal).await;
    ratatui::restore();
    result
}
