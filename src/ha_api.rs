//use reqwest::Client;
use futures::executor;
use std::time;

pub mod errors;
pub mod types;

const CLIENT_ID: &str = "https://halcyon.casa";

#[derive(Debug)]
pub struct HomeAssistantAPI {
    instance_urls: Vec<String>,
    token: Token,
    client: reqwest::Client,
    webhook_id: Option<String>,
    cloudhook_url: Option<String>,
    remote_ui_url: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Token {
    Oauth(OAuthToken),
    LongLived(LongLivedToken),
}

#[derive(Debug, Clone)]
pub struct OAuthToken {
    token: String,
    token_expiration: std::time::SystemTime,
    refresh_token: String,
}

#[derive(Debug, Clone)]
pub struct LongLivedToken {
    token: String,
}

impl HomeAssistantAPI {
    pub fn from_token(
        instance_urls: Vec<String>,
        oauth_token: String,
        refresh_token: String,
        token_expiration: u64,
    ) -> Result<Self, errors::Error> {
        let oauth = OAuthToken {
            token: oauth_token,
            refresh_token: refresh_token,
            token_expiration: (time::UNIX_EPOCH + time::Duration::from_secs(token_expiration)),
        };

        let ret = Self {
            instance_urls: instance_urls,
            token: Token::Oauth(oauth),
            client: reqwest::Client::new(),
            webhook_id: None,
            cloudhook_url: None,
            remote_ui_url: None,
        };
        return Ok(ret);
    }

    pub fn from_authorization_code(
        instance_urls: Vec<String>,
        authorization_code: String,
    ) -> Result<Self, errors::Error> {
        let client = reqwest::Client::new();
        let response = client
            .post(instance_urls[0].as_str())
            .query(&[
                ("grant_type", "authorization_code"),
                ("client_id", CLIENT_ID),
                ("code", authorization_code.as_str()),
            ])
            .send();
        match executor::block_on(response) {
            Ok(response) => {
                match response.error_for_status() {
                    Ok(response) => {
                        match executor::block_on(response.json::<types::AuthorizationCode>()) {
                            Ok(response_data) => {
                                let token_time = (time::SystemTime::now()
                                    + time::Duration::from_secs(1800))
                                .duration_since(time::SystemTime::UNIX_EPOCH)
                                .unwrap()
                                .as_secs();

                                return HomeAssistantAPI::from_token(
                                    instance_urls,
                                    response_data.access_token,
                                    response_data.refresh_token,
                                    token_time,
                                );
                            }
                            Err(err) => {
                                return Err(errors::Error::from(err));
                            }
                        };
                    }
                    Err(err) => {
                        return Err(errors::Error::from(err));
                    }
                };
            }
            Err(err) => {
                return Err(errors::Error::from(err));
            }
        };
    }

    pub fn from_long_lived_token(
        instance_urls: Vec<String>,
        long_lived_token: String,
    ) -> Result<Self, errors::Error> {
        let token = LongLivedToken {
            token: long_lived_token,
        };

        let ret = Self {
            instance_urls: instance_urls,
            token: Token::LongLived(token),
            client: reqwest::Client::new(),
            webhook_id: None,
            cloudhook_url: None,
            remote_ui_url: None,
        };
        return Ok(ret);
    }

    fn get_token(&self) -> String {
        let token = self.token.clone();
        return match token {
            Token::Oauth(oauth) => oauth.refresh_token,
            Token::LongLived(long_lived) => long_lived.token
        }
    }

    pub fn need_refresh(&self) -> bool {
        match &self.token {
            Token::Oauth(oauth) => {
                match time::SystemTime::now().duration_since(oauth.token_expiration) {
                    Ok(sec_left) => {
                        if sec_left <= time::Duration::from_secs(10) {
                            return false;
                        } else {
                            return true;
                        }
                    }
                    Err(_) => return false,
                };
            }
            Token::LongLived(_) => {
                return false;
            }
        }
    }

    pub fn refresh_token(&mut self) -> Result<(), errors::Error> {
        let token = self.token.clone(); // This is dump but I have to do it apparently
        let refresh_token: String = match token {
            Token::Oauth(oauth) => oauth.refresh_token,
            Token::LongLived(_) => {
                return Err(errors::Error::Refresh());
            }
        };

        let response = self
            .client
            .post(&self.instance_urls[0])
            .query(&[
                ("grant_type", "refresh_token"),
                ("client_id", CLIENT_ID),
                ("refresh_token", refresh_token.as_str()),
            ])
            .send();
        match executor::block_on(response) {
            Ok(response) => {
                match response.error_for_status() {
                    Ok(response) => {
                        match executor::block_on(response.json::<types::RefreshToken>()) {
                            Ok(response_data) => {
                                let oauth = OAuthToken {
                                    token: response_data.access_token,
                                    token_expiration: time::SystemTime::now()
                                        + time::Duration::from_secs(response_data.expires_in),
                                    refresh_token: refresh_token.to_string(),
                                };
                                self.token = Token::Oauth(oauth);
                                return Ok(());
                            }
                            Err(err) => {
                                return Err(errors::Error::from(err));
                            }
                        };
                    }
                    Err(err) => {
                        return Err(errors::Error::from(err));
                    }
                };
            }
            Err(err) => {
                return Err(errors::Error::from(err));
            }
        };
    }

    pub fn register_device(
        &mut self,
        device_data: types::DeviceRegistrationRequest,
    ) -> Result<(), errors::Error> {
        if self.need_refresh() {
            self.refresh_token().unwrap()
        }
        let url = format!(
            "{}/api/mobile_app/registrations",
            self.instance_urls[0].as_str()
        );

        println!("{:?}", self.get_token());

        let response = self
            .client
            .post(&url)
            .bearer_auth(self.get_token())
            .json(&device_data);

        match executor::block_on(response.send()) {
            Ok(response) => {
                match response.error_for_status() {
                    Ok(response) => {
                        match executor::block_on(
                            response.json::<types::DeviceRegistrationResponse>(),
                        ) {
                            Ok(response_data) => {
                                self.webhook_id = Some(response_data.webhook_id);
                                self.cloudhook_url = response_data.cloudhook_url;
                                self.remote_ui_url = response_data.remote_ui_url;
                                return Ok(());
                            }
                            Err(err) => {
                                return Err(errors::Error::from(err));
                            }
                        };
                    }
                    Err(err) => {
                        return Err(errors::Error::from(err));
                    }
                };
            }
            Err(err) => {
                return Err(errors::Error::from(err));
            }
        }
    }

    pub fn register_sensor(
        &mut self,
        sensor_data: types::SensorRegistrationData,
    ) -> Result<(), errors::Error> {
        if self.need_refresh() {
            self.refresh_token().unwrap()
        }
        let register_sensor = types::SensorRegistrationRequest {
            data: sensor_data,
            r#type: String::from("register_sensor"),
        };
        match &self.webhook_id {
            Some(webhook_id) => {
                let url = format!(
                    "{}/api/webhook/{}",
                    self.instance_urls[0].as_str(),
                    webhook_id
                );

                println!("{}", self.get_token());

                let response = self
                    .client
                    .post(&url)
                    .bearer_auth(self.get_token())
                    .json(&register_sensor)
                    .send();
                match executor::block_on(response) {
                    Ok(response) => {
                        match response.error_for_status() {
                            Ok(_) => {
                                return Ok(());
                            }
                            Err(err) => {
                                return Err(errors::Error::from(err));
                            }
                        };
                    }
                    Err(err) => {
                        return Err(errors::Error::from(err));
                    }
                };
            }
            None => {
                return Err(errors::Error::Config(String::from("Missing Webhook ID")));
            }
        }
    }

    pub fn update_sensor(
        &mut self,
        sensor_data: types::SensorUpdateData,
    ) -> Result<(), errors::Error> {
        if self.need_refresh() {
            self.refresh_token().unwrap()
        }
        let register_sensor = types::SensorUpdateRequest {
            data: sensor_data,
            r#type: String::from("update_sensor_states"),
        };
        match &self.webhook_id {
            Some(webhook_id) => {
                let url = format!(
                    "{}/api/webhook/{}",
                    self.instance_urls[0].as_str(),
                    webhook_id
                );
                let response = self
                    .client
                    .post(&url)
                    .bearer_auth(self.get_token())
                    .json(&register_sensor)
                    .send();

                match executor::block_on(response) {
                    Ok(response) => {
                        match response.error_for_status() {
                            Ok(response) => {
                                match executor::block_on(response.json::<types::RefreshToken>()) {
                                    Ok(_response_data) => {
                                        return Ok(());
                                    }
                                    Err(err) => {
                                        return Err(errors::Error::from(err));
                                    }
                                };
                            }
                            Err(err) => {
                                return Err(errors::Error::from(err));
                            }
                        };
                    }
                    Err(err) => {
                        return Err(errors::Error::from(err));
                    }
                };
            }
            None => {
                return Err(errors::Error::Config(String::from("Missing Webhook ID")));
            }
        }
    }
}
