//! Utility for `cranelift_serde`.

#![deny(
    missing_docs,
    trivial_numeric_casts,
    unused_extern_crates,
    unstable_features
)]
#![warn(unused_import_braces)]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::new_without_default))]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        clippy::float_arithmetic,
        clippy::mut_mut,
        clippy::nonminimal_bool,
        clippy::map_unwrap_or,
        clippy::clippy::unicode_not_nfc,
        clippy::use_self
    )
)]

use clap::{App, Arg, SubCommand};
use cranelift_reader::parse_functions;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, Write};
use std::process;

mod serde_clif_json;

fn call_ser(file: &str, pretty: bool) -> Result<(), String> {
    let ret_of_parse = parse_functions(file);
    match ret_of_parse {
        Ok(funcs) => {
            let ser_funcs = serde_clif_json::SerObj::new(&funcs);
            let ser_str = if pretty {
                serde_json::to_string_pretty(&ser_funcs).unwrap()
            } else {
                serde_json::to_string(&ser_funcs).unwrap()
            };
            println!("{}", ser_str);
            Ok(())
        }
        Err(_pe) => Err("There was a parsing error".to_string()),
    }
}

fn call_de(file: &File) -> Result<(), String> {
    let de: serde_clif_json::SerObj = match serde_json::from_reader(file) {
        Result::Ok(val) => val,
        Result::Err(err) => panic!("{}", err),
    };
    println!("{:?}", de);
    Ok(())
}

fn main() {
    let matches = App::new("Cranelift JSON serializer/deserializer utility")
        .subcommand(
            SubCommand::with_name("serialize")
                .display_order(1)
                .about("Serializes Cranelift IR into JSON.")
                .arg(Arg::with_name("pretty").short("p").help("pretty json"))
                .arg(
                    Arg::with_name("FILE")
                        .required(true)
                        .value_name("FILE")
                        .help("Input file for serialization"),
                ),
        )
        .subcommand(
            SubCommand::with_name("deserialize")
                .about("Deserializes Cranelift IR into JSON.")
                .arg(
                    Arg::with_name("FILE")
                        .required(true)
                        .value_name("FILE")
                        .help("Input file for deserialization"),
                ),
        )
        .get_matches();

    let res_serde = match matches.subcommand() {
        ("serialize", Some(m)) => {
            let mut file =
                File::open(m.value_of("FILE").unwrap()).expect("Unable to open the file");
            let mut contents = String::new();
            file.read_to_string(&mut contents)
                .expect("Unable to read the file");

            match m.occurrences_of("pretty") {
                0 => call_ser(&contents, false),
                _ => call_ser(&contents, true),
            }
        }
        ("deserialize", Some(m)) => {
            let file = File::open(m.value_of("FILE").unwrap()).expect("Unable to open the file");
            call_de(&file)
        }
        _ => Err("Invalid subcommand.".to_string()),
    };

    if let Err(mut msg) = res_serde {
        if !msg.ends_with('\n') {
            msg.push('\n');
        }
        io::stdout().flush().expect("flushing stdout");
        io::stderr().write_all(msg.as_bytes()).unwrap();
        process::exit(1);
    }
}
