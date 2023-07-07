//! Test runner for `.wat` files to exercise CLIF-to-Wasm translations.

mod config;
mod env;

use anyhow::{bail, ensure, Context, Result};
use config::TestConfig;
use env::ModuleEnv;
use similar::TextDiff;
use std::{fmt::Write, path::Path};

/// Run one `.wat` test.
pub fn run(path: &Path, wat: &str) -> Result<()> {
    debug_assert_eq!(path.extension().unwrap_or_default(), "wat");

    // The test config source is the leading lines of the WAT file that are
    // prefixed with `;;!`.
    let config_lines: Vec<_> = wat
        .lines()
        .take_while(|l| l.starts_with(";;!"))
        .map(|l| &l[3..])
        .collect();
    let config_text = config_lines.join("\n");

    let config: TestConfig =
        toml::from_str(&config_text).context("failed to parse the test configuration")?;
    log::debug!("Wasm test config = {config:#?}");

    config
        .validate()
        .context("test configuration is malformed")?;

    let parsed = cranelift_reader::parse_sets_and_triple(&config.settings, &config.target)
        .context("invalid ISA target or Cranelift settings")?;
    let flags_or_isa = parsed.as_fisa();
    ensure!(
        flags_or_isa.isa.is_some(),
        "Running `.wat` tests requires specifying an ISA"
    );
    let isa = flags_or_isa.isa.unwrap();

    let mut env = ModuleEnv::new(isa, config.clone());

    let wasm = wat::parse_str(wat).context("failed to parse the test WAT")?;
    let mut validator = wasmparser::Validator::new_with_features(
        cranelift_wasm::ModuleEnvironment::wasm_features(&env),
    );
    validator
        .validate_all(&wasm)
        .context("test WAT failed to validate")?;

    cranelift_wasm::translate_module(&wasm, &mut env)
        .context("failed to translate the test case into CLIF")?;

    let mut actual = String::new();
    for (_index, func) in env.inner.info.function_bodies.iter() {
        if config.compile {
            let mut ctx = cranelift_codegen::Context::for_function(func.clone());
            ctx.set_disasm(true);
            let code = ctx
                .compile(isa, &mut Default::default())
                .map_err(|e| crate::pretty_anyhow_error(&e.func, e.inner))?;
            writeln!(&mut actual, "function {}:", func.name).unwrap();
            writeln!(&mut actual, "{}", code.vcode.as_ref().unwrap()).unwrap();
        } else if config.optimize {
            let mut ctx = cranelift_codegen::Context::for_function(func.clone());
            ctx.optimize(isa)
                .map_err(|e| crate::pretty_anyhow_error(&ctx.func, e))?;
            writeln!(&mut actual, "{}", ctx.func.display()).unwrap();
        } else {
            writeln!(&mut actual, "{}", func.display()).unwrap();
        }
    }
    let actual = actual.trim();
    log::debug!("=== actual ===\n{actual}");

    // The test's expectation is the final comment.
    let mut expected_lines: Vec<_> = wat
        .lines()
        .rev()
        .take_while(|l| l.starts_with(";;"))
        .map(|l| {
            if l.starts_with(";; ") {
                &l[3..]
            } else {
                &l[2..]
            }
        })
        .collect();
    expected_lines.reverse();
    let expected = expected_lines.join("\n");
    let expected = expected.trim();
    log::debug!("=== expected ===\n{expected}");

    if actual == expected {
        return Ok(());
    }

    if std::env::var("CRANELIFT_TEST_BLESS").unwrap_or_default() == "1" {
        let old_expectation_line_count = wat
            .lines()
            .rev()
            .take_while(|l| l.starts_with(";;"))
            .count();
        let old_wat_line_count = wat.lines().count();
        let new_wat_lines: Vec<_> = wat
            .lines()
            .take(old_wat_line_count - old_expectation_line_count)
            .map(|l| l.to_string())
            .chain(actual.lines().map(|l| {
                if l.is_empty() {
                    ";;".to_string()
                } else {
                    format!(";; {l}")
                }
            }))
            .collect();
        let mut new_wat = new_wat_lines.join("\n");
        new_wat.push('\n');
        std::fs::write(path, new_wat)
            .with_context(|| format!("failed to write file: {}", path.display()))?;
        return Ok(());
    }

    bail!(
        "Did not get the expected CLIF translation:\n\n\
         {}\n\n\
         Note: You can re-run with the `CRANELIFT_TEST_BLESS=1` environment\n\
         variable set to update test expectations.",
        TextDiff::from_lines(expected, actual)
            .unified_diff()
            .header("expected", "actual")
    )
}
