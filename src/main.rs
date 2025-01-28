pub mod database;
pub mod settings;

fn main() {
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
}
