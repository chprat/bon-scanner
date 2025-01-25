use config::Config;
use std::path::Path;

pub struct Settings {
    pub settings_file: String,
    pub database_file: String,
}

impl Default for Settings {
    fn default() -> Self {
        let mut settings = Self {
            settings_file: Self::build_default_settings_path(),
            database_file: "".to_string(),
        };
        settings.database_file = settings.database_path();
        settings
    }
}

impl Settings {
    fn build_default_database_path() -> String {
        let home = dirs::home_dir().expect("Couldn't detect home folder");
        let home_dir = Path::new(&home);
        home_dir
            .join(".bon-scanner.sqlite")
            .to_str()
            .expect("Couldn't convert path to string")
            .to_string()
    }

    fn build_default_settings_path() -> String {
        let home = dirs::home_dir().expect("Couldn't detect home folder");
        let home_dir = Path::new(&home);
        home_dir
            .join(".bon-scanner.toml")
            .to_str()
            .expect("Couldn't convert path to string")
            .to_string()
    }

    pub fn database_exists(&self) -> bool {
        let database = Path::new(&self.database_file);
        database.exists()
    }

    fn database_path(&self) -> String {
        let mut ret = Self::build_default_database_path();
        if self.settings_exists() {
            let settings = Config::builder()
                .add_source(config::File::with_name(&self.settings_file))
                .build()
                .expect("Couldn't build settings file");
            if let Ok(database) = settings.get_string("database") {
                ret = database;
            }
        }
        ret
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn settings_exists(&self) -> bool {
        let settings = Path::new(&self.settings_file);
        settings.exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn nonexistent_config() {
        let mut settings = Settings::new();
        settings.settings_file = "noconfig.toml".to_string();
        assert!(!settings.settings_exists())
    }

    #[test]
    fn existent_config() {
        let mut settings = Settings::new();
        let cur_dir = env::current_dir().expect("Couldn't get current directory");
        settings.settings_file = cur_dir
            .join("config/bon-scanner.toml")
            .to_str()
            .expect("Couldn't build settings file")
            .to_string();
        assert!(settings.settings_exists())
    }

    #[test]
    fn nonexistent_database() {
        let mut settings = Settings::new();
        settings.database_file = "nodatabase.sqlite".to_string();
        assert!(!settings.database_exists())
    }

    #[test]
    fn existent_database() {
        let mut settings = Settings::new();
        let cur_dir = env::current_dir().expect("Couldn't get current directory");
        settings.database_file = cur_dir
            .join("config/bon-scanner.sqlite")
            .to_str()
            .expect("Couldn't build settings file")
            .to_string();
        assert!(settings.database_exists())
    }

    #[test]
    fn read_config() {
        let mut settings = Settings::new();
        let cur_dir = env::current_dir().expect("Couldn't get current directory");
        settings.settings_file = cur_dir
            .join("config/bon-scanner.toml")
            .to_str()
            .expect("Couldn't build settings file")
            .to_string();
        settings.database_file = settings.database_path();
        assert_eq!(settings.database_file, "config/bon-scanner.sqlite");
    }
}
