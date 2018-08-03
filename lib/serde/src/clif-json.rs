//! Utility for `cranelift_serde`.

#![deny(missing_docs, trivial_numeric_casts, unused_extern_crates, unstable_features)]
#![warn(unused_import_braces)]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
#![cfg_attr(feature = "cargo-clippy", allow(new_without_default, new_without_default_derive))]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        float_arithmetic, mut_mut, nonminimal_bool, option_map_unwrap_or, option_map_unwrap_or_else,
        unicode_not_nfc, use_self
    )
)]

extern crate cranelift_reader;
extern crate docopt;
#[macro_use]
extern crate serde_derive;
extern crate cranelift_codegen;

extern crate serde_json;

use cranelift_reader::parse_functions;
use docopt::Docopt;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, Write};
use std::process;
use std::vec::Vec;

mod serde_clif_json;

const USAGE: &str = "
Cranelift JSON serializer/deserializer utility

Usage:
    clif-json serialize [-p] <file>
    clif-json deserialize <file>

Options:
    -p, --pretty     print pretty json

";

#[derive(Deserialize, Debug)]
struct Args {
    cmd_serialize: bool,
    cmd_deserialize: bool,
    flag_pretty: bool,
    arg_file: Vec<String>,
}

/// A command either succeeds or fails with an error message.
pub type CommandResult = Result<(), String>;

/// Serialize Cranelift IR to JSON
fn call_ser(file: &str, pretty: bool) -> CommandResult {
    let ret_of_parse = parse_functions(file);
    match ret_of_parse {
        Ok(funcs) => {
            let ser_funcs = serde_clif_json::SerObj::new(&funcs);
            let ser_str: String;
            if pretty {
                ser_str = serde_json::to_string_pretty(&ser_funcs).unwrap();
            } else {
                ser_str = serde_json::to_string(&ser_funcs).unwrap();
            }
            println!("{}", ser_str);
            Ok(())
        }
        Err(_pe) => Err(format!("this was a parsing error")),
    }
}

/// Deserialize JSON to Cranelift IR
fn call_de(file: &File) -> CommandResult {
    let de: serde_clif_json::SerObj = match serde_json::from_reader(file) {
        Result::Ok(val) => val,
        Result::Err(err) => panic!("{}", err),
    };
    println!("{:?}", de);
    Ok(())
}

/// Parse the command line arguments and run the requested command.
fn clif_json() -> CommandResult {
    // Parse command line arguments.
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.help(true).deserialize())
        .unwrap_or_else(|e| e.exit());

    // Find the sub-command to execute.
    let result = if args.cmd_serialize {
        if let Some(first_file) = args.arg_file.first() {
            let mut file = File::open(first_file).expect("Unable to open the file");
            let mut contents = String::new();
            file.read_to_string(&mut contents)
                .expect("Unable to read the file");
            call_ser(&contents, args.flag_pretty)
        } else {
            Err(format!("No file was passed"))
        }
    } else if args.cmd_deserialize {
        if let Some(first_file) = args.arg_file.first() {
            let mut file = File::open(first_file).expect("Unable to open the file");
            call_de(&file)
        } else {
            Err(format!("No file was passed"))
        }
    } else {
        // Debugging / shouldn't happen with proper command line handling above.
        Err(format!("Unhandled args: {:?}", args))
    };

    result
}

fn main() {
    if let Err(mut msg) = clif_json() {
        if !msg.ends_with('\n') {
            msg.push('\n');
        }
        io::stdout().flush().expect("flushing stdout");
        io::stderr().write_all(msg.as_bytes()).unwrap();
        process::exit(1);
    }
}
