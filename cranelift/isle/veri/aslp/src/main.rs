use anyhow::Result;
use clap::Parser as ClapParser;
use cranelift_isle_veri_aslp::parser;
use std::{fs, path::PathBuf};

#[derive(ClapParser)]
#[command(version, about)]
struct Args {
    /// Input file to be formatted
    file: PathBuf,

    /// Print debugging output (repeat for more detail)
    #[arg(short = 'd', long = "debug", action = clap::ArgAction::Count)]
    debug_level: u8,
}

fn main() -> Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_timer(tracing_subscriber::fmt::time::uptime())
        .with_level(true)
        .with_target(false)
        .with_max_level(match args.debug_level {
            0 => tracing::Level::WARN,
            1 => tracing::Level::INFO,
            2 => tracing::Level::DEBUG,
            _ => tracing::Level::TRACE,
        })
        .init();

    let src = fs::read_to_string(args.file).unwrap();

    let block = parser::parse(&src)?;
    println!("ast = {block:?}");

    Ok(())
}
