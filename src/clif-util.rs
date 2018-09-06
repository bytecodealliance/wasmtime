#![deny(trivial_numeric_casts)]
#![warn(unused_import_braces, unstable_features, unused_extern_crates)]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        float_arithmetic, mut_mut, nonminimal_bool, option_map_unwrap_or, option_map_unwrap_or_else,
        unicode_not_nfc, use_self
    )
)]

extern crate file_per_thread_logger;
#[macro_use]
extern crate cfg_if;
#[cfg(feature = "disas")]
extern crate capstone;
extern crate clap;
extern crate cranelift_codegen;
extern crate cranelift_entity;
extern crate cranelift_filetests;
extern crate cranelift_reader;
extern crate pretty_env_logger;

cfg_if! {
    if #[cfg(feature = "wasm")] {
        extern crate cranelift_wasm;
        extern crate term;
        extern crate wabt;
        mod wasm;
    }
}
extern crate target_lexicon;

use clap::{App, Arg, SubCommand};
use cranelift_codegen::dbg::LOG_FILENAME_PREFIX;
use cranelift_codegen::VERSION;
use std::io::{self, Write};
use std::option::Option;
use std::process;

mod cat;
mod compile;
mod print_cfg;
mod utils;

/// A command either succeeds or fails with an error message.
pub type CommandResult = Result<(), String>;

fn add_input_file_arg<'a>() -> clap::Arg<'a, 'a> {
    Arg::with_name("file")
        .default_value("-")
        .multiple(true)
        .value_name("file")
        .help("Specify file(s) to be used for test. Defaults to reading from stdin.")
}

fn add_single_input_file_arg<'a>() -> clap::Arg<'a, 'a> {
    Arg::with_name("single-file")
        .default_value("-")
        .value_name("single-file")
        .help("Specify a file to be used. Defaults to reading from stdin.")
}

fn add_pass_arg<'a>() -> clap::Arg<'a, 'a> {
    Arg::with_name("pass")
        .required(true)
        .multiple(true)
        .value_name("pass")
        .help("Specify pass(s) to be run on test file")
}

fn add_verbose_flag<'a>() -> clap::Arg<'a, 'a> {
    Arg::with_name("verbose").short("v").help("Be more verbose")
}

fn add_time_flag<'a>() -> clap::Arg<'a, 'a> {
    Arg::with_name("time-passes")
        .short("T")
        .help("Print pass timing report for test")
}

fn add_set_flag<'a>() -> clap::Arg<'a, 'a> {
    Arg::with_name("set")
        .long("set")
        .takes_value(true)
        .multiple(true)
        .help("Configure Cranelift settings")
}

fn add_target_flag<'a>() -> clap::Arg<'a, 'a> {
    Arg::with_name("target")
        .takes_value(true)
        .long("target")
        .help("Specify the Cranelift target")
}

fn add_print_flag<'a>() -> clap::Arg<'a, 'a> {
    Arg::with_name("print")
        .short("p")
        .help("Print the resulting Cranelift IR")
}

fn add_debug_flag<'a>() -> clap::Arg<'a, 'a> {
    Arg::with_name("debug")
        .short("d")
        .help("enable debug output on stderr/stdout")
}

/// Returns a vector of clap value options and changes these options into a vector of strings
fn get_vec<'a>(argument_vec: Option<clap::Values<'a>>) -> Vec<String> {
    let mut ret_vec: Vec<String> = Vec::new();
    if let Some(clap_vec) = argument_vec {
        for val in clap_vec {
            ret_vec.push(val.to_string());
        }
    }

    ret_vec
}

fn add_wasm_or_compile<'a>(cmd: &str) -> clap::App<'a, 'a> {
    let about_str = match cmd {
        "wasm" => "Compiles Cranelift IR into target language",
        "compile" => "Compiles Cranelift IR into target language",
        _ => panic!("Invalid command"),
    };

    SubCommand::with_name(cmd)
        .about(about_str)
        .arg(add_verbose_flag())
        .arg(add_print_flag())
        .arg(add_time_flag())
        .arg(add_set_flag())
        .arg(add_target_flag())
        .arg(add_input_file_arg())
        .arg(add_debug_flag())
}

fn handle_debug_flag(debug: bool) {
    if debug {
        pretty_env_logger::init();
    } else {
        file_per_thread_logger::initialize(LOG_FILENAME_PREFIX);
    }
}

fn main() {
    let app_cmds = App::new("Cranelift code generator utility")
        .version(VERSION)
        .subcommand(
            SubCommand::with_name("test")
                .about("Run Cranelift tests")
                .arg(add_verbose_flag())
                .arg(add_time_flag())
                .arg(add_input_file_arg())
                .arg(add_debug_flag()),
        )
        .subcommand(
            SubCommand::with_name("cat")
                .about("Outputs .clif file")
                .arg(add_input_file_arg())
                .arg(add_debug_flag()),
        )
        .subcommand(
            SubCommand::with_name("print-cfg")
                .about("Prints out cfg in dot format")
                .arg(add_input_file_arg())
                .arg(add_debug_flag()),
        )
        .subcommand(
            add_wasm_or_compile("compile")
                .arg(
                    Arg::with_name("just-decode")
                        .short("t")
                        .help("Just decode WebAssembly to Cranelift IR"),
                )
                .arg(Arg::with_name("check-translation").short("c").help(
                    "Just checks the correctness of Cranelift IR translated from WebAssembly",
                )),
        )
        .subcommand(add_wasm_or_compile("wasm"))
        .subcommand(
            SubCommand::with_name("pass")
                .about("Run specified pass(s) on an input file.")
                .arg(add_single_input_file_arg())
                .arg(add_target_flag())
                .arg(add_pass_arg())
                .arg(add_debug_flag())
                .arg(add_time_flag()),
        );

    let res_util = match app_cmds.get_matches().subcommand() {
        ("cat", Some(rest_cmd)) => {
            handle_debug_flag(rest_cmd.is_present("debug"));
            cat::run(&get_vec(rest_cmd.values_of("file")))
        }
        ("test", Some(rest_cmd)) => {
            handle_debug_flag(rest_cmd.is_present("debug"));
            cranelift_filetests::run(
                rest_cmd.is_present("verbose"),
                &get_vec(rest_cmd.values_of("file")),
            ).map(|_time| ())
        }
        ("pass", Some(rest_cmd)) => {
            handle_debug_flag(rest_cmd.is_present("debug"));

            let mut target_val: &str = "";
            if let Some(clap_target) = rest_cmd.value_of("target") {
                target_val = clap_target;
            }

            // Can be unwrapped because 'single-file' is required
            cranelift_filetests::run_passes(
                rest_cmd.is_present("verbose"),
                &get_vec(rest_cmd.values_of("pass")),
                target_val,
                &rest_cmd.value_of("single-file").unwrap().to_string(),
            ).map(|_time| ())
        }
        ("print-cfg", Some(rest_cmd)) => {
            handle_debug_flag(rest_cmd.is_present("debug"));
            print_cfg::run(&get_vec(rest_cmd.values_of("file")))
        }
        ("compile", Some(rest_cmd)) => {
            handle_debug_flag(rest_cmd.is_present("debug"));

            let mut target_val: &str = "";
            if let Some(clap_target) = rest_cmd.value_of("target") {
                target_val = clap_target;
            }

            compile::run(
                get_vec(rest_cmd.values_of("file")),
                rest_cmd.is_present("print"),
                &get_vec(rest_cmd.values_of("set")),
                target_val,
            )
        }
        ("wasm", Some(rest_cmd)) => {
            handle_debug_flag(rest_cmd.is_present("debug"));

            let mut target_val: &str = "";
            if let Some(clap_target) = rest_cmd.value_of("target") {
                target_val = clap_target;
            }

            #[cfg(feature = "wasm")]
            let result = wasm::run(
                get_vec(rest_cmd.values_of("file")),
                rest_cmd.is_present("verbose"),
                rest_cmd.is_present("just-decode"),
                rest_cmd.is_present("check-translation"),
                rest_cmd.is_present("print"),
                &get_vec(rest_cmd.values_of("set")),
                target_val,
                rest_cmd.is_present("print-size"),
            );

            #[cfg(not(feature = "wasm"))]
            let result = Err("Error: clif-util was compiled without wasm support.".to_owned());

            result
        }
        _ => Err(format!("Invalid subcommand.")),
    };

    if let Err(mut msg) = res_util {
        if !msg.ends_with('\n') {
            msg.push('\n');
        }
        io::stdout().flush().expect("flushing stdout");
        io::stderr().write_all(msg.as_bytes()).unwrap();
        process::exit(1);
    }
}
