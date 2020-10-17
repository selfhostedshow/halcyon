mod config;

use clap::{App, Arg};
use ha_api::types::{RegisterDeviceRequest, SensorRegistrationData, SensorRegistrationRequest};
use ha_api::HomeAssistantAPI;
use platform_info::{PlatformInfo, Uname};
use std::error;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const OAUTH_CLIENT_ID: &str = "http://127.0.0.1:8000";

type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

#[tokio::main]
async fn main() -> Result<()> {
    let args = App::new("")
        .version("1.0")
        .author("Bradley Nelson <bradleynelson102@gmail.com>")
        .about("Home Assistant Linux Companion App")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .takes_value(true)
                .value_name("FILE"),
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .subcommand(App::new("setup").about("Setup Application"))
        .get_matches();

    match args.subcommand() {
        ("setup", Some(sub_m)) => command_setup(sub_m).await?,
        _ => {
            println!("Nothing to do, Goodbye");
        }
    };
    Ok(())
}
async fn command_setup(args: &clap::ArgMatches<'_>) -> Result<()> {
    println!("Welcome to setup");
    let config_file = args.value_of("config").unwrap_or("config.yml");
    let platform_info = PlatformInfo::new()?;

    let mut config = config::read_config_yml(config_file)?;
    config.update_device_id_if_needed(config_file)?;

    let mut ha_api = HomeAssistantAPI::new(config.ha.host.clone(), OAUTH_CLIENT_ID.to_string(), config.ha.long_lived_token.clone());

    config
        .update_long_lived_access_token_if_needed(&mut ha_api, OAUTH_CLIENT_ID.to_string(), config_file)
        .await?;


    let states = ha_api.api_states().await?;
    let name = platform_info.nodename().to_string();
    let maybe_current_device_state = states
        .into_iter()
        .find(|r| r.attributes.friendly_name.as_deref().unwrap_or("") == name);

    match maybe_current_device_state {
        None => {
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
            println!("Registering Machine");
            let register_match_resp = ha_api.register_machine(&request).await?;
            config.update_webhook_id_if_needed(config_file, &register_match_resp)?;
            println!("Successfully Registered Device!");
            let register_sensor_request = SensorRegistrationRequest {
                r#type: "register_sensor".to_string(),
                data: SensorRegistrationData {
                    device_class: Some(String::from("battery")),
                    icon: String::from("mdi:die-multiple"),
                    name: String::from("Sample Sensor"),
                    state: String::from("init"),
                    r#type: String::from("sensor"),
                    unique_id: String::from("sensor123"),
                    unit_of_measurement: String::from("none"),
                    attributes: std::collections::HashMap::new(),
                },
            };

            println!("Registering sensors");
            ha_api.register_sensor(&register_sensor_request).await?;
            println!("Successfully Registered Sensors")
        }
        Some(_) => println!("Device {} is already registered on Home Assistant", name),
    }

    Ok(())
}
