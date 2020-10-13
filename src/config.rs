use serde::{Serialize, Deserialize};


#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    connection: Option<>
}

pub struct  {
    cloudhook_url: String,
    remote_ui_url: String,
    webhook_id: String,
    instance_urls: Vec<String>,
}

