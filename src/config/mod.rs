use serde::{Deserialize, Serialize};
use std::error;
use std::io::Write;
use uuid::Uuid;

use tiny_http::{Response, Server};

use crate::ha_api;
use crate::ha_api::{ GetAccessTokenResponse, RegisterDeviceResponse};
use std::collections::HashMap;
use url::Url;

use serde_json::Value;
use tungstenite::{connect, Message};

// HA creates tokens for 10 years so we do the same
const LONG_LIVED_TOKEN_VALID_FOR: u32 = 365;

const LONG_LIVED_TOKEN_WS_COMMAND_ID: u32 = 11;

const LOCAL_SERVER_HOST: &str = "0.0.0.0:8000";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HaConfig {
    pub host: String,
    #[serde(rename = "long-lived-token")]
    pub long_lived_token: Option<String>,
    #[serde(rename = "device-id")]
    pub device_id: Option<String>,
    #[serde(rename = "webhook-id")]
    pub webhook_id: Option<String>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct YamlConfig {
    pub ha: HaConfig,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WsAuthRequest {
    #[serde(rename(serialize = "type"))]
    auth_type: String,
    access_token: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WsLongLivedAccessTokenRequest {
    id: u32,
    #[serde(rename(serialize = "type"))]
    command_type: String,
    client_name: String,
    lifespan: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WsLongLivedAccessTokenResponse {
    id: u32,
    #[serde(rename(deserialize = "type"))]
    command_type: String,
    success: bool,
    result: Option<String>,
}

type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

pub async fn wait_for_token(config: &YamlConfig) -> Result<GetAccessTokenResponse> {
    let maybe_server = Server::http(LOCAL_SERVER_HOST);
    match maybe_server {
        Ok(server) => {
            println!("Open http://{}/auth/authorize?client_id=http%3A%2F%2Flocalhost%3A8000&redirect_uri=http%3A%2F%2Flocalhost%3A8000%2Fcallback in your browser", config.ha.host.as_str());
            // blocks until the next request is received
            let maybe_token_resp = match server.recv() {
                Ok(request) => {
                    let url = format!("http://{}{}", LOCAL_SERVER_HOST, request.url());
                    let query_params: HashMap<_, _> = Url::parse(url.as_str())
                        .unwrap()
                        .query_pairs()
                        .into_owned()
                        .collect();
                    match query_params.get("code") {
                        Some(code) => {
                            let access_token_resp_result =
                                ha_api::get_access_token(config, code.to_string()).await;
                            match access_token_resp_result {
                                Ok(resp) => {
                                    request.respond(Response::from_string(
                                        "Halcyon now authenticated to Home Assistant. You can close this page now.",
                                    ))?;
                                    Some(resp)
                                }
                                Err(e) => {
                                    request.respond(
                                        Response::from_string(
                                            format!("Something went wrong with authenticating Halcyon with Home Assistant {}", e),
                                        )
                                            .with_status_code(500),
                                    )?;
                                    None
                                }
                            }
                        }
                        None => {
                            request.respond(
                                Response::from_string(
                                    "Something went wrong with authenticating Halcyon with Home Assistant",
                                )
                                    .with_status_code(500),
                            )?;
                            None
                        }
                    }
                }
                Err(e) => {
                    println!("error: {}", e);
                    None
                }
            };
            maybe_token_resp.ok_or_else(|| "could not retrieve HA access token".into())
        }
        Err(e) => Err(format!("could not start server {}", e).into()),
    }
}

fn get_long_lived_token_from_ws(
    config: &YamlConfig,
    access_token: String,
) -> Result<WsLongLivedAccessTokenResponse> {
    let ws_url = format!("ws://{}/api/websocket", config.ha.host);
    let url = Url::parse(ws_url.as_str())?;
    let (mut socket, _) = connect(url)?;

    let mut maybe_long_lived_token_response: Option<WsLongLivedAccessTokenResponse> = None;
    loop {
        let msg = socket.read_message()?.into_text()?;
        let msg_as_json: Value = serde_json::from_str(msg.as_str())?;
        let maybe_response_type = msg_as_json
            .get("type")
            .and_then(|response_type| response_type.as_str());

        match maybe_response_type {
            Some(response_type) => match response_type {
                "auth_required" => {
                    let req = WsAuthRequest {
                        auth_type: "auth".to_string(),
                        access_token: access_token.clone(),
                    };
                    let req_as_str = Message::Text(serde_json::to_string(&req)?);
                    socket.write_message(req_as_str)?;
                }
                "auth_ok" => {
                    let req = WsLongLivedAccessTokenRequest {
                        id: LONG_LIVED_TOKEN_WS_COMMAND_ID,
                        command_type: "auth/long_lived_access_token".to_string(),
                        client_name: "Halcyon".to_string(),
                        lifespan: LONG_LIVED_TOKEN_VALID_FOR,
                    };

                    let req_as_str = Message::Text(serde_json::to_string(&req)?);
                    socket.write_message(req_as_str)?;
                }
                "result" => {
                    let response: WsLongLivedAccessTokenResponse =
                        serde_json::from_str(msg.as_str())?;
                    if response.success {
                        maybe_long_lived_token_response = Some(response);
                    }
                    break;
                }
                "auth_invalid" => {
                    let error_msg = msg_as_json
                        .get("message")
                        .and_then(|error| error.as_str())
                        .unwrap_or("");
                    println!("Error authorizing websocket {}", error_msg);
                    break;
                }
                _ => println!(
                    "Unexpected response from websocket during authorization {}",
                    msg
                ),
            },
            None => {
                println!("Unexpected response from websocket {}", msg);
                break;
            }
        }
    }
    maybe_long_lived_token_response
        .ok_or_else(|| Box::from("Could not retrieve long lived token from websocket (perhaps it already has been created for Halcyon?)"))
}

fn write_new_config(config: &YamlConfig, file_name: &str) -> Result<()> {
    let new_config_str = serde_yaml::to_string(&config)?;
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(file_name)?;
    f.write_all(new_config_str.as_bytes())?;
    Ok(())
}

impl YamlConfig {
    pub fn update_device_id_if_needed(self, file_name: &str) -> Result<Self> {
        let new_config = match self.ha.device_id {
            None => {
                let ha_config = HaConfig {
                    device_id: Some(Uuid::new_v4().to_string()),
                    ..self.ha
                };
                let config = YamlConfig { ha: ha_config };
                println!("no device-id found in config, so we are making one for you");
                write_new_config(&config, file_name)?;
                config
            }
            Some(_) => self,
        };
        Ok(new_config)
    }

    pub async fn update_long_lived_access_token_if_needed(self, file_name: &str) -> Result<Self> {
        let new_config = match self.ha.long_lived_token {
            None => {
                let access_token_resp = wait_for_token(&self).await?;
                let long_lived_access_token_resp =
                    get_long_lived_token_from_ws(&self, access_token_resp.access_token)?;
                let ha_config = HaConfig {
                    long_lived_token: long_lived_access_token_resp.result,
                    ..self.ha
                };
                let config = YamlConfig { ha: ha_config };
                println!(
                    "no long lived access token found in config, so we are making one for you"
                );
                write_new_config(&config, file_name)?;
                config
            }
            Some(_) => self,
        };
        Ok(new_config)
    }

    pub fn update_webhook_id_if_needed(self, file_name: &str, register_device_response: &RegisterDeviceResponse) -> Result<Self> {
        let new_config = match self.ha.webhook_id {
            None => {
                let ha_config = HaConfig {
                    webhook_id: Some(register_device_response.webhook_id.clone()),
                    ..self.ha
                };
                let config = YamlConfig { ha: ha_config };
                write_new_config(&config, file_name)?;
                config
            }
            Some(_) => self,
        };
        Ok(new_config)
    }
}

pub fn read_config_yml(file_name: &str) -> Result<YamlConfig> {
    let f = std::fs::File::open(file_name)?;
    let config: YamlConfig = serde_yaml::from_reader(f)?;
    Ok(config)
}
