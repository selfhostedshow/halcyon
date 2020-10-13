use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct RefreshToken {
    pub access_token: String,
    pub expires_in: u64,
    pub token_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthorizationCode {
    pub access_token: String,
    pub expires_in: u64,
    pub refresh_token: String,
    pub token_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeviceRegistrationRequest {
    pub device_id: String,
    pub app_id: String,
    pub app_name: String,
    pub app_version: String,
    pub device_name: String,
    pub manufacturer: String,
    pub model: String,
    pub os_name: String,
    pub os_version: String,
    pub supports_encryption: bool,
    pub app_data: AppData,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AppData {}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeviceRegistrationResponse {
    pub cloudhook_url: Option<String>,
    pub remote_ui_url: Option<String>,
    pub secret: Option<String>,
    pub webhook_id: String,
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

#[derive(Serialize, Deserialize, Debug)]
pub struct SensorUpdateRequest {
    pub r#type: String,
    pub data: SensorUpdateData,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SensorUpdateData {
    pub icon: String,
    pub state: String,
    pub r#type: String,
    pub unique_id: String,
    pub attributes: std::collections::HashMap<String, String>,
}
