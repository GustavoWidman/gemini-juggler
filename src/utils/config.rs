use std::{path::PathBuf, sync::Arc};

use easy_config_store::ConfigStore;
use eyre::Result;
use serde::{Deserialize, Serialize};

pub type Config = Arc<ConfigStore<ConfigInner>>;
pub fn config(path: PathBuf) -> Result<Config> {
    Ok(ConfigStore::<ConfigInner>::read(path, "config".to_string())?.arc())
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
        let cfg = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config.default.toml",));

        toml::from_str(cfg).unwrap() // should be okay
    }
}
