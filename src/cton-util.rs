#![deny(trivial_numeric_casts, unused_extern_crates)]
#![warn(unused_import_braces, unstable_features)]
#![cfg_attr(feature="cargo-clippy", warn(
                float_arithmetic,
                mut_mut,
                nonminimal_bool,
                option_map_unwrap_or,
                option_map_unwrap_or_else,
                unicode_not_nfc,
                use_self,
                ))]

extern crate cretonne_codegen;
extern crate cretonne_filetests;
extern crate cretonne_reader;
extern crate cretonne_wasm;
extern crate docopt;
extern crate filecheck;
#[macro_use]
extern crate serde_derive;
extern crate tempdir;
extern crate term;

use cretonne_codegen::{timing, VERSION};
use docopt::Docopt;
use std::io::{self, Write};
use std::process;

mod cat;
mod compile;
mod print_cfg;
mod rsfilecheck;
mod utils;
mod wasm;

const USAGE: &str = "
Cretonne code generator utility

Usage:
    cton-util test [-vT] <file>...
    cton-util cat <file>...
    cton-util filecheck [-v] <file>
    cton-util print-cfg <file>...
    cton-util compile [-vpT] [--set <set>]... [--isa <isa>] <file>...
    cton-util wasm [-ctvpTs] [--set <set>]... [--isa <isa>] <file>...
    cton-util --help | --version

Options:
    -v, --verbose   be more verbose
    -T, --time-passes
                    print pass timing report
    -t, --just-decode
                    just decode WebAssembly to Cretonne IR
    -s, --print-size
                    prints generated code size
    -c, --check-translation
                    just checks the correctness of Cretonne IR translated from WebAssembly
    -p, --print     print the resulting Cretonne IR
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
    cmd_compile: bool,
    cmd_wasm: bool,
    arg_file: Vec<String>,
    flag_just_decode: bool,
    flag_check_translation: bool,
    flag_print: bool,
    flag_verbose: bool,
    flag_set: Vec<String>,
    flag_isa: String,
    flag_time_passes: bool,
    flag_print_size: bool,
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
    let result = if args.cmd_test {
        cretonne_filetests::run(args.flag_verbose, &args.arg_file).map(|_time| ())
    } else if args.cmd_cat {
        cat::run(&args.arg_file)
    } else if args.cmd_filecheck {
        rsfilecheck::run(&args.arg_file, args.flag_verbose)
    } else if args.cmd_print_cfg {
        print_cfg::run(&args.arg_file)
    } else if args.cmd_compile {
        compile::run(
            args.arg_file,
            args.flag_print,
            &args.flag_set,
            &args.flag_isa,
        )
    } else if args.cmd_wasm {
        wasm::run(
            args.arg_file,
            args.flag_verbose,
            args.flag_just_decode,
            args.flag_check_translation,
            args.flag_print,
            &args.flag_set,
            &args.flag_isa,
            args.flag_print_size,
        )
    } else {
        // Debugging / shouldn't happen with proper command line handling above.
        Err(format!("Unhandled args: {:?}", args))
    };

    if args.flag_time_passes {
        print!("{}", timing::take_current());
    }

    result
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
