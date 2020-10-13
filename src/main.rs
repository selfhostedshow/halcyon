use clap::{App, Arg};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::File;
//use log::{debug, error, info, trace, warn};
use chrono;
use std::convert::TryFrom;
//use std::io::Read;
use std::os::unix::net::UnixListener;
//use std::path::Path;
//use std::time;
use std::u8;
use sysinfo::SystemExt;


mod ha_api;

const CONFIG_DIR_ENV_VAR: &str = "HALCYON_CONFIG_DIR";
const DEFAULT_CONFIG_DIR: &str = "/etc/halcyon/halcyon.yaml";
const CONFIG_SOCK_ENV_VAR: &str = "HALCYON_SOCK_PATH";
const DEFAULT_SOCK_PATH: &str = "/var/run/halcyon.sock";
const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Config {
    home_assistant_token: String,
    home_refresh_assistant_token: String,
    device_name: String,
    log_level: i8,
}

fn setup_logger(logging_level: u8) -> Result<(), fern::InitError> {
    let logging_level = match logging_level {
        0 => log::LevelFilter::Error,
        1 => log::LevelFilter::Warn,
        2 => log::LevelFilter::Info,
        3 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(logging_level)
        .chain(std::io::stdout())
        .chain(fern::log_file("output.log")?)
        .apply()?;
    Ok(())
}

#[tokio::main]
async fn main() {
    let args = App::new("")
        .version("1.0")
        .author("Bradley Nelson <bradleynelson102@gmail.com>")
        .about("Home Assistant Linux Companion App")
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .subcommand(App::new("setup").about("Setup Application"))
        .subcommand(App::new("service").about("Long Running Program to connect to HA"))
        .get_matches();
    let logging: u64 = args.occurrences_of("v");
    let logging: u8 = match logging {
        0..=8 => u8::try_from(logging).unwrap(),
        _ => {
            println!("Dang Calm down with the logging level");
            u8::MAX
        }
    };

    setup_logger(logging).expect("Setting up logging Failed");

    match args.subcommand() {
        ("setup", Some(sub_m)) => command_setup(sub_m),
        ("service", Some(sub_m)) => command_service(sub_m),
        _ => {
            println!("Nothing to do, Goodbye");
        }
    }
}

fn command_setup(_args: &clap::ArgMatches) {
    println!("Welcome to setup");
    println!("Hello, world!");

    //mdns or Home Assistant URL

    //send auth request to Home Assistant

    //Get authorization_code from user

    //Get Token & Refresh Token
    let home_assistant = ha_api::HomeAssistantAPI::from_authorization_code(
        vec!["www.ha.com".to_string()],
        "Test".to_string(),
    )
    .unwrap();
    home_assistant.need_refresh();

    //check if mobile_app is loaded

    //Can't to this no library If mobile_app is not loaded publish mdns _hass-mobile-app._tcp.local. and wait 60 seconds

    //Ask for Device name

    //register device

    //register sensors
}

fn command_service(_args: &clap::ArgMatches) {
    println!("Welcome to the service");

    println!("Enter Access Token");
    let mut token = String::new();

    let stdin = std::io::stdin();
    stdin.read_line(&mut token).unwrap();
    token = String::from(token.trim());

    println!("Got Access Token");

    //Load system setting file
    let mut _config_file = match env::var_os(CONFIG_DIR_ENV_VAR) {
        Some(val) => {
            println!("{}: {:?}", CONFIG_DIR_ENV_VAR, val);
            match File::open(&val) {
                Ok(file) => file,
                Err(why) => panic!("Couldn't open {}: {}", val.to_str().expect("Unknown"), why),
            }
        }
        None => {
            println!("{} is not defined in the environment.", CONFIG_DIR_ENV_VAR);
            match File::open(DEFAULT_CONFIG_DIR) {
                Ok(file) => file,
                Err(why) => panic!("Couldn't open {}: {}", DEFAULT_CONFIG_DIR, why),
            }
        }
    };

    let mut _socket: String;
    //Check for connection settings
    let sock_path = env::var_os(CONFIG_SOCK_ENV_VAR).unwrap_or(DEFAULT_SOCK_PATH.into());

    let listener = match UnixListener::bind(&sock_path) {
        Ok(socket) => socket,
        Err(err) => match err.kind() {
            std::io::ErrorKind::AddrInUse => {
                println!("Removing Existing Socket");
                match std::fs::remove_file(&sock_path) {
                    Ok(_) => match UnixListener::bind(&sock_path) {
                        Ok(socket) => socket,
                        Err(err) => {
                            panic!(
                                "Couldn't Bind Socket {}: {:?}",
                                sock_path.to_str().unwrap_or(""),
                                err
                            );
                        }
                    },
                    Err(err) => {
                        panic!(
                            "Couldn't Remove Socket {}: {:?}",
                            sock_path.to_str().unwrap_or(""),
                            err
                        );
                    }
                }
            }
            _ => {
                panic!(
                    "Couldn't Bind Socket {}: {:?}",
                    sock_path.to_str().unwrap_or(""),
                    err
                );
            }
        },
    };

    println!("Created Socket");

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(_stream) => {
                    //thread::spawn(|| handle_client(stream));
                }
                Err(err) => {
                    println!("Error: {}", err);
                    break;
                }
            }
        }
    });

    /*let time: u64 = (time::SystemTime::now() + time::Duration::from_secs(1800))
        .duration_since(time::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();*/

    let mut instance_url = String::new();
    println!("Enter HA URL");

    let stdin = std::io::stdin();
    stdin.read_line(&mut instance_url).unwrap();

    instance_url = String::from(instance_url.trim());

    //TODO remove trailing slash in general verify url


    let mut home_assistant = ha_api::HomeAssistantAPI::from_long_lived_token(
        vec![instance_url],
        token)
    .unwrap();
    
    let mut system = sysinfo::System::new_all();
    system.refresh_all();

    let device = ha_api::types::DeviceRegistrationRequest {
        device_id: String::from("casa.halcyon.example"),
        app_id: String::from("casa.halcyon"),
        app_name: String::from("Halcyon"),
        app_version: String::from(VERSION.unwrap_or("unknown")),
        device_name: String::from("casa.halcyon.example"),
        manufacturer: String::from("unknown"),
        model: String::from("unknown"),
        os_name: String::from("Linux"),
        os_version: String::from("unknown"),
        supports_encryption: false,
        app_data: ha_api::types::AppData {},
    };

    home_assistant.register_device(device).unwrap();

    let sensor = ha_api::types::SensorRegistrationData {
        device_class: Some(String::from("battery")),
        icon: String::from("mdi:die-multiple"),
        name: String::from("Sample Sensor"),
        state: String::from("init"),
        r#type: String::from("sensor"),
        unique_id: String::from("sensor123"),
        unit_of_measurement: String::from("none"),
        attributes: std::collections::HashMap::new(),
    };

    match home_assistant.register_sensor(sensor) {
        Ok(_) => {
            println!("Registered Sensor");
        }
        Err(err) => {
            panic!("Register Sensor: {}", err);
        }
    }

    //Register Sensors
}
