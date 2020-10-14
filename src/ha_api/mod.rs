use crate::config::YamlConfig;
use platform_info::{PlatformInfo, Uname};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error;

type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize, Deserialize, Debug)]
pub struct HaEntityAttribute {
    pub friendly_name: Option<String>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct HaEntityState {
    pub attributes: HaEntityAttribute,
}

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
    supports_encryption: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterDeviceResponse {
    pub cloud_hook_url: Option<String>,
    pub remote_ui_url: Option<String>,
    pub secret: Option<String>,
    pub webhook_id: String,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct GetAccessTokenRequest {
    grant_type: String,
    code: String,
    client_id: String,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct GetAccessTokenResponse {
    pub access_token: String,
    pub expires_in: u32,
    pub refresh_token: String,
    pub token_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetAccessTokenError {
    error: String,
    error_description: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SensorRegistrationRequest {
    pub r#type: String,
    pub data: SensorRegistrationData,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SensorRegistrationData {
    pub device_class: Option<String>,
    pub icon: String,
    pub name: String,
    pub state: String,
    pub r#type: String,
    pub unique_id: String,
    pub unit_of_measurement: String,
    pub attributes: std::collections::HashMap<String, String>,
}

pub async fn get_access_token(config: &YamlConfig, code: String) -> Result<GetAccessTokenResponse> {
    let request = GetAccessTokenRequest {
        grant_type: "authorization_code".to_string(),
        code,
        client_id: "http://localhost:8000".to_string(),
    };
    let resp = Client::new()
        .post(format!("http://{}/auth/token", config.ha.host).as_str())
        .form(&request)
        .send()
        .await?;

    match resp.status().as_str() {
        "200" => Ok(resp.json::<GetAccessTokenResponse>().await?),
        _ => {
            let error = resp.json::<GetAccessTokenError>().await?;
            Err(format!(
                "Error getting access token from HA Error: {} Details: {}",
                error.error, error.error_description
            )
            .into())
        }
    }
}

pub async fn get_api_states(config: &YamlConfig) -> Result<Vec<HaEntityState>> {
    let endpoint = format!("http://{}/api/states", config.ha.host);
    let long_lived_token = config
        .ha
        .long_lived_token
        .as_deref()
        .ok_or_else(|| "expected long lived token to exist")?;
    let resp = Client::new()
        .get(endpoint.as_str())
        .header("Authorization", format!("Bearer {}", long_lived_token))
        .send()
        .await?;

    let api_states = resp.json::<Vec<HaEntityState>>().await?;
    Ok(api_states)
}

pub async fn register_machine(
    config: &YamlConfig,
    platform_info: &PlatformInfo,
) -> Result<RegisterDeviceResponse> {
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
        supports_encryption: false,
    };

    let endpoint = format!("http://{}/api/mobile_app/registrations", config.ha.host);
    let resp = Client::new()
        .post(endpoint.as_str())
        .header(
            "Authorization",
            format!("Bearer {}", config.ha.long_lived_token.as_ref().unwrap()),
        )
        .json(&request)
        .send()
        .await?;

    let r: RegisterDeviceResponse = resp.json().await?;
    println!("{:?}", r);
    Ok(r)
}

pub async fn register_sensor(
    request: &SensorRegistrationRequest,
    webhook_id: &str,
    host_url: &str,
    access_token: &str
) -> Result<()> {
    let endpoint = format!("http://{}/api/webhook/{}", host_url, webhook_id);

    let response = Client::new()
        .post(endpoint.as_str())
        .header(
            "Authorization",
            format!("Bearer {}", access_token),
        )
        .json(&request)
        .send()
        .await?;

    let resp_json =  response.text().await?;
    println!("Register sensor response {}", resp_json);
    Ok(())
}
