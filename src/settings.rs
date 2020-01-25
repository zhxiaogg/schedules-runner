use config::{Config, Environment};
use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Settings {
    pub server: String,
    pub token: String,
    pub logs: String,
}

impl Settings {
    pub fn new(token: Option<String>) -> Result<Self, String> {
        let mut config = Config::new();
        config
            .merge(config::File::with_name("settings"))
            .unwrap()
            .merge(Environment::with_prefix("SCHEDULES_RUNNER"))
            .unwrap();
        if let Some(t) = token {
            config.set("token", t).unwrap();
        }
        match config.get_str("token") {
            Ok(t) if t.trim().is_empty() => Err("no token found!".to_string()),
            Err(e) => Err("no token found!".to_string()),
            _ => config.try_into().map_err(|s| "invalid config".to_string()),
        }
    }
}
