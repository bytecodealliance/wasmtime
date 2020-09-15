#![deny(trivial_numeric_casts)]
#![warn(unused_import_braces, unstable_features, unused_extern_crates)]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        clippy::float_arithmetic,
        clippy::mut_mut,
        clippy::nonminimal_bool,
        clippy::option_map_unwrap_or,
        clippy::option_map_unwrap_or_else,
        clippy::unicode_not_nfc,
        clippy::use_self
    )
)]

use clap::{arg_enum, App, Arg, SubCommand};
use cranelift_codegen::dbg::LOG_FILENAME_PREFIX;
use cranelift_codegen::VERSION;
use std::io::{self, Write};
use std::option::Option;
use std::process;

mod bugpoint;
mod cat;
mod compile;
mod disasm;
mod interpret;
mod print_cfg;
mod run;
#[cfg(feature = "souper-harvest")]
mod souper_harvest;
mod utils;

#[cfg(feature = "peepmatic-souper")]
mod souper_to_peepmatic;

#[cfg(feature = "wasm")]
mod wasm;

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
        .required(true)
        .value_name("single-file")
        .help("Specify a file to be used. Use '-' for stdin.")
}

fn add_output_arg<'a>() -> clap::Arg<'a, 'a> {
    Arg::with_name("output")
        .required(true)
        .default_value("-")
        .value_name("output")
        .short("o")
        .help("Specify output file to be used. Use '-' for stdout.")
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

arg_enum! {
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum UseTerminalColor {
        Auto,
        Never,
        Always
    }
}

fn add_color<'a>() -> clap::Arg<'a, 'a> {
    Arg::with_name("color")
        .long("color")
        .possible_values(&UseTerminalColor::variants())
        .takes_value(true)
        .multiple(false)
        .default_value("auto")
        .case_insensitive(true)
        .help("Use colors in output")
}

fn add_time_flag<'a>() -> clap::Arg<'a, 'a> {
    Arg::with_name("time-passes")
        .short("T")
        .help("Print pass timing report for test")
}

fn add_size_flag<'a>() -> clap::Arg<'a, 'a> {
    Arg::with_name("print-size")
        .short("X")
        .help("Print bytecode size")
}

fn add_disasm_flag<'a>() -> clap::Arg<'a, 'a> {
    Arg::with_name("disasm")
        .long("disasm")
        .short("D")
        .help("Print machine code disassembly")
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
        .help("Enable debug output on stderr/stdout")
}

fn add_just_decode_flag<'a>() -> clap::Arg<'a, 'a> {
    Arg::with_name("just-decode")
        .short("t")
        .help("Just decode into Cranelift IR")
}

fn add_check_translation_flag<'a>() -> clap::Arg<'a, 'a> {
    Arg::with_name("check-translation")
        .short("c")
        .help("Just checks the correctness of Cranelift IR translated from WebAssembly")
}

fn add_value_ranges<'a>() -> clap::Arg<'a, 'a> {
    Arg::with_name("value-ranges")
        .long("value-ranges")
        .help("Display values ranges and their locations")
}

/// Returns a vector of clap value options and changes these options into a vector of strings
fn get_vec(argument_vec: Option<clap::Values>) -> Vec<String> {
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
        "wasm" => "Compiles Wasm binary/text into Cranelift IR and then into target language",
        "compile" => "Compiles Cranelift IR into target language",
        _ => panic!("Invalid command"),
    };

    SubCommand::with_name(cmd)
        .about(about_str)
        .arg(add_verbose_flag())
        .arg(add_print_flag())
        .arg(add_time_flag())
        .arg(add_disasm_flag())
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
            SubCommand::with_name("run")
                .about("Execute CLIF code and verify with test expressions")
                .arg(add_verbose_flag())
                .arg(add_input_file_arg())
                .arg(add_debug_flag()),
        )
        .subcommand(
            SubCommand::with_name("interpret")
                .about("Interpret CLIF code")
                .arg(add_verbose_flag())
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
        .subcommand(add_wasm_or_compile("compile"))
        .subcommand(
            add_wasm_or_compile("wasm")
                .arg(add_size_flag())
                .arg(add_just_decode_flag())
                .arg(add_check_translation_flag())
                .arg(add_value_ranges())
                .arg(add_color()),
        )
        .subcommand(
            SubCommand::with_name("pass")
                .about("Run specified pass(s) on an input file.")
                .arg(add_single_input_file_arg())
                .arg(add_target_flag())
                .arg(add_pass_arg())
                .arg(add_debug_flag())
                .arg(add_time_flag()),
        )
        .subcommand(
            SubCommand::with_name("bugpoint")
                .about("Reduce size of clif file causing panic during compilation.")
                .arg(add_single_input_file_arg())
                .arg(add_set_flag())
                .arg(add_target_flag())
                .arg(add_verbose_flag()),
        )
        .subcommand(
            SubCommand::with_name("souper-to-peepmatic")
                .about("Convert Souper optimizations into Peepmatic DSL.")
                .arg(add_single_input_file_arg())
                .arg(add_output_arg()),
        )
        .subcommand(
            SubCommand::with_name("souper-harvest")
                .arg(add_single_input_file_arg())
                .arg(add_output_arg())
                .arg(add_target_flag())
                .arg(add_set_flag()),
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
                rest_cmd.is_present("time-passes"),
                &get_vec(rest_cmd.values_of("file")),
            )
            .map(|_time| ())
        }
        ("run", Some(rest_cmd)) => {
            handle_debug_flag(rest_cmd.is_present("debug"));
            run::run(
                get_vec(rest_cmd.values_of("file")),
                rest_cmd.is_present("verbose"),
            )
            .map(|_time| ())
        }
        ("interpret", Some(rest_cmd)) => {
            handle_debug_flag(rest_cmd.is_present("debug"));
            interpret::run(
                get_vec(rest_cmd.values_of("file")),
                rest_cmd.is_present("verbose"),
            )
            .map(|_time| ())
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
                rest_cmd.is_present("time-passes"),
                &get_vec(rest_cmd.values_of("pass")),
                target_val,
                rest_cmd.value_of("single-file").unwrap(),
            )
            .map(|_time| ())
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
                rest_cmd.is_present("disasm"),
                rest_cmd.is_present("time-passes"),
                &get_vec(rest_cmd.values_of("set")),
                target_val,
            )
        }
        ("wasm", Some(rest_cmd)) => {
            handle_debug_flag(rest_cmd.is_present("debug"));

            #[cfg(feature = "wasm")]
            let result = {
                let mut target_val: &str = "";
                if let Some(clap_target) = rest_cmd.value_of("target") {
                    target_val = clap_target;
                }

                wasm::run(
                    get_vec(rest_cmd.values_of("file")),
                    rest_cmd.value_of("color").unwrap().parse().unwrap(),
                    rest_cmd.is_present("verbose"),
                    rest_cmd.is_present("just-decode"),
                    rest_cmd.is_present("check-translation"),
                    rest_cmd.is_present("print"),
                    rest_cmd.is_present("disasm"),
                    &get_vec(rest_cmd.values_of("set")),
                    target_val,
                    rest_cmd.is_present("print-size"),
                    rest_cmd.is_present("time-passes"),
                    rest_cmd.is_present("value-ranges"),
                )
            };

            #[cfg(not(feature = "wasm"))]
            let result = Err("Error: clif-util was compiled without wasm support.".to_owned());

            result
        }
        ("bugpoint", Some(rest_cmd)) => {
            let mut target_val: &str = "";
            if let Some(clap_target) = rest_cmd.value_of("target") {
                target_val = clap_target;
            }

            bugpoint::run(
                rest_cmd.value_of("single-file").unwrap(),
                &get_vec(rest_cmd.values_of("set")),
                target_val,
                rest_cmd.is_present("verbose"),
            )
        }
        ("souper-to-peepmatic", Some(rest_cmd)) => {
            #[cfg(feature = "peepmatic-souper")]
            {
                use std::path::Path;
                souper_to_peepmatic::run(
                    Path::new(rest_cmd.value_of("single-file").unwrap()),
                    Path::new(rest_cmd.value_of("output").unwrap()),
                )
            }
            #[cfg(not(feature = "peepmatic-souper"))]
            {
                Err(
                    "Error: clif-util was compiled without support for the `souper-to-peepmatic` \
                     subcommand"
                        .into(),
                )
            }
        }
        ("souper-harvest", Some(rest_cmd)) => {
            #[cfg(feature = "souper-harvest")]
            {
                souper_harvest::run(
                    rest_cmd.value_of("target").unwrap_or_default(),
                    rest_cmd.value_of("single-file").unwrap(),
                    rest_cmd.value_of("output").unwrap(),
                    &get_vec(rest_cmd.values_of("set")),
                )
            }

            #[cfg(not(feature = "souper-harvest"))]
            {
                Err("clif-util was compiled without `souper-harvest` support".into())
            }
        }
        _ => Err("Invalid subcommand.".to_owned()),
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
