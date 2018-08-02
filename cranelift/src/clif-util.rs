#![deny(trivial_numeric_casts)]
#![warn(unused_import_braces, unstable_features, unused_extern_crates)]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        float_arithmetic, mut_mut, nonminimal_bool, option_map_unwrap_or, option_map_unwrap_or_else,
        unicode_not_nfc, use_self
    )
)]

#[macro_use]
extern crate cfg_if;
extern crate cranelift_codegen;
extern crate cranelift_filetests;
extern crate cranelift_reader;
extern crate docopt;
extern crate filecheck;
#[macro_use]
extern crate serde_derive;
#[cfg(feature = "disas")]
extern crate capstone;
extern crate term;

cfg_if! {
    if #[cfg(feature = "wasm")] {
        extern crate cranelift_entity;
        extern crate cranelift_wasm;
        extern crate wabt;
        mod wasm;
    }
}
extern crate target_lexicon;

use cranelift_codegen::{timing, VERSION};
use docopt::Docopt;
use std::io::{self, Write};
use std::process;

mod cat;
mod compile;
mod print_cfg;
mod rsfilecheck;
mod utils;

const USAGE: &str = "
Cranelift code generator utility

Usage:
    clif-util test [-vT] <file>...
    clif-util cat <file>...
    clif-util filecheck [-v] <file>
    clif-util print-cfg <file>...
    clif-util compile [-vpT] [--set <set>]... [--target <triple>] <file>...
    clif-util wasm [-ctvpTs] [--set <set>]... [--target <triple>] <file>...
    clif-util --help | --version

Options:
    -v, --verbose   be more verbose
    -T, --time-passes
                    print pass timing report
    -t, --just-decode
                    just decode WebAssembly to Cranelift IR
    -s, --print-size
                    prints generated code size
    -c, --check-translation
                    just checks the correctness of Cranelift IR translated from WebAssembly
    -p, --print     print the resulting Cranelift IR
    -h, --help      print this help message
    --set=<set>     configure Cranelift settings
    --target=<triple>
                    specify the Cranelift target
    --version       print the Cranelift version

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
    flag_target: String,
    flag_time_passes: bool,
    flag_print_size: bool,
}

/// A command either succeeds or fails with an error message.
pub type CommandResult = Result<(), String>;

/// Parse the command line arguments and run the requested command.
fn clif_util() -> CommandResult {
    // Parse command line arguments.
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| {
            d.help(true)
                .version(Some(format!("Cranelift {}", VERSION)))
                .deserialize()
        })
        .unwrap_or_else(|e| e.exit());

    // Find the sub-command to execute.
    let result = if args.cmd_test {
        cranelift_filetests::run(args.flag_verbose, &args.arg_file).map(|_time| ())
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
            &args.flag_target,
        )
    } else if args.cmd_wasm {
        #[cfg(feature = "wasm")]
        let result = wasm::run(
            args.arg_file,
            args.flag_verbose,
            args.flag_just_decode,
            args.flag_check_translation,
            args.flag_print,
            &args.flag_set,
            &args.flag_target,
            args.flag_print_size,
        );

        #[cfg(not(feature = "wasm"))]
        let result = Err("Error: clif-util was compiled without wasm support.".to_owned());

        result
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
    if let Err(mut msg) = clif_util() {
        if !msg.ends_with('\n') {
            msg.push('\n');
        }
        io::stdout().flush().expect("flushing stdout");
        io::stderr().write_all(msg.as_bytes()).unwrap();
        process::exit(1);
    }
}
