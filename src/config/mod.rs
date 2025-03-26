use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Config {
    pub config: ConfigInner,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ConfigInner {
    pub api_key: String,
    pub keys: Vec<String>,
    pub host: String,
    pub port: u16,
}

impl Default for ConfigInner {
    fn default() -> Self {
        log::info!("Created default config, please edit it.");
        Self {
            api_key: "password".into(),
            keys: vec![],
            host: "0.0.0.0".into(),
            port: 8080,
        }
    }
}

impl Deref for Config {
    type Target = ConfigInner;

    fn deref(&self) -> &Self::Target {
        &self.config
    }
}
impl DerefMut for Config {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.config
    }
}
