pub mod settings;

fn main() {
    let settings = settings::Settings::new();
    if !settings.settings_exists() {
        println!(
            "Settings file {} does not exist, using defaults",
            &settings.settings_file
        );
    }
}
