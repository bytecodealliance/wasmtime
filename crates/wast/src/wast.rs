use crate::spectest::instantiate_spectest;
use anyhow::{anyhow, bail, Context as _, Result};
use std::collections::HashMap;
use std::path::Path;
use std::str;
use wasmtime::*;
use wast::parser::{Parse, Parser};

/// Translate from a `script::Value` to a `RuntimeValue`.
fn runtime_value(v: &wast::Expression<'_>) -> Result<Val> {
    use wast::Instruction::*;

    if v.instrs.len() != 1 {
        bail!("too many instructions in {:?}", v);
    }
    Ok(match &v.instrs[0] {
        I32Const(x) => Val::I32(*x),
        I64Const(x) => Val::I64(*x),
        F32Const(x) => Val::F32(x.bits),
        F64Const(x) => Val::F64(x.bits),
        V128Const(x) => Val::V128(u128::from_le_bytes(x.to_le_bytes())),
        other => bail!("couldn't convert {:?} to a runtime value", other),
    })
}

/// The wast test script language allows modules to be defined and actions
/// to be performed on them.
pub struct WastContext {
    /// Wast files have a concept of a "current" module, which is the most
    /// recently defined.
    current: Option<Instance>,

    instances: HashMap<String, Instance>,
    store: Store,
    spectest: Option<HashMap<&'static str, Extern>>,
}

enum Outcome<T = Vec<Val>> {
    Ok(T),
    Trap(Trap),
}

impl<T> Outcome<T> {
    fn into_result(self) -> Result<T, Trap> {
        match self {
            Outcome::Ok(t) => Ok(t),
            Outcome::Trap(t) => Err(t),
        }
    }
}

impl WastContext {
    /// Construct a new instance of `WastContext`.
    pub fn new(store: Store) -> Self {
        Self {
            current: None,
            store,
            spectest: None,
            instances: HashMap::new(),
        }
    }

    fn get_instance(&self, instance_name: Option<&str>) -> Result<Instance> {
        match instance_name {
            Some(name) => self
                .instances
                .get(name)
                .cloned()
                .ok_or_else(|| anyhow!("failed to find instance named `{}`", name)),
            None => self
                .current
                .clone()
                .ok_or_else(|| anyhow!("no previous instance found")),
        }
    }

    fn instantiate(&self, module: &[u8]) -> Result<Outcome<Instance>> {
        let module = Module::new(&self.store, module)?;
        let mut imports = Vec::new();
        for import in module.imports() {
            if import.module() == "spectest" {
                let spectest = self
                    .spectest
                    .as_ref()
                    .ok_or_else(|| anyhow!("spectest module isn't instantiated"))?;
                let export = spectest
                    .get(import.name())
                    .ok_or_else(|| anyhow!("unknown import `spectest::{}`", import.name()))?;
                imports.push(export.clone());
                continue;
            }

            let instance = self
                .instances
                .get(import.module())
                .ok_or_else(|| anyhow!("no module named `{}`", import.module()))?;
            let export = instance
                .get_export(import.name())
                .ok_or_else(|| anyhow!("unknown import `{}::{}`", import.name(), import.module()))?
                .clone();
            imports.push(export);
        }
        let instance = match Instance::new(&module, &imports) {
            Ok(i) => i,
            Err(e) => return e.downcast::<Trap>().map(Outcome::Trap),
        };
        Ok(Outcome::Ok(instance))
    }

    /// Register "spectest" which is used by the spec testsuite.
    pub fn register_spectest(&mut self) -> Result<()> {
        self.spectest = Some(instantiate_spectest(&self.store));
        Ok(())
    }

    /// Perform the action portion of a command.
    fn perform_execute(&mut self, exec: wast::WastExecute<'_>) -> Result<Outcome> {
        match exec {
            wast::WastExecute::Invoke(invoke) => self.perform_invoke(invoke),
            wast::WastExecute::Module(mut module) => {
                let binary = module.encode()?;
                let result = self.instantiate(&binary)?;
                Ok(match result {
                    Outcome::Ok(_) => Outcome::Ok(Vec::new()),
                    Outcome::Trap(e) => Outcome::Trap(e),
                })
            }
            wast::WastExecute::Get { module, global } => self.get(module.map(|s| s.name()), global),
        }
    }

    fn perform_invoke(&mut self, exec: wast::WastInvoke<'_>) -> Result<Outcome> {
        let values = exec
            .args
            .iter()
            .map(runtime_value)
            .collect::<Result<Vec<_>>>()?;
        self.invoke(exec.module.map(|i| i.name()), exec.name, &values)
    }

    /// Define a module and register it.
    fn module(&mut self, instance_name: Option<&str>, module: &[u8]) -> Result<()> {
        let instance = match self.instantiate(module)? {
            Outcome::Ok(i) => i,
            Outcome::Trap(e) => bail!("instantiation failed with: {}", e.message()),
        };
        if let Some(name) = instance_name {
            self.instances.insert(name.to_string(), instance.clone());
        }
        self.current = Some(instance);
        Ok(())
    }

    /// Register an instance to make it available for performing actions.
    fn register(&mut self, name: Option<&str>, as_name: &str) -> Result<()> {
        let instance = self.get_instance(name)?.clone();
        self.instances.insert(as_name.to_string(), instance);
        Ok(())
    }

    /// Invoke an exported function from an instance.
    fn invoke(
        &mut self,
        instance_name: Option<&str>,
        field: &str,
        args: &[Val],
    ) -> Result<Outcome> {
        let instance = self.get_instance(instance_name.as_ref().map(|x| &**x))?;
        let func = instance
            .get_export(field)
            .and_then(|e| e.func())
            .ok_or_else(|| anyhow!("no function named `{}`", field))?;
        Ok(match func.call(args) {
            Ok(result) => Outcome::Ok(result.into()),
            Err(e) => Outcome::Trap(e),
        })
    }

    /// Get the value of an exported global from an instance.
    fn get(&mut self, instance_name: Option<&str>, field: &str) -> Result<Outcome> {
        let instance = self.get_instance(instance_name.as_ref().map(|x| &**x))?;
        let global = instance
            .get_export(field)
            .and_then(|e| e.global())
            .ok_or_else(|| anyhow!("no global named `{}`", field))?;
        Ok(Outcome::Ok(vec![global.get()]))
    }

    fn assert_return(&self, result: Outcome, results: &[impl MatchesVal]) -> Result<()> {
        let values = result.into_result()?;
        if values.len() != results.len() {
            bail!("expected {} results, got {}", results.len(), values.len());
        }
        for (v, e) in values.iter().zip(results) {
            if e.matches(v)? {
                continue;
            }
            bail!("expected {:?}, got {:?}", e, v)
        }
        Ok(())
    }

    fn assert_trap(&self, result: Outcome, expected: &str) -> Result<()> {
        let trap = match result {
            Outcome::Ok(values) => bail!("expected trap, got {:?}", values),
            Outcome::Trap(t) => t,
        };
        let actual = trap.message();
        if actual.contains(expected)
            // `bulk-memory-operations/bulk.wast` checks for a message that
            // specifies which element is uninitialized, but our traps don't
            // shepherd that information out.
            || (expected.contains("uninitialized element 2") && actual.contains("uninitialized element"))
        {
            return Ok(());
        }
        if cfg!(feature = "lightbeam") {
            println!("TODO: Check the assert_trap message: {}", expected);
            return Ok(());
        }
        bail!("expected '{}', got '{}'", expected, actual)
    }

    /// Run a wast script from a byte buffer.
    pub fn run_buffer(&mut self, filename: &str, wast: &[u8]) -> Result<()> {
        let wast = str::from_utf8(wast)?;

        let adjust_wast = |mut err: wast::Error| {
            err.set_path(filename.as_ref());
            err.set_text(wast);
            err
        };

        let buf = wast::parser::ParseBuffer::new(wast).map_err(adjust_wast)?;
        let ast = wast::parser::parse::<Wast>(&buf).map_err(adjust_wast)?;

        for directive in ast.directives {
            let sp = directive.span();
            self.run_directive(directive).with_context(|| {
                let (line, col) = sp.linecol_in(wast);
                format!("failed directive on {}:{}:{}", filename, line + 1, col)
            })?;
        }
        Ok(())
    }

    fn run_directive(&mut self, directive: WastDirective) -> Result<()> {
        use WastDirective::*;

        match directive {
            WitModule(mut module) => {
                let binary = module.encode()?;
                self.module(module.core.id.map(|s| s.name()), &binary)?;
            }
            WitAssertReturn {
                exec,
                results,
                span: _,
            } => {
                let values = exec
                    .args
                    .iter()
                    .map(|e| e.to_val())
                    .collect::<Result<Vec<_>>>()?;
                let actual = self.invoke(None, exec.name, &values)?;
                self.assert_return(actual, &results)?;
            }
            WitAssertTrap {
                exec,
                message,
                span: _,
            } => {
                let values = exec
                    .args
                    .iter()
                    .map(|e| e.to_val())
                    .collect::<Result<Vec<_>>>()?;
                let actual = self.invoke(None, exec.name, &values)?;
                self.assert_trap(actual, message)?;
            }
            Standard(d) => self.run_standard_directive(d)?,
        }

        Ok(())
    }
    fn run_standard_directive(&mut self, directive: wast::WastDirective) -> Result<()> {
        use wast::WastDirective::*;

        match directive {
            Module(mut module) => {
                let binary = module.encode()?;
                self.module(module.id.map(|s| s.name()), &binary)?;
            }
            Register {
                span: _,
                name,
                module,
            } => {
                self.register(module.map(|s| s.name()), name)?;
            }
            Invoke(i) => {
                self.perform_invoke(i)?;
            }
            AssertReturn {
                span: _,
                exec,
                results,
            } => {
                let result = self.perform_execute(exec)?;
                self.assert_return(result, &results)?;
            }
            AssertTrap {
                span: _,
                exec,
                message,
            } => {
                let result = self.perform_execute(exec)?;
                self.assert_trap(result, message)?;
            }
            AssertExhaustion {
                span: _,
                call,
                message,
            } => {
                let result = self.perform_invoke(call)?;
                self.assert_trap(result, message)?;
            }
            AssertInvalid {
                span: _,
                mut module,
                message,
            } => {
                let bytes = module.encode()?;
                let err = match self.module(None, &bytes) {
                    Ok(()) => bail!("expected module to fail to build"),
                    Err(e) => e,
                };
                let error_message = format!("{:?}", err);
                if !is_matching_assert_invalid_error_message(&message, &error_message) {
                    bail!(
                        "assert_invalid: expected \"{}\", got \"{}\"",
                        message,
                        error_message
                    )
                }
            }
            AssertMalformed {
                module,
                span: _,
                message: _,
            } => {
                let mut module = match module {
                    wast::QuoteModule::Module(m) => m,
                    // This is a `*.wat` parser test which we're not
                    // interested in.
                    wast::QuoteModule::Quote(_) => return Ok(()),
                };
                let bytes = module.encode()?;
                if let Ok(_) = self.module(None, &bytes) {
                    bail!("expected malformed module to fail to instantiate");
                }
            }
            AssertUnlinkable {
                span: _,
                mut module,
                message,
            } => {
                let bytes = module.encode()?;
                let err = match self.module(None, &bytes) {
                    Ok(()) => bail!("expected module to fail to link"),
                    Err(e) => e,
                };
                let error_message = format!("{:?}", err);
                if !error_message.contains(&message) {
                    bail!(
                        "assert_unlinkable: expected {}, got {}",
                        message,
                        error_message
                    )
                }
            }
        }

        Ok(())
    }

    /// Run a wast script from a file.
    pub fn run_file(&mut self, path: &Path) -> Result<()> {
        let bytes =
            std::fs::read(path).with_context(|| format!("failed to read `{}`", path.display()))?;
        self.run_buffer(path.to_str().unwrap(), &bytes)
    }
}

fn is_matching_assert_invalid_error_message(expected: &str, actual: &str) -> bool {
    actual.contains(expected)
        // Waiting on https://github.com/WebAssembly/bulk-memory-operations/pull/137
        // to propagate to WebAssembly/testsuite.
        || (expected.contains("unknown table") && actual.contains("unknown elem"))
        // `elem.wast` and `proposals/bulk-memory-operations/elem.wast` disagree
        // on the expected error message for the same error.
        || (expected.contains("out of bounds") && actual.contains("does not fit"))
}

fn extract_lane_as_i8(bytes: u128, lane: usize) -> i8 {
    (bytes >> (lane * 8)) as i8
}

fn extract_lane_as_i16(bytes: u128, lane: usize) -> i16 {
    (bytes >> (lane * 16)) as i16
}

fn extract_lane_as_i32(bytes: u128, lane: usize) -> i32 {
    (bytes >> (lane * 32)) as i32
}

fn extract_lane_as_i64(bytes: u128, lane: usize) -> i64 {
    (bytes >> (lane * 64)) as i64
}

fn is_canonical_f32_nan(bits: u32) -> bool {
    (bits & 0x7fff_ffff) == 0x7fc0_0000
}

fn is_canonical_f64_nan(bits: u64) -> bool {
    (bits & 0x7fff_ffff_ffff_ffff) == 0x7ff8_0000_0000_0000
}

fn is_arithmetic_f32_nan(bits: u32) -> bool {
    const AF32_NAN: u32 = 0x0040_0000;
    (bits & AF32_NAN) == AF32_NAN
}

fn is_arithmetic_f64_nan(bits: u64) -> bool {
    const AF64_NAN: u64 = 0x0008_0000_0000_0000;
    (bits & AF64_NAN) == AF64_NAN
}

trait MatchesVal: std::fmt::Debug {
    fn matches(&self, val: &Val) -> Result<bool>;
}

impl MatchesVal for wast::AssertExpression<'_> {
    fn matches(&self, val: &Val) -> Result<bool> {
        Ok(match (val, self) {
            (Val::I32(a), wast::AssertExpression::I32(b)) => a == b,
            (Val::I64(a), wast::AssertExpression::I64(b)) => a == b,
            // Note that these float comparisons are comparing bits, not float
            // values, so we're testing for bit-for-bit equivalence
            (Val::F32(a), wast::AssertExpression::F32(b)) => f32_matches(*a, b),
            (Val::F64(a), wast::AssertExpression::F64(b)) => f64_matches(*a, b),
            (Val::V128(a), wast::AssertExpression::V128(b)) => v128_matches(*a, b),
            _ => bail!("don't know how to compare {:?} and {:?} yet", self, val,),
        })
    }
}

fn f32_matches(actual: u32, expected: &wast::NanPattern<wast::Float32>) -> bool {
    match expected {
        wast::NanPattern::CanonicalNan => is_canonical_f32_nan(actual),
        wast::NanPattern::ArithmeticNan => is_arithmetic_f32_nan(actual),
        wast::NanPattern::Value(expected_value) => actual == expected_value.bits,
    }
}

fn f64_matches(actual: u64, expected: &wast::NanPattern<wast::Float64>) -> bool {
    match expected {
        wast::NanPattern::CanonicalNan => is_canonical_f64_nan(actual),
        wast::NanPattern::ArithmeticNan => is_arithmetic_f64_nan(actual),
        wast::NanPattern::Value(expected_value) => actual == expected_value.bits,
    }
}

fn v128_matches(actual: u128, expected: &wast::V128Pattern) -> bool {
    match expected {
        wast::V128Pattern::I8x16(b) => b
            .iter()
            .enumerate()
            .all(|(i, b)| *b == extract_lane_as_i8(actual, i)),
        wast::V128Pattern::I16x8(b) => b
            .iter()
            .enumerate()
            .all(|(i, b)| *b == extract_lane_as_i16(actual, i)),
        wast::V128Pattern::I32x4(b) => b
            .iter()
            .enumerate()
            .all(|(i, b)| *b == extract_lane_as_i32(actual, i)),
        wast::V128Pattern::I64x2(b) => b
            .iter()
            .enumerate()
            .all(|(i, b)| *b == extract_lane_as_i64(actual, i)),
        wast::V128Pattern::F32x4(b) => b.iter().enumerate().all(|(i, b)| {
            let a = extract_lane_as_i32(actual, i) as u32;
            f32_matches(a, b)
        }),
        wast::V128Pattern::F64x2(b) => b.iter().enumerate().all(|(i, b)| {
            let a = extract_lane_as_i64(actual, i) as u64;
            f64_matches(a, b)
        }),
    }
}

struct Wast<'a> {
    directives: Vec<WastDirective<'a>>,
}

impl<'a> Parse<'a> for Wast<'a> {
    fn parse(parser: Parser<'a>) -> wast::parser::Result<Self> {
        let _r = parser.register_annotation("interface");

        // All `*.wast` files that use interface types will currently be
        // required to start with `(@interface)` at the top, so see if that's
        // there. If not we parse as a normal wast file.
        if !parser.peek2::<annotation::interface>() {
            let wast::Wast { directives } = parser.parse()?;
            return Ok(Wast {
                directives: directives
                    .into_iter()
                    .map(WastDirective::Standard)
                    .collect(),
            });
        }

        // Consume `(@interface)` ...
        parser.parens(|p| p.parse::<annotation::interface>())?;

        // ... then parse a bunch of our custom directives
        let mut directives = Vec::new();
        while !parser.is_empty() {
            directives.push(parser.parens(|p| p.parse())?);
        }
        Ok(Wast { directives })
    }
}

/// Extension of the standard wast directives with a few directives that can
/// have wasm interface types in them as well.
enum WastDirective<'a> {
    WitModule(wit_text::Module<'a>),
    WitAssertReturn {
        span: wast::Span,
        exec: Invoke<'a>,
        results: Vec<AssertExpression<'a>>,
    },
    WitAssertTrap {
        span: wast::Span,
        exec: Invoke<'a>,
        message: &'a str,
    },
    Standard(wast::WastDirective<'a>),
}

impl WastDirective<'_> {
    fn span(&self) -> wast::Span {
        match self {
            WastDirective::WitModule(m) => m.core.span,
            WastDirective::WitAssertReturn { span, .. } => *span,
            WastDirective::WitAssertTrap { span, .. } => *span,
            WastDirective::Standard(e) => e.span(),
        }
    }
}

impl<'a> Parse<'a> for WastDirective<'a> {
    fn parse(parser: Parser<'a>) -> wast::parser::Result<Self> {
        if parser.peek::<wast::kw::module>() {
            Ok(WastDirective::WitModule(parser.parse()?))
        } else if parser.peek::<wast::kw::assert_return>() {
            let span = parser.parse::<wast::kw::assert_return>()?.0;
            let exec = parser.parens(|p| p.parse())?;
            let mut results = Vec::new();
            while !parser.is_empty() {
                results.push(parser.parens(|p| p.parse())?);
            }
            Ok(WastDirective::WitAssertReturn {
                span,
                exec,
                results,
            })
        } else if parser.peek::<wast::kw::assert_trap>() {
            let span = parser.parse::<wast::kw::assert_trap>()?.0;
            Ok(WastDirective::WitAssertTrap {
                span,
                exec: parser.parens(|p| p.parse())?,
                message: parser.parse()?,
            })
        } else {
            Err(parser.error("failed to parse interface types directive"))
        }
    }
}

struct Invoke<'a> {
    pub span: wast::Span,
    pub name: &'a str,
    pub args: Vec<AssertExpression<'a>>,
}

impl<'a> Parse<'a> for Invoke<'a> {
    fn parse(parser: Parser<'a>) -> wast::parser::Result<Self> {
        let span = parser.parse::<wast::kw::invoke>()?.0;
        let name = parser.parse()?;
        let mut args = Vec::new();
        while !parser.is_empty() {
            args.push(parser.parens(|p| p.parse())?);
        }
        Ok(Invoke { span, name, args })
    }
}

#[derive(Debug)]
enum AssertExpression<'a> {
    S8(i8),
    S16(i16),
    S32(i32),
    S64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    String(&'a str),
    Other(wast::AssertExpression<'a>),
}

impl AssertExpression<'_> {
    fn to_val(&self) -> Result<Val> {
        Ok(match self {
            AssertExpression::S8(v) => Val::S8(*v),
            AssertExpression::S16(v) => Val::S16(*v),
            AssertExpression::S32(v) => Val::S32(*v),
            AssertExpression::S64(v) => Val::S64(*v),
            AssertExpression::U8(v) => Val::U8(*v),
            AssertExpression::U16(v) => Val::U16(*v),
            AssertExpression::U32(v) => Val::U32(*v),
            AssertExpression::U64(v) => Val::U64(*v),
            AssertExpression::String(s) => Val::String(s.to_string()),
            AssertExpression::Other(e) => match e {
                wast::AssertExpression::I32(v) => Val::I32(*v),
                wast::AssertExpression::I64(v) => Val::I64(*v),
                other => bail!("unsupported constant in wast {:?}", other),
            },
        })
    }
}

impl MatchesVal for AssertExpression<'_> {
    fn matches(&self, val: &Val) -> Result<bool> {
        Ok(match (val, self) {
            (Val::S8(a), AssertExpression::S8(b)) => a == b,
            (Val::S16(a), AssertExpression::S16(b)) => a == b,
            (Val::S32(a), AssertExpression::S32(b)) => a == b,
            (Val::S64(a), AssertExpression::S64(b)) => a == b,
            (Val::U8(a), AssertExpression::U8(b)) => a == b,
            (Val::U16(a), AssertExpression::U16(b)) => a == b,
            (Val::U32(a), AssertExpression::U32(b)) => a == b,
            (Val::U64(a), AssertExpression::U64(b)) => a == b,
            (Val::String(a), AssertExpression::String(b)) => a == b,
            (_, AssertExpression::Other(m)) => return m.matches(val),
            _ => bail!("don't know how to compare {:?} and {:?} yet", self, val,),
        })
    }
}

impl<'a> Parse<'a> for AssertExpression<'a> {
    fn parse(parser: Parser<'a>) -> wast::parser::Result<Self> {
        macro_rules! parse {
            ($($variant:ident = $kw:ident)*) => ($(
                wast::custom_keyword!($kw = concat!(stringify!($kw), ".const"));
                if parser.peek::<$kw>() {
                    parser.parse::<$kw>()?;
                    return Ok(AssertExpression::$variant(parser.parse()?));
                }
            )*)
        }

        parse! {
            S8 = s8
            S16 = s16
            S32 = s32
            S64 = s64
            U8 = u8
            U16 = u16
            U32 = u32
            U64 = u64
            String = string
        }

        Ok(AssertExpression::Other(parser.parse()?))
    }
}

mod annotation {
    wast::annotation!(interface);
}
