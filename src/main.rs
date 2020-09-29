use clap::{App, Arg};

fn main() {
    let args = App::new("")
        .version("1.0")
        .author("Bradley Nelson <bradleynelson102@gmail.com>")
        .about("Home Assistant Linux Companion App")
        .arg(Arg::with_name("config").short("c").long("config"))
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .subcommand(App::new("setup").about("Setup Application"))
        .get_matches();

    match args.subcommand() {
        ("setup", Some(sub_m)) => command_setup(sub_m),
        _ => {}
    }
}

fn command_setup(args: &clap::ArgMatches) {}
