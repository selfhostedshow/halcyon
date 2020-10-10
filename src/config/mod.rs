

use serde::{Deserialize, Serialize};
use serde_yaml;
use std::error;
use std::io::Write;
use uuid;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HaConfig {
    pub host: String,
    pub token: String,
    #[serde(rename = "device-id")]
    pub device_id: Option<String>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct YamlConfig {
    pub ha: HaConfig
}
type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

impl YamlConfig {
    pub fn update_device_id_if_needed(self, file_name: &str) -> Result<Self> {
        let new_config = match self.ha.device_id {
            None => {
                let ha_config = HaConfig {
                    device_id: Some(Uuid::new_v4().to_string()),
                    ..self.ha
                };
                let config = YamlConfig {
                    ha: ha_config,
                    ..self
                };
                let new_config_str = serde_yaml::to_string(&config)?;
                let mut f = std::fs::OpenOptions::new().write(true).truncate(true).open(file_name)?;
                f.write_all(new_config_str.as_bytes())?;
                println!("no device-id found in config, so we are making one for you");
                config
            },
            Some(_) => self
        };
        Ok(new_config)
    }
}
pub fn read_config_yml(file_name: &str) -> Result<YamlConfig> {
    let f = std::fs::File::open(file_name)?;
    let config: YamlConfig = serde_yaml::from_reader(f)?;
    Ok(config)
}