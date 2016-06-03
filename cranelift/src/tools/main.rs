
extern crate cretonne;
extern crate cton_reader;
extern crate docopt;
extern crate rustc_serialize;

use cretonne::VERSION;
use docopt::Docopt;
use std::io::{self, Write};
use std::process;


mod cat;

const USAGE: &'static str = "
Cretonne code generator utility

Usage:
    cton-util cat <file>...
    cton-util --help | --version

Options:
    -h, --help  print this help message
    --version   print the Cretonne version

";

#[derive(RustcDecodable, Debug)]
struct Args {
    cmd_cat: bool,
    arg_file: Vec<String>,
}

/// A command either succeeds or fails with an error message.
pub type CommandResult = Result<(), String>;

/// Parse the command line arguments and run the requested command.
fn cton_util() -> CommandResult {
    // Parse comand line arguments.
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| {
            d.help(true)
                .version(Some(format!("Cretonne {}", VERSION)))
                .decode()
        })
        .unwrap_or_else(|e| e.exit());

    // Find the sub-command to execute.
    if args.cmd_cat {
        cat::run(args.arg_file)
    } else {
        // Debugging / shouldn't happen with proper command line handling above.
        Err(format!("Unhandled args: {:?}", args))
    }
}

fn main() {
    if let Err(mut msg) = cton_util() {
        if !msg.ends_with('\n') {
            msg.push('\n');
        }
        io::stderr().write(msg.as_bytes()).unwrap();
        process::exit(1);
    }
}
