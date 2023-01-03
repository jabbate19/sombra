mod icmp;
mod runtime;
mod vars;

use clap::{Arg, ArgAction, Command};

fn main() {
    let app = Command::new("RShell")
        .author("Joe Abbate, joe.abbate@mail.rit.edu")
        .version("1.0.0")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_parser(clap::builder::NonEmptyStringValueParser::new())
                .action(ArgAction::Set)
                .required(false)
                .help("Provides a config file to refer to"),
        )
        .arg(
            Arg::new("interface")
                .short('i')
                .long("interface")
                .value_parser(clap::builder::NonEmptyStringValueParser::new())
                .action(ArgAction::Set)
                .required(true)
                .help("Interface to listen on"),
        )
        .about("Ping C2")
        .get_matches();

    runtime::main(
        app.get_one::<String>("config"),
        app.get_one::<String>("interface").unwrap().to_owned(),
    );
}
