use libtest_mimic::{Arguments, Failed, Trial};

fn main() -> std::process::ExitCode {
    let args = Arguments::from_args();

    let tests = vec![Trial::test("filetests", filetests)];

    libtest_mimic::run(&args, tests).exit()
}

fn filetests() -> Result<(), Failed> {
    // Run all the filetests in the following directories.
    cranelift_filetests::run(false, false, &["filetests".into(), "docs".into()])
        .map_err(Failed::from)?;
    Ok(())
}
