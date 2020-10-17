use serde::{Deserialize, Serialize};
use std::error;
use std::io::Write;
use uuid::Uuid;

use tiny_http::{Response, Server};

use std::collections::HashMap;
use url::Url;

use serde_json::Value;
use tungstenite::{connect, Message};

use ha_api::types::{GetAccessTokenResponse, RegisterDeviceResponse};
use ha_api::HomeAssistantAPI;

use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};

const FRAGMENT: &AsciiSet = &CONTROLS.add(b':').add(b'/');

// HA creates tokens for 10 years so we do the same
const LONG_LIVED_TOKEN_VALID_FOR: u32 = 365;

const LONG_LIVED_TOKEN_WS_COMMAND_ID: u32 = 11;

const LOCAL_SERVER_HOST: &str = "127.0.0.1:8000";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HaConfig {
    pub host: String,
    #[serde(rename = "long-lived-token")]
    pub long_lived_token: Option<String>,
    #[serde(rename = "device-id")]
    pub device_id: Option<String>,
    #[serde(rename = "webhook-id")]
    pub webhook_id: Option<String>,
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

pub async fn wait_for_token(
    config: &YamlConfig,
    client_id: String,
    ha_api: &mut HomeAssistantAPI,
) -> Result<GetAccessTokenResponse> {
    let maybe_server = Server::http(LOCAL_SERVER_HOST);
    match maybe_server {
        Ok(server) => {
            let query_prams = format!("client_id={}&redirect_uri=http://{}/callback", client_id, LOCAL_SERVER_HOST);
            let query_params_encoded: String = utf8_percent_encode(&query_prams, FRAGMENT).collect();
            println!("Open http://{}/auth/authorize?{} in your browser", config.ha.host.as_str(), query_params_encoded);
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
                                ha_api.access_token(code.to_string(), client_id).await;
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


impl YamlConfig {

    fn write_new_config(&self, file_name: &str) -> Result<()> {
        let new_config_str = serde_yaml::to_string(self)?;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(file_name)?;
        f.write_all(new_config_str.as_bytes())?;
        Ok(())
    }

    pub fn update_device_id_if_needed(&mut self, file_name: &str) -> Result<()> {
        match self.ha.device_id {
            None => {
                self.ha.device_id = Some(Uuid::new_v4().to_string());
                println!("no device-id found in config, so we are making one for you");
                self.write_new_config(file_name)?;
                Ok(())
            }
            Some(_) => Ok(()),
        }
    }

    pub async fn update_long_lived_access_token_if_needed(
        &mut self,
        ha_api: &mut HomeAssistantAPI,
        client_id: String,
        file_name: &str,
    ) -> Result<()> {
        match self.ha.long_lived_token {
            None => {
                let access_token_resp = wait_for_token(&self,  client_id, ha_api).await?;
                let long_lived_access_token_resp =
                    get_long_lived_token_from_ws(&self, access_token_resp.access_token)?;
                self.ha.long_lived_token = long_lived_access_token_resp.result;
                println!(
                    "no long lived access token found in config, so we are making one for you"
                );
                self.write_new_config(file_name)?;
                Ok(())
            }
            Some(_) => Ok(()),
        }
    }

    pub fn update_webhook_id_if_needed(
        &mut self,
        file_name: &str,
        register_device_response: &RegisterDeviceResponse,
    ) -> Result<()> {
        match self.ha.webhook_id {
            None => {
                self.ha.webhook_id = Some(register_device_response.webhook_id.clone());
                self.write_new_config(file_name)?;
                Ok(())
            }
            Some(_) => Ok(()),
        }
    }
}

pub fn read_config_yml(file_name: &str) -> Result<YamlConfig> {
    let f = std::fs::File::open(file_name)?;
    let config: YamlConfig = serde_yaml::from_reader(f)?;
    Ok(config)
}
