//! CLI tool to read Cranelift IR files and compile them into native code.

use crate::disasm::print_all;
use crate::run::architectures_compatible;
use crate::utils::read_to_string;
use anyhow::{Context as _, Result};
use clap::Parser;
use cranelift_codegen::Context;
use cranelift_codegen::isa::{self, OwnedTargetIsa, TargetIsa};
use cranelift_codegen::print_errors::pretty_error;
use cranelift_codegen::settings::{self, Configurable, Flags};
use cranelift_codegen::timing;
use cranelift_module::Module;
use cranelift_reader::{
    IsaSpec, Location, ParseError, ParseOptionError, ParseOptions, parse_option, parse_options,
    parse_test,
};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use target_lexicon::{Architecture, Triple};

/// Compiles Cranelift IR into target language
#[derive(Parser)]
pub struct Options {
    /// Print the resulting Cranelift IR
    #[arg(short)]
    print: bool,

    /// Print pass timing report
    #[arg(short = 'T')]
    report_times: bool,

    /// Print machine code disassembly
    #[arg(short = 'D', long)]
    disasm: bool,

    /// Configure Cranelift settings.
    ///
    /// Settings in the input file are the defaults. Each value here overrides
    /// the file, including an ISA-specific flag for the selected target.
    #[arg(long = "set")]
    settings: Vec<String>,

    /// Specify the Cranelift target.
    ///
    /// An empty string selects the file's unique target. Otherwise this triple
    /// selects a matching file target: an exact architecture wins, then a
    /// compatible RISC-V variant. The selected target's settings are kept, and
    /// options after the triple override them. Explicit `--set` values and
    /// those target options win over the file. When nothing in the file
    /// matches, only the command-line target is used.
    #[arg(long = "target")]
    target: String,

    /// Specify an input file to be used. Use '-' for stdin.
    files: Vec<PathBuf>,

    /// Output object file
    #[arg(short = 'o', long = "output")]
    output: Option<PathBuf>,
}

struct CliTarget {
    /// Word the user passed before any inline target options.
    token: String,
    triple: Triple,
}

struct CliRequest {
    target: Option<CliTarget>,
    /// Words after the triple in `--target`.
    inline_options: Vec<String>,
    settings: Vec<String>,
}

impl CliRequest {
    fn parse(settings: &[String], target: &str) -> Result<Self> {
        let mut words = target.trim().split_whitespace();
        let target = match words.next() {
            Some(token) => Some(CliTarget {
                token: token.to_string(),
                triple: Triple::from_str(token)?,
            }),
            None => None,
        };
        let cli = Self {
            target,
            inline_options: words.map(ToOwned::to_owned).collect(),
            settings: settings.to_vec(),
        };
        cli.validate()?;
        Ok(cli)
    }

    fn validate(&self) -> Result<()> {
        if let Some(target) = &self.target {
            // Reject a bad triple or flag before reading inputs.
            build_isa(&target.triple, &target.token, None, None, self)?;
            return Ok(());
        }

        let mut unknown_settings = Vec::new();
        let mut flag_builder = settings::builder();
        for flag in &self.settings {
            match parse_option(flag, &mut flag_builder, Location { line_number: 0 }) {
                Ok(()) => {}
                Err(ParseOptionError::UnknownFlag { name, .. }) => unknown_settings.push(name),
                Err(ParseOptionError::UnknownValue { name, value, .. }) => {
                    unknown_settings.push(format!("{name}={value}"));
                }
                Err(ParseOptionError::Generic(err)) => return Err(err.into()),
            }
        }
        if !unknown_settings.is_empty() {
            anyhow::bail!("unknown settings: '{}'", unknown_settings.join("', '"));
        }
        Ok(())
    }
}

pub fn run(options: &Options) -> Result<()> {
    let cli = CliRequest::parse(&options.settings, &options.target)?;
    let mut object_module: Option<cranelift_object::ObjectModule> = None;

    for path in &options.files {
        let name = String::from(path.as_os_str().to_string_lossy());
        let buffer = read_to_string(path)?;
        let test_file = parse_test(&buffer, ParseOptions::default())
            .with_context(|| format!("failed to parse {name}"))?;
        let isa = resolve_compile_isa(&cli, &test_file.isa_spec, &name)?;

        if let Some(output) = &options.output {
            if let Some(existing) = &object_module {
                if !same_target_config(existing.isa(), isa.as_ref()) {
                    anyhow::bail!(
                        "{name}: resolved target configuration is incompatible with the object output"
                    );
                }
            } else {
                object_module = Some(new_object_module(&isa, output)?);
            }
        }

        compile_functions(options, &test_file, &isa, &mut object_module)?;
    }

    if let (Some(module), Some(output)) = (object_module, &options.output) {
        let bytes = module.finish().emit()?;
        std::fs::write(output, bytes)?;
    }

    Ok(())
}

fn new_object_module(
    isa: &OwnedTargetIsa,
    output: &Path,
) -> Result<cranelift_object::ObjectModule> {
    let builder = cranelift_object::ObjectBuilder::new(
        isa.clone(),
        output
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("a.out"),
        cranelift_module::default_libcall_names(),
    )?;
    Ok(cranelift_object::ObjectModule::new(builder))
}

/// Resolve the ISA used to compile one input file.
///
/// File settings are the defaults. An explicit command-line triple keeps its
/// operating system and calling convention, and takes shared and ISA settings
/// from the matching file target. `--set` and inline `--target` options are
/// applied last.
fn resolve_compile_isa(cli: &CliRequest, spec: &IsaSpec, file: &str) -> Result<OwnedTargetIsa> {
    match &cli.target {
        None => match spec {
            IsaSpec::None(_) => anyhow::bail!("{file}: compilation requires a target isa"),
            IsaSpec::Some(isas) if isas.len() == 1 => {
                let isa = &*isas[0];
                let token = isa.triple().to_string();
                build_isa(isa.triple(), &token, None, Some(isa), cli)
            }
            IsaSpec::Some(isas) if isas.is_empty() => {
                anyhow::bail!("{file}: compilation requires a target isa")
            }
            IsaSpec::Some(_) => anyhow::bail!(
                "{file}: multiple target configurations match; pass --target to select one"
            ),
        },
        Some(target) => match spec {
            IsaSpec::None(flags) => {
                build_isa(&target.triple, &target.token, Some(flags), None, cli)
            }
            IsaSpec::Some(isas) => {
                let selected = select_file_target(file, target.triple.architecture, isas)?;
                build_isa(&target.triple, &target.token, None, selected, cli)
            }
        },
    }
}

fn select_file_target<'a>(
    file: &str,
    requested: Architecture,
    isas: &'a [OwnedTargetIsa],
) -> Result<Option<&'a dyn TargetIsa>> {
    let exact = isas
        .iter()
        .filter(|isa| isa.triple().architecture == requested)
        .collect::<Vec<_>>();
    if exact.len() > 1 {
        anyhow::bail!("{file}: multiple target configurations match {requested}");
    }
    if let Some(isa) = exact.first() {
        return Ok(Some((*isa).as_ref()));
    }

    let family = isas
        .iter()
        .filter(|isa| architectures_compatible(requested, isa.triple().architecture))
        .collect::<Vec<_>>();
    if family.len() > 1 {
        anyhow::bail!("{file}: multiple target configurations match {requested}");
    }
    Ok(family.first().map(|isa| (*isa).as_ref() as &dyn TargetIsa))
}

fn build_isa(
    triple: &Triple,
    triple_name: &str,
    shared_flags: Option<&Flags>,
    file_isa: Option<&dyn TargetIsa>,
    cli: &CliRequest,
) -> Result<OwnedTargetIsa> {
    let mut flag_builder = settings::builder();
    if let Some(isa) = file_isa {
        copy_values(isa.flags().iter(), &mut flag_builder)?;
    } else if let Some(flags) = shared_flags {
        copy_values(flags.iter(), &mut flag_builder)?;
    }

    let mut isa_builder = lookup_isa(triple_name, triple)?;
    if let Some(isa) = file_isa {
        copy_values(isa.isa_flags(), &mut isa_builder)?;
    }
    apply_cli_options(cli, &mut flag_builder, &mut isa_builder)?;
    Ok(isa_builder.finish(settings::Flags::new(flag_builder))?)
}

fn lookup_isa(name: &str, triple: &Triple) -> Result<isa::Builder> {
    isa::lookup(triple.clone()).map_err(|err| match err {
        isa::LookupError::SupportDisabled => {
            anyhow::anyhow!("support for triple '{name}' is disabled")
        }
        isa::LookupError::Unsupported => {
            anyhow::anyhow!("support for triple '{name}' is not implemented yet")
        }
    })
}

fn copy_values(
    values: impl IntoIterator<Item = settings::Value>,
    builder: &mut dyn Configurable,
) -> Result<()> {
    for value in values {
        let rendered = value.value_string();
        builder
            .set(value.name, &rendered)
            .with_context(|| format!("setting {}={rendered}", value.name))?;
    }
    Ok(())
}

fn apply_cli_options(
    cli: &CliRequest,
    shared: &mut settings::Builder,
    isa_builder: &mut isa::Builder,
) -> Result<()> {
    let mut unknown_settings = Vec::new();
    for flag in &cli.settings {
        match parse_option(flag, shared, Location { line_number: 0 }) {
            Ok(()) => {}
            Err(ParseOptionError::UnknownFlag { name, .. }) => unknown_settings.push(name),
            Err(ParseOptionError::UnknownValue { name, value, .. }) => {
                unknown_settings.push(format!("{name}={value}"));
            }
            Err(ParseOptionError::Generic(err)) => return Err(err.into()),
        }
    }
    parse_options(
        unknown_settings.iter().map(String::as_str),
        isa_builder,
        Location { line_number: 0 },
    )
    .map_err(ParseError::from)?;
    parse_options(
        cli.inline_options.iter().map(String::as_str),
        isa_builder,
        Location { line_number: 0 },
    )
    .map_err(ParseError::from)?;
    Ok(())
}

fn same_target_config(a: &dyn TargetIsa, b: &dyn TargetIsa) -> bool {
    a.triple() == b.triple() && a.flags() == b.flags() && isa_flag_strings(a) == isa_flag_strings(b)
}

fn isa_flag_strings(isa: &dyn TargetIsa) -> Vec<(&'static str, String)> {
    isa.isa_flags()
        .iter()
        .map(|value| (value.name, value.value_string()))
        .collect()
}

fn compile_functions(
    options: &Options,
    test_file: &cranelift_reader::TestFile<'_>,
    isa: &OwnedTargetIsa,
    object_module: &mut Option<cranelift_object::ObjectModule>,
) -> Result<()> {
    for (func, _) in &test_file.functions {
        let mut context = Context::new();
        context.func = func.clone();

        // Compile and encode the result to machine code.
        let compiled_code = context
            .compile(isa.as_ref(), &mut Default::default())
            .map_err(|err| anyhow::anyhow!("{}", pretty_error(&err.func, err.inner)))?;
        let code_info = compiled_code.code_info();

        if let Some(module) = object_module.as_mut() {
            let name = context.func.name.to_string();
            let fid = module.declare_function(
                &name,
                cranelift_module::Linkage::Export,
                &context.func.signature,
            )?;
            module.define_function_with_control_plane(
                fid,
                &mut context,
                &mut Default::default(),
            )?;
        }

        if options.print {
            println!("{}", context.func.display());
        }

        if options.disasm {
            let result = context.compiled_code().unwrap();
            print_all(
                isa.as_ref(),
                &context.func,
                context.compiled_code().unwrap().code_buffer(),
                code_info.total_size,
                options.print,
                result.buffer.relocs(),
                result.buffer.traps(),
            )?;
        }
    }

    if options.report_times {
        print!("{}", timing::take_current());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::settings::OptLevel;

    const VECTOR_RETURN: &str = "\
set enable_multi_ret_implicit_sret=true
target riscv64 has_v

function %vec_ret() -> i8x16 {
block0:
    v0 = vconst.i8x16 0x00
    return v0
}
";

    fn resolve(target: &str, settings: &[&str], clif: &str) -> Result<OwnedTargetIsa> {
        let settings = settings
            .iter()
            .map(|s| (*s).to_string())
            .collect::<Vec<_>>();
        let cli = CliRequest::parse(&settings, target)?;
        let parsed = parse_test(clif, ParseOptions::default())?;
        resolve_compile_isa(&cli, &parsed.isa_spec, "input.clif")
    }

    fn isa_bool(isa: &OwnedTargetIsa, name: &str) -> bool {
        isa.isa_flags()
            .iter()
            .find(|value| value.name == name)
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    }

    fn err_string<T>(result: Result<T>) -> String {
        match result {
            Ok(_) => panic!("expected an error"),
            Err(err) => err.to_string(),
        }
    }

    fn compile_clif(clif: &str, isa: &dyn TargetIsa) -> Result<()> {
        let parsed = parse_test(clif, ParseOptions::default())?;
        let mut context = Context::new();
        context.func = parsed.functions.into_iter().next().expect("function").0;
        context
            .compile(isa, &mut Default::default())
            .map(|_| ())
            .map_err(|err| anyhow::anyhow!("{}", pretty_error(&err.func, err.inner)))
    }

    #[test]
    fn vector_return_keeps_file_settings() {
        let isa = resolve("riscv64", &[], VECTOR_RETURN).unwrap();
        assert!(isa.flags().enable_multi_ret_implicit_sret());
        assert!(isa_bool(&isa, "has_v"));
        assert_eq!(isa.triple(), &Triple::from_str("riscv64").unwrap());
        compile_clif(VECTOR_RETURN, isa.as_ref()).expect("file settings compile");

        let bare = isa::lookup(Triple::from_str("riscv64").unwrap())
            .unwrap()
            .finish(settings::Flags::new(settings::builder()))
            .unwrap();
        assert!(compile_clif(VECTOR_RETURN, bare.as_ref()).is_err());
    }

    #[test]
    fn cli_overrides_win_and_unrelated_file_settings_remain() {
        let clif = "\
set opt_level=speed
set is_pic
target riscv64 has_v has_zbb

function %t() -> i64 {
block0:
    v0 = iconst.i64 1
    return v0
}
";
        let isa = resolve("riscv64 has_zbb=false", &["opt_level=none"], clif).unwrap();
        assert_eq!(isa.flags().opt_level(), OptLevel::None);
        assert!(isa.flags().is_pic());
        assert!(isa_bool(&isa, "has_v"));
        assert!(!isa_bool(&isa, "has_zbb"));

        let isa = resolve("riscv64", &["opt_level=none", "has_zbb=false"], clif).unwrap();
        assert_eq!(isa.flags().opt_level(), OptLevel::None);
        assert!(isa.flags().is_pic());
        assert!(isa_bool(&isa, "has_v"));
        assert!(!isa_bool(&isa, "has_zbb"));
    }

    #[test]
    fn invalid_settings_and_targets_have_errors() {
        let missing = err_string(CliRequest::parse(&["not_a_real_setting".into()], "riscv64"));
        assert!(missing.contains("not_a_real_setting"), "{missing}");

        let bad_value = err_string(CliRequest::parse(&["opt_level=banana".into()], "riscv64"));
        assert!(bad_value.contains("opt_level=banana"), "{bad_value}");

        let bad_inline = err_string(CliRequest::parse(&[], "riscv64 not_a_flag"));
        assert!(bad_inline.contains("not_a_flag"), "{bad_inline}");

        let unsupported = err_string(CliRequest::parse(&[], "riscv32"));
        assert!(unsupported.contains("riscv32"), "{unsupported}");

        let unknown = err_string(CliRequest::parse(&[], "not-a-triple"));
        assert!(!unknown.is_empty(), "{unknown}");
    }

    #[test]
    fn target_matching_is_exact_then_riscv_family() {
        let clif = "\
set opt_level=speed
target x86_64 has_sse41
target riscv64gc has_v

function %t() -> i64 {
block0:
    v0 = iconst.i64 1
    return v0
}
";
        let isa = resolve("riscv64", &[], clif).unwrap();
        assert_eq!(isa.triple(), &Triple::from_str("riscv64").unwrap());
        assert_eq!(isa.flags().opt_level(), OptLevel::Speed);
        assert!(isa_bool(&isa, "has_v"));
        assert!(!isa_bool(&isa, "has_sse41"));

        let isa = resolve("x86_64", &[], clif).unwrap();
        assert_eq!(isa.flags().opt_level(), OptLevel::Speed);
        assert!(isa_bool(&isa, "has_sse41"));

        let prefer_exact = "\
target riscv64gc has_zbb
target riscv64 has_v

function %t() -> i64 {
block0:
    v0 = iconst.i64 1
    return v0
}
";
        let isa = resolve("riscv64", &[], prefer_exact).unwrap();
        assert!(isa_bool(&isa, "has_v"));
        assert!(!isa_bool(&isa, "has_zbb"));

        let linux = resolve("riscv64-unknown-linux-gnu", &[], prefer_exact).unwrap();
        assert_eq!(
            linux.triple(),
            &Triple::from_str("riscv64-unknown-linux-gnu").unwrap()
        );
        assert!(isa_bool(&linux, "has_v"));

        let ambiguous = "\
target riscv64 has_v
target riscv64 has_zbb

function %t() -> i64 {
block0:
    v0 = iconst.i64 1
    return v0
}
";
        let err = err_string(resolve("riscv64", &[], ambiguous));
        assert!(err.contains("input.clif"), "{err}");
        assert!(err.contains("multiple target configurations"), "{err}");

        let ambiguous_family = "\
target riscv64gc has_v
target riscv64imac has_zbb

function %t() -> i64 {
block0:
    v0 = iconst.i64 1
    return v0
}
";
        let err = err_string(resolve("riscv64", &[], ambiguous_family));
        assert!(err.contains("input.clif"), "{err}");
        assert!(err.contains("multiple target configurations"), "{err}");
    }

    #[test]
    fn shared_settings_follow_the_selected_target_snapshot() {
        let clif = "\
set opt_level=speed
target riscv64 has_v
set is_pic
target x86_64 has_sse41

function %t() -> i64 {
block0:
    v0 = iconst.i64 1
    return v0
}
";
        let riscv = resolve("riscv64", &[], clif).unwrap();
        assert_eq!(riscv.flags().opt_level(), OptLevel::Speed);
        assert!(!riscv.flags().is_pic());
        assert!(isa_bool(&riscv, "has_v"));

        let x64 = resolve("x86_64", &[], clif).unwrap();
        assert_eq!(x64.flags().opt_level(), OptLevel::Speed);
        assert!(x64.flags().is_pic());
        assert!(isa_bool(&x64, "has_sse41"));
    }

    #[test]
    fn file_without_target_seeds_shared_settings() {
        let clif = "\
set opt_level=speed
set enable_multi_ret_implicit_sret

function %t() -> i64 {
block0:
    v0 = iconst.i64 1
    return v0
}
";
        let isa = resolve("riscv64", &[], clif).unwrap();
        assert_eq!(isa.flags().opt_level(), OptLevel::Speed);
        assert!(isa.flags().enable_multi_ret_implicit_sret());
        assert_eq!(isa.triple(), &Triple::from_str("riscv64").unwrap());
    }

    #[test]
    fn empty_target_uses_the_unique_file_target() {
        let clif = "\
set opt_level=speed
target riscv64 has_v

function %t() -> i64 {
block0:
    v0 = iconst.i64 1
    return v0
}
";
        let isa = resolve("", &[], clif).unwrap();
        assert_eq!(
            isa.triple().architecture,
            Architecture::Riscv64(target_lexicon::Riscv64Architecture::Riscv64)
        );
        assert_eq!(isa.flags().opt_level(), OptLevel::Speed);
        assert!(isa_bool(&isa, "has_v"));

        let overridden = resolve("", &["opt_level=none"], clif).unwrap();
        assert_eq!(overridden.flags().opt_level(), OptLevel::None);
        assert!(isa_bool(&overridden, "has_v"));

        let many = "\
target riscv64 has_v
target x86_64

function %t() -> i64 {
block0:
    v0 = iconst.i64 1
    return v0
}
";
        let err = err_string(resolve("", &[], many));
        assert!(err.contains("input.clif"), "{err}");
        assert!(err.contains("multiple target configurations"), "{err}");

        let none = "\
function %t() -> i64 {
block0:
    v0 = iconst.i64 1
    return v0
}
";
        let err = err_string(resolve("", &[], none));
        assert!(err.contains("input.clif"), "{err}");
        assert!(err.contains("compilation requires a target isa"), "{err}");
    }

    #[test]
    fn unmatched_file_target_does_not_donate_settings() {
        let clif = "\
set opt_level=speed
target x86_64 has_sse41

function %t() -> i64 {
block0:
    v0 = iconst.i64 1
    return v0
}
";
        let isa = resolve("riscv64", &[], clif).unwrap();
        assert_eq!(isa.triple(), &Triple::from_str("riscv64").unwrap());
        assert_eq!(isa.flags().opt_level(), OptLevel::None);
        assert!(!isa_bool(&isa, "has_v"));
        assert!(!isa.flags().is_pic());
    }
}
