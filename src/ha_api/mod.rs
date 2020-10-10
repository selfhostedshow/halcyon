use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::error;
use platform_info::{PlatformInfo, Uname};
use crate::config::YamlConfig;


type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[derive(Serialize, Deserialize)]
struct RegisterDeviceRequest {
    device_id: String,
    app_id: String,
    app_name: String,
    app_version: String,
    device_name: String,
    manufacturer: String,
    model: String,
    os_name: String,
    os_version: String,
    supports_encryption: bool
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterDeviceResponse {
    cloud_hook_url: Option<String>,
    remote_ui_url: Option<String>,
    secret: Option<String>,
    webhook_id: String
}

pub async fn register_machine(config: &YamlConfig, platform_info: &PlatformInfo) -> Result<RegisterDeviceResponse> {
    let request = RegisterDeviceRequest {
        device_id: config.ha.device_id.as_ref().unwrap().to_string(),
        app_id: String::from("HalcyonAppId"),
        app_name: String::from("Halcyon"),
        app_version: String::from(VERSION),
        device_name: String::from(platform_info.nodename()),
        manufacturer: String::from("PC"),
        model: String::from(platform_info.machine()),
        os_name: String::from(platform_info.sysname()),
        os_version: String::from(platform_info.version()),
        supports_encryption: false
    };

    let resp = Client::new()
        .post(&config.ha.host)
        .header("Authorization", format!("Bearer {}", config.ha.token))
        .json(&request)
        .send().await?;

    let r: RegisterDeviceResponse = resp.json().await?;
    println!("{:?}", r);
    Ok(r)
}