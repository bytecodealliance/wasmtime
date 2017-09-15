#[macro_use(dbg)]
extern crate cretonne;
extern crate cton_reader;
extern crate cton_wasm;
extern crate docopt;
#[macro_use]
extern crate serde_derive;
extern crate filecheck;
extern crate num_cpus;
extern crate tempdir;
extern crate term;

use cretonne::VERSION;
use docopt::Docopt;
use std::io::{self, Write};
use std::process;

mod utils;
mod filetest;
mod cat;
mod print_cfg;
mod rsfilecheck;
mod wasm;

const USAGE: &str = "
Cretonne code generator utility

Usage:
    cton-util test [-v] <file>...
    cton-util cat <file>...
    cton-util filecheck [-v] <file>
    cton-util print-cfg <file>...
    cton-util wasm [-cvo] [--set <set>]... [--isa <isa>] <file>...
    cton-util --help | --version

Options:
    -v, --verbose   be more verbose
    -c, --check     checks the correctness of Cretonne IL translated from WebAssembly
    -o, --optimize  runs otpimization passes on translated WebAssembly functions
    -h, --help      print this help message
    --set=<set>     configure Cretonne settings
    --isa=<isa>     specify the Cretonne ISA
    --version       print the Cretonne version

";

#[derive(Deserialize, Debug)]
struct Args {
    cmd_test: bool,
    cmd_cat: bool,
    cmd_filecheck: bool,
    cmd_print_cfg: bool,
    cmd_wasm: bool,
    arg_file: Vec<String>,
    flag_check: bool,
    flag_optimize: bool,
    flag_verbose: bool,
    flag_set: Vec<String>,
    flag_isa: String,
}

/// A command either succeeds or fails with an error message.
pub type CommandResult = Result<(), String>;

/// Parse the command line arguments and run the requested command.
fn cton_util() -> CommandResult {
    // Parse command line arguments.
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| {
            d.help(true)
                .version(Some(format!("Cretonne {}", VERSION)))
                .deserialize()
        })
        .unwrap_or_else(|e| e.exit());

    // Find the sub-command to execute.
    if args.cmd_test {
        filetest::run(args.flag_verbose, args.arg_file)
    } else if args.cmd_cat {
        cat::run(args.arg_file)
    } else if args.cmd_filecheck {
        rsfilecheck::run(args.arg_file, args.flag_verbose)
    } else if args.cmd_print_cfg {
        print_cfg::run(args.arg_file)
    } else if args.cmd_wasm {
        wasm::run(
            args.arg_file,
            args.flag_verbose,
            args.flag_optimize,
            args.flag_check,
            args.flag_set,
            args.flag_isa,
        )
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
        io::stdout().flush().expect("flushing stdout");
        io::stderr().write_all(msg.as_bytes()).unwrap();
        process::exit(1);
    }
}
