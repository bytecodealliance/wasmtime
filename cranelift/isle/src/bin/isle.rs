use clap::{App, Arg};

use isle::{error, lexer, parser, compile};

fn main() -> Result<(), error::Error> {
    let _ = env_logger::try_init();

    let matches = App::new("isle")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Chris Fallin <chris@cfallin.org>")
        .about("Instruction selection logic engine (ISLE) code generator")
        .arg(
            Arg::with_name("input")
                .short("i")
                .long("input")
                .value_name("FILE.isle")
                .takes_value(true)
                .multiple(true)
                .required(true),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .value_name("FILE.rs")
                .takes_value(true)
                .required(true),
        )
        .get_matches();

    let input_files = matches
        .values_of("input")
        .unwrap()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let output_file = matches.value_of("output").unwrap();

    let lexer = lexer::Lexer::from_files(input_files)?;
    let mut parser = parser::Parser::new(lexer);
    let defs = match parser.parse_defs() {
        Ok(defs) => defs,
        Err(error) => {
            eprintln!("{}", error);
            eprintln!("Failed to parse input.");
            std::process::exit(1);
        }
    };
    let code = match compile::compile(&defs) {
        Ok(code) => code,
        Err(errors) => {
            for error in errors {
                eprintln!("{}", error);
            }
            eprintln!("Failed to compile.");
            std::process::exit(1);
        }
    };

    {
        use std::io::Write;
        let mut f = std::fs::File::create(output_file)?;
        writeln!(&mut f, "{}", code)?;
    }

    Ok(())
}

