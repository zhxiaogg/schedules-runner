use config::{Config, ConfigError, Environment};
use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Settings {
    pub server: String,
    pub token: String,
    pub logs: String,
}

impl Settings {
    pub fn new(token: Option<String>) -> Result<Self, ConfigError> {
        let mut config = Config::new();
        config
            .merge(config::File::with_name("settings"))
            .unwrap()
            .merge(Environment::with_prefix("SCHEDULES_RUNNER"))
            .unwrap();
        if let Some(t) = token {
            config.set("token", t).unwrap();
        }
        config.get_str("token")?;
        config.try_into()
    }
}
