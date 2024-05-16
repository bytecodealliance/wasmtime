//! The `wasmtime` command line tool.
//!
//! Primarily used to run WebAssembly modules.
//! See `wasmtime --help` for usage.

use anyhow::Result;
use clap::Parser;

/// Wasmtime WebAssembly Runtime
#[derive(Parser, PartialEq)]
#[command(
    version = version(),
    after_help = "If a subcommand is not provided, the `run` subcommand will be used.\n\
                  \n\
                  Usage examples:\n\
                  \n\
                  Running a WebAssembly module with a start function:\n\
                  \n  \
                  wasmtime example.wasm
                  \n\
                  Passing command line arguments to a WebAssembly module:\n\
                  \n  \
                  wasmtime example.wasm arg1 arg2 arg3\n\
                  \n\
                  Invoking a specific function (e.g. `add`) in a WebAssembly module:\n\
                  \n  \
                  wasmtime --invoke add example.wasm 1 2\n",

    // This option enables the pattern below where we ask clap to parse twice
    // sorta: once where it's trying to find a subcommand and once assuming
    // a subcommand doesn't get passed. Clap should then, apparently,
    // fill in the `subcommand` if found and otherwise fill in the
    // `RunCommand`.
    args_conflicts_with_subcommands = true
)]
struct Wasmtime {
    #[cfg(not(feature = "run"))]
    #[command(subcommand)]
    subcommand: Subcommand,

    #[cfg(feature = "run")]
    #[command(subcommand)]
    subcommand: Option<Subcommand>,
    #[command(flatten)]
    #[cfg(feature = "run")]
    run: wasmtime_cli::commands::RunCommand,
}

/// If WASMTIME_VERSION_INFO is set, use it, otherwise use CARGO_PKG_VERSION.
fn version() -> &'static str {
    option_env!("WASMTIME_VERSION_INFO").unwrap_or(env!("CARGO_PKG_VERSION"))
}

#[derive(Parser, PartialEq)]
enum Subcommand {
    /// Runs a WebAssembly module
    #[cfg(feature = "run")]
    Run(wasmtime_cli::commands::RunCommand),

    /// Controls Wasmtime configuration settings
    #[cfg(feature = "cache")]
    Config(wasmtime_cli::commands::ConfigCommand),

    /// Compiles a WebAssembly module.
    #[cfg(feature = "compile")]
    Compile(wasmtime_cli::commands::CompileCommand),

    /// Explore the compilation of a WebAssembly module to native code.
    #[cfg(feature = "explore")]
    Explore(wasmtime_cli::commands::ExploreCommand),

    /// Serves requests from a wasi-http proxy component.
    #[cfg(feature = "serve")]
    Serve(wasmtime_cli::commands::ServeCommand),

    /// Displays available Cranelift settings for a target.
    #[cfg(feature = "cranelift")]
    Settings(wasmtime_cli::commands::SettingsCommand),

    /// Runs a WebAssembly test script file
    #[cfg(feature = "wast")]
    Wast(wasmtime_cli::commands::WastCommand),
}

impl Wasmtime {
    /// Executes the command.
    pub fn execute(self) -> Result<()> {
        #[cfg(feature = "run")]
        let subcommand = self.subcommand.unwrap_or(Subcommand::Run(self.run));
        #[cfg(not(feature = "run"))]
        let subcommand = self.subcommand;

        match subcommand {
            #[cfg(feature = "run")]
            Subcommand::Run(c) => c.execute(),

            #[cfg(feature = "cache")]
            Subcommand::Config(c) => c.execute(),

            #[cfg(feature = "compile")]
            Subcommand::Compile(c) => c.execute(),

            #[cfg(feature = "explore")]
            Subcommand::Explore(c) => c.execute(),

            #[cfg(feature = "serve")]
            Subcommand::Serve(c) => c.execute(),

            #[cfg(feature = "cranelift")]
            Subcommand::Settings(c) => c.execute(),

            #[cfg(feature = "wast")]
            Subcommand::Wast(c) => c.execute(),
        }
    }
}

fn main() -> Result<()> {
    #[cfg(feature = "old-cli")]
    return old_cli::main();

    #[cfg(not(feature = "old-cli"))]
    return Wasmtime::parse().execute();
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Wasmtime::command().debug_assert()
}

#[cfg(feature = "old-cli")]
mod old_cli {
    use crate::Wasmtime;
    use anyhow::{bail, Result};
    use clap::error::ErrorKind;
    use clap::Parser;
    use wasmtime_cli::old_cli as old;

    enum WhichCli {
        Old,
        New,
        Unspecified,
    }

    const DEFAULT_OLD_BEHAVIOR: bool = true;

    fn which_cli() -> Result<WhichCli> {
        Ok(match std::env::var("WASMTIME_NEW_CLI") {
            Ok(s) if s == "0" => WhichCli::Old,
            Ok(s) if s == "1" => WhichCli::New,
            Ok(_) => bail!("the `WASMTIME_NEW_CLI` should either be `0` or `1`"),
            Err(_) => WhichCli::Unspecified,
        })
    }

    pub fn main() -> Result<()> {
        match which_cli()? {
            // If the old or the new CLI is explicitly selected, then run that
            // variant no questions asked.
            WhichCli::New => return Wasmtime::parse().execute(),
            WhichCli::Old => return try_parse_old().unwrap_or_else(|e| e.exit()).execute(),

            // fall through below to run an unspecified version of the CLI
            WhichCli::Unspecified => {}
        }

        // Here it's not specified which version of the CLI should be used, so
        // both the old and the new CLI parsers are used and depending on the
        // results an execution happens.
        let new = Wasmtime::try_parse();
        let old = try_parse_old();
        match (new, old) {
            // Both parsers succeeded. This means that it's likely that no
            // options to configure execution, e.g. `--disable-logging`, were
            // used in the old parser or the new.
            //
            // The result of parsing can still differ though. For example:
            //
            //      wasmtime foo.wasm --invoke foo
            //
            // would previously be parsed as passing `--invoke` to Wasmtime but
            // the new parser instead parses that as passing the flag to
            // `foo.wasm`.
            //
            // In this situation use the `DEFAULT_OLD_BEHAVIOR` constant to
            // dictate which wins and additionally print a warning message.
            (Ok(new), Ok(old)) => {
                if new == old {
                    return new.execute();
                }
                if DEFAULT_OLD_BEHAVIOR {
                    eprintln!(
                        "\
warning: this CLI invocation of Wasmtime will be parsed differently in future
         Wasmtime versions -- see this online issue for more information:
         https://github.com/bytecodealliance/wasmtime/issues/7384

         Wasmtime will now execute with the old (<= Wasmtime 13) CLI parsing,
         however this behavior can also be temporarily configured with an
         environment variable:

         - WASMTIME_NEW_CLI=0 to indicate old semantics are desired and silence this warning, or
         - WASMTIME_NEW_CLI=1 to indicate new semantics are desired and use the latest behavior\
"
                    );
                    old.execute()
                } else {
                    // this error message is not statically reachable due to
                    // `DEFAULT_OLD_BEHAVIOR=true` at this time, but when that
                    // changes this should be updated to have a more accurate
                    // set of text.
                    assert!(false);
                    eprintln!(
                        "\
warning: this CLI invocation of Wasmtime is parsed differently than it was
         previously -- see this online issue for more information:
         https://github.com/bytecodealliance/wasmtime/issues/7384

         Wasmtime will now execute with the new (>= Wasmtime XXX) CLI parsing,
         however this behavior can also be temporarily configured with an
         environment variable:

         - WASMTIME_NEW_CLI=0 to indicate old semantics are desired instead of the new, or
         - WASMTIME_NEW_CLI=1 to indicate new semantics are desired and silences this warning\
"
                    );
                    new.execute()
                }
            }

            // Here the new parser succeeded where the old one failed. This
            // could indicate for example that new options are being passed on
            // the CLI.
            //
            // In this situation assume that the new parser is what's intended
            // so execute those semantics.
            (Ok(new), Err(_old)) => new.execute(),

            // Here the new parser failed and the old parser succeeded. This
            // could indicate for example passing old CLI flags.
            //
            // Here assume that the old semantics are desired but emit a warning
            // indicating that this will change in the future.
            (Err(_new), Ok(old)) => {
                eprintln!(
                    "\
warning: this CLI invocation of Wasmtime is going to break in the future -- for
         more information see this issue online:
         https://github.com/bytecodealliance/wasmtime/issues/7384

         Wasmtime will now execute with the old (<= Wasmtime 13) CLI parsing,
         however this behavior can also be temporarily configured with an
         environment variable:

         - WASMTIME_NEW_CLI=0 to indicate old semantics are desired and silence this warning, or
         - WASMTIME_NEW_CLI=1 to indicate new semantics are desired and see the error\
"
                );
                old.execute()
            }

            // Both parsers failed to parse the CLI invocation.
            //
            // This could mean that someone manually passed an old flag
            // incorrectly. This could also mean that a new flag was passed
            // incorrectly. Clap also models `--help` requests as errors here so
            // this could also mean that a `--help` flag was passed.
            //
            // The current assumption in any case is that there's a human
            // interacting with the CLI at this point. They may or may not be
            // aware of the old CLI vs new CLI but if we're going to print an
            // error message then now's probably as good a time as any to nudge
            // them towards the new CLI. Any preexisting scripts which parsed
            // the old CLI should not hit this case which means that all old
            // successful parses will not go through here.
            //
            // In any case, display the error for the new CLI, including new
            // help text.
            (Err(new), Err(_old)) => new.exit(),
        }
    }

    fn try_parse_old() -> clap::error::Result<Wasmtime> {
        match old::Wasmtime::try_parse() {
            Ok(old) => Ok(convert(old)),
            Err(e) => {
                if let ErrorKind::InvalidSubcommand | ErrorKind::UnknownArgument = e.kind() {
                    if let Ok(run) = old::RunCommand::try_parse() {
                        return Ok(Wasmtime {
                            subcommand: None,
                            run: run.convert(),
                        });
                    }
                }
                Err(e)
            }
        }
    }

    fn convert(old: old::Wasmtime) -> Wasmtime {
        let subcommand = match old {
            old::Wasmtime::Compile(c) => crate::Subcommand::Compile(c.convert()),
            old::Wasmtime::Run(c) => crate::Subcommand::Run(c.convert()),
        };
        let mut run = wasmtime_cli::commands::RunCommand::parse_from::<_, &str>(["x", "y"]);
        run.module_and_args = Vec::new();
        Wasmtime {
            subcommand: Some(subcommand),
            run,
        }
    }
}
