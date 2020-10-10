use crate::config::YamlConfig;
use either::*;
use platform_info::{PlatformInfo, Uname};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error;

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
    supports_encryption: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterDeviceResponse {
    cloud_hook_url: Option<String>,
    remote_ui_url: Option<String>,
    secret: Option<String>,
    webhook_id: String,
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

pub async fn get_access_token(
    config: &YamlConfig,
    code: String,
) -> Result<Either<GetAccessTokenError, GetAccessTokenResponse>> {
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

    let either = match resp.status().as_str() {
        "200" => Right(resp.json::<GetAccessTokenResponse>().await?),
        _ => Left(resp.json::<GetAccessTokenError>().await?),
    };
    Ok(either)
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
