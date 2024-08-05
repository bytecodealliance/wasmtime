//! Utility for `cranelift_serde`.

#![deny(missing_docs)]

use clap::Parser;
use cranelift_codegen::ir::Function;
use cranelift_reader::parse_functions;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::process;

fn call_ser(file: &str, pretty: bool) -> Result<(), String> {
    let ret_of_parse = parse_functions(file);
    match ret_of_parse {
        Ok(funcs) => {
            let ser_str = if pretty {
                serde_json::to_string_pretty(&funcs).unwrap()
            } else {
                serde_json::to_string(&funcs).unwrap()
            };
            println!("{ser_str}");
            Ok(())
        }
        Err(_pe) => Err("There was a parsing error".to_string()),
    }
}

fn call_de(file: &File) -> Result<(), String> {
    let de: Vec<Function> = match serde_json::from_reader(file) {
        Result::Ok(val) => val,
        Result::Err(err) => panic!("{}", err),
    };
    println!("{de:?}");
    Ok(())
}

/// Cranelift JSON serializer/deserializer utility
#[derive(Parser, Debug)]
#[command(about)]
enum Args {
    /// Serializes Cranelift IR into JSON
    Serialize {
        /// Generate pretty json
        #[arg(long, short)]
        pretty: bool,

        /// Input file for serialization
        file: String,
    },
    /// Deserializes Cranelift IR into JSON
    Deserialize {
        /// Input file for deserialization
        file: String,
    },
}

fn main() {
    let res_serde = match Args::parse() {
        Args::Serialize { pretty, file } => {
            let mut contents = String::new();
            let mut file = File::open(file).expect("Unable to open the file");
            file.read_to_string(&mut contents)
                .expect("Unable to read the file");

            call_ser(&contents, pretty)
        }
        Args::Deserialize { file } => {
            let file = File::open(file).expect("Unable to open the file");
            call_de(&file)
        }
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
