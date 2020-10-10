mod config;
mod ha_api;

use clap::{App, Arg};
use platform_info::PlatformInfo;
use std::error;

use tiny_http::{Response, Server};

use crate::config::YamlConfig;
use std::collections::HashMap;
use url::Url;

use std::net::TcpListener;
use std::thread::spawn;
use tungstenite::server::accept;

use tungstenite::{connect, Message};

use serde::{Deserialize, Serialize};

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
async fn command_setup(_args: &clap::ArgMatches<'_>) -> Result<()> {
    println!("Welcome to setup");
    let config_file = _args.value_of("config").unwrap_or("config.yml");
    let platform_info = PlatformInfo::new()?;
    let config = config::read_config_yml(config_file)?
        .update_device_id_if_needed(config_file)?
        .update_refresh_token_if_needed(config_file)
        .await?
        .update_long_lived_access_token_if_needed(config_file)?;

    let access_token = config.ha.access_token.as_ref().unwrap().as_str();
    ha_api::register_machine(&config, &platform_info).await?;
    Ok(())
}
