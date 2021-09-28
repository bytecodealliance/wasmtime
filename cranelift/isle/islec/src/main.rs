use clap::{App, Arg};
use isle::{compile, lexer, parser};
use miette::{IntoDiagnostic, Result};

fn main() -> Result<()> {
    let _ = env_logger::try_init();

    let _ = miette::set_hook(Box::new(|_| {
        Box::new(
            miette::MietteHandlerOpts::new()
                // `miette` mistakenly uses braille-optimized output for emacs's
                // `M-x shell`.
                .force_graphical(true)
                .build(),
        )
    }));

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
    let defs = parser.parse_defs()?;
    let code = compile::compile(&defs)?;

    {
        use std::io::Write;
        let mut f = std::fs::File::create(output_file).into_diagnostic()?;
        writeln!(&mut f, "{}", code).into_diagnostic()?;
    }

    Ok(())
}
