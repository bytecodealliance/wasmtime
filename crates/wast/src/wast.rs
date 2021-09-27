use crate::spectest::link_spectest;
use anyhow::{anyhow, bail, Context as _, Result};
use std::fmt::{Display, LowerHex};
use std::path::Path;
use std::str;
use wasmtime::*;
use wast::Wat;
use wast::{
    parser::{self, ParseBuffer},
    HeapType,
};

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
        RefNull(HeapType::Extern) => Val::ExternRef(None),
        RefNull(HeapType::Func) => Val::FuncRef(None),
        RefExtern(x) => Val::ExternRef(Some(ExternRef::new(*x))),
        other => bail!("couldn't convert {:?} to a runtime value", other),
    })
}

/// The wast test script language allows modules to be defined and actions
/// to be performed on them.
pub struct WastContext<T> {
    /// Wast files have a concept of a "current" module, which is the most
    /// recently defined.
    current: Option<Instance>,
    linker: Linker<T>,
    store: Store<T>,
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

impl<T> WastContext<T> {
    /// Construct a new instance of `WastContext`.
    pub fn new(store: Store<T>) -> Self {
        // Spec tests will redefine the same module/name sometimes, so we need
        // to allow shadowing in the linker which picks the most recent
        // definition as what to link when linking.
        let mut linker = Linker::new(store.engine());
        linker.allow_shadowing(true);
        Self {
            current: None,
            linker,
            store,
        }
    }

    fn get_export(&mut self, module: Option<&str>, name: &str) -> Result<Extern> {
        match module {
            Some(module) => self
                .linker
                .get(&mut self.store, module, Some(name))
                .ok_or_else(|| anyhow!("no item named `{}::{}` found", module, name)),
            None => self
                .current
                .as_ref()
                .ok_or_else(|| anyhow!("no previous instance found"))?
                .get_export(&mut self.store, name)
                .ok_or_else(|| anyhow!("no item named `{}` found", name)),
        }
    }

    fn instantiate(&mut self, module: &[u8]) -> Result<Outcome<Instance>> {
        let module = Module::new(self.store.engine(), module)?;
        let instance = match self.linker.instantiate(&mut self.store, &module) {
            Ok(i) => i,
            Err(e) => return e.downcast::<Trap>().map(Outcome::Trap),
        };
        Ok(Outcome::Ok(instance))
    }

    /// Register "spectest" which is used by the spec testsuite.
    pub fn register_spectest(&mut self) -> Result<()> {
        link_spectest(&mut self.linker, &mut self.store)?;
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
            .map(|v| runtime_value(v))
            .collect::<Result<Vec<_>>>()?;
        self.invoke(exec.module.map(|i| i.name()), exec.name, &values)
    }

    /// Define a module and register it.
    fn module(&mut self, instance_name: Option<&str>, module: &[u8]) -> Result<()> {
        let instance = match self.instantiate(module)? {
            Outcome::Ok(i) => i,
            Outcome::Trap(e) => return Err(e).context("instantiation failed"),
        };
        if let Some(name) = instance_name {
            self.linker.instance(&mut self.store, name, instance)?;
        }
        self.current = Some(instance);
        Ok(())
    }

    /// Register an instance to make it available for performing actions.
    fn register(&mut self, name: Option<&str>, as_name: &str) -> Result<()> {
        match name {
            Some(name) => self.linker.alias_module(name, as_name),
            None => {
                let current = *self
                    .current
                    .as_ref()
                    .ok_or(anyhow!("no previous instance"))?;
                self.linker.instance(&mut self.store, as_name, current)?;
                Ok(())
            }
        }
    }

    /// Invoke an exported function from an instance.
    fn invoke(
        &mut self,
        instance_name: Option<&str>,
        field: &str,
        args: &[Val],
    ) -> Result<Outcome> {
        let func = self
            .get_export(instance_name, field)?
            .into_func()
            .ok_or_else(|| anyhow!("no function named `{}`", field))?;

        let mut results = vec![Val::null(); func.ty(&self.store).results().len()];
        Ok(match func.call(&mut self.store, args, &mut results) {
            Ok(()) => Outcome::Ok(results.into()),
            Err(e) => Outcome::Trap(e.downcast()?),
        })
    }

    /// Get the value of an exported global from an instance.
    fn get(&mut self, instance_name: Option<&str>, field: &str) -> Result<Outcome> {
        let global = self
            .get_export(instance_name, field)?
            .into_global()
            .ok_or_else(|| anyhow!("no global named `{}`", field))?;
        Ok(Outcome::Ok(vec![global.get(&mut self.store)]))
    }

    fn assert_return(&self, result: Outcome, results: &[wast::AssertExpression]) -> Result<()> {
        let values = result.into_result()?;
        for (i, (v, e)) in values.iter().zip(results).enumerate() {
            match_val(v, e).with_context(|| format!("result {} didn't match", i))?;
        }
        Ok(())
    }

    fn assert_trap(&self, result: Outcome, expected: &str) -> Result<()> {
        let trap = match result {
            Outcome::Ok(values) => bail!("expected trap, got {:?}", values),
            Outcome::Trap(t) => t,
        };
        let actual = trap.to_string();
        if actual.contains(expected)
            // `bulk-memory-operations/bulk.wast` checks for a message that
            // specifies which element is uninitialized, but our traps don't
            // shepherd that information out.
            || (expected.contains("uninitialized element 2") && actual.contains("uninitialized element"))
        {
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
        let ast = wast::parser::parse::<wast::Wast>(&buf).map_err(adjust_wast)?;

        for directive in ast.directives {
            let sp = directive.span();
            self.run_directive(directive, &adjust_wast)
                .with_context(|| {
                    let (line, col) = sp.linecol_in(wast);
                    format!("failed directive on {}:{}:{}", filename, line + 1, col)
                })?;
        }
        Ok(())
    }

    fn run_directive(
        &mut self,
        directive: wast::WastDirective,
        adjust: impl Fn(wast::Error) -> wast::Error,
    ) -> Result<()> {
        use wast::WastDirective::*;

        match directive {
            Module(mut module) => {
                let binary = module.encode().map_err(adjust)?;
                self.module(module.id.map(|s| s.name()), &binary)?;
            }
            QuoteModule { span: _, source } => {
                let mut module = String::new();
                for src in source {
                    module.push_str(str::from_utf8(src)?);
                    module.push_str(" ");
                }
                let buf = ParseBuffer::new(&module)?;
                let mut wat = parser::parse::<Wat>(&buf).map_err(|mut e| {
                    e.set_text(&module);
                    e
                })?;
                let binary = wat.module.encode()?;
                self.module(wat.module.id.map(|s| s.name()), &binary)?;
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
                module,
                message,
            } => {
                let mut module = match module {
                    wast::QuoteModule::Module(m) => m,
                    // This is a `*.wat` parser test which we're not
                    // interested in.
                    wast::QuoteModule::Quote(_) => return Ok(()),
                };
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
                let bytes = module.encode().map_err(adjust)?;
                if let Ok(_) = self.module(None, &bytes) {
                    bail!("expected malformed module to fail to instantiate");
                }
            }
            AssertUnlinkable {
                span: _,
                mut module,
                message,
            } => {
                let bytes = module.encode().map_err(adjust)?;
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
            AssertUncaughtException { .. } => bail!("unimplemented assert_uncaught_exception"),
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
        // `elem.wast` and `proposals/bulk-memory-operations/elem.wast` disagree
        // on the expected error message for the same error.
        || (expected.contains("out of bounds") && actual.contains("does not fit"))
        // slight difference in error messages
        || (expected.contains("unknown elem segment") && actual.contains("unknown element segment"))
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

fn match_val(actual: &Val, expected: &wast::AssertExpression) -> Result<()> {
    match (actual, expected) {
        (Val::I32(a), wast::AssertExpression::I32(b)) => match_int(a, b),
        (Val::I64(a), wast::AssertExpression::I64(b)) => match_int(a, b),
        // Note that these float comparisons are comparing bits, not float
        // values, so we're testing for bit-for-bit equivalence
        (Val::F32(a), wast::AssertExpression::F32(b)) => match_f32(*a, b),
        (Val::F64(a), wast::AssertExpression::F64(b)) => match_f64(*a, b),
        (Val::V128(a), wast::AssertExpression::V128(b)) => match_v128(*a, b),
        (Val::ExternRef(x), wast::AssertExpression::RefNull(Some(HeapType::Extern))) => {
            if let Some(x) = x {
                let x = x
                    .data()
                    .downcast_ref::<u32>()
                    .expect("only u32 externrefs created in wast test suites");
                bail!("expected null externref, found {}", x);
            } else {
                Ok(())
            }
        }
        (Val::ExternRef(x), wast::AssertExpression::RefExtern(y)) => {
            if let Some(x) = x {
                let x = x
                    .data()
                    .downcast_ref::<u32>()
                    .expect("only u32 externrefs created in wast test suites");
                if x == y {
                    Ok(())
                } else {
                    bail!("expected {} found {}", y, x);
                }
            } else {
                bail!("expected non-null externref, found null")
            }
        }
        (Val::FuncRef(x), wast::AssertExpression::RefNull(_)) => {
            if x.is_none() {
                Ok(())
            } else {
                bail!("expected null funcref, found non-null")
            }
        }
        _ => bail!(
            "don't know how to compare {:?} and {:?} yet",
            actual,
            expected
        ),
    }
}

fn match_int<T>(actual: &T, expected: &T) -> Result<()>
where
    T: Eq + Display + LowerHex,
{
    if actual == expected {
        Ok(())
    } else {
        bail!(
            "expected {:18} / {0:#018x}\n\
             actual   {:18} / {1:#018x}",
            expected,
            actual
        )
    }
}

fn match_f32(actual: u32, expected: &wast::NanPattern<wast::Float32>) -> Result<()> {
    match expected {
        // Check if an f32 (as u32 bits to avoid possible quieting when moving values in registers, e.g.
        // https://developer.arm.com/documentation/ddi0344/i/neon-and-vfp-programmers-model/modes-of-operation/default-nan-mode?lang=en)
        // is a canonical NaN:
        //  - the sign bit is unspecified,
        //  - the 8-bit exponent is set to all 1s
        //  - the MSB of the payload is set to 1 (a quieted NaN) and all others to 0.
        // See https://webassembly.github.io/spec/core/syntax/values.html#floating-point.
        wast::NanPattern::CanonicalNan => {
            let canon_nan = 0x7fc0_0000;
            if (actual & 0x7fff_ffff) == canon_nan {
                Ok(())
            } else {
                bail!(
                    "expected {:10} / {:#010x}\n\
                     actual   {:10} / {:#010x}",
                    "canon-nan",
                    canon_nan,
                    f32::from_bits(actual),
                    actual,
                )
            }
        }

        // Check if an f32 (as u32, see comments above) is an arithmetic NaN.
        // This is the same as a canonical NaN including that the payload MSB is
        // set to 1, but one or more of the remaining payload bits MAY BE set to
        // 1 (a canonical NaN specifies all 0s). See
        // https://webassembly.github.io/spec/core/syntax/values.html#floating-point.
        wast::NanPattern::ArithmeticNan => {
            const AF32_NAN: u32 = 0x7f80_0000;
            let is_nan = actual & AF32_NAN == AF32_NAN;
            const AF32_PAYLOAD_MSB: u32 = 0x0040_0000;
            let is_msb_set = actual & AF32_PAYLOAD_MSB == AF32_PAYLOAD_MSB;
            if is_nan && is_msb_set {
                Ok(())
            } else {
                bail!(
                    "expected {:>10} / {:>10}\n\
                     actual   {:10} / {:#010x}",
                    "arith-nan",
                    "0x7fc*****",
                    f32::from_bits(actual),
                    actual,
                )
            }
        }
        wast::NanPattern::Value(expected_value) => {
            if actual == expected_value.bits {
                Ok(())
            } else {
                bail!(
                    "expected {:10} / {:#010x}\n\
                     actual   {:10} / {:#010x}",
                    f32::from_bits(expected_value.bits),
                    expected_value.bits,
                    f32::from_bits(actual),
                    actual,
                )
            }
        }
    }
}

fn match_f64(actual: u64, expected: &wast::NanPattern<wast::Float64>) -> Result<()> {
    match expected {
        // Check if an f64 (as u64 bits to avoid possible quieting when moving values in registers, e.g.
        // https://developer.arm.com/documentation/ddi0344/i/neon-and-vfp-programmers-model/modes-of-operation/default-nan-mode?lang=en)
        // is a canonical NaN:
        //  - the sign bit is unspecified,
        //  - the 11-bit exponent is set to all 1s
        //  - the MSB of the payload is set to 1 (a quieted NaN) and all others to 0.
        // See https://webassembly.github.io/spec/core/syntax/values.html#floating-point.
        wast::NanPattern::CanonicalNan => {
            let canon_nan = 0x7ff8_0000_0000_0000;
            if (actual & 0x7fff_ffff_ffff_ffff) == canon_nan {
                Ok(())
            } else {
                bail!(
                    "expected {:18} / {:#018x}\n\
                     actual   {:18} / {:#018x}",
                    "canon-nan",
                    canon_nan,
                    f64::from_bits(actual),
                    actual,
                )
            }
        }

        // Check if an f64 (as u64, see comments above) is an arithmetic NaN. This is the same as a
        // canonical NaN including that the payload MSB is set to 1, but one or more of the remaining
        // payload bits MAY BE set to 1 (a canonical NaN specifies all 0s). See
        // https://webassembly.github.io/spec/core/syntax/values.html#floating-point.
        wast::NanPattern::ArithmeticNan => {
            const AF64_NAN: u64 = 0x7ff0_0000_0000_0000;
            let is_nan = actual & AF64_NAN == AF64_NAN;
            const AF64_PAYLOAD_MSB: u64 = 0x0008_0000_0000_0000;
            let is_msb_set = actual & AF64_PAYLOAD_MSB == AF64_PAYLOAD_MSB;
            if is_nan && is_msb_set {
                Ok(())
            } else {
                bail!(
                    "expected {:>18} / {:>18}\n\
                     actual   {:18} / {:#018x}",
                    "arith-nan",
                    "0x7ff8************",
                    f64::from_bits(actual),
                    actual,
                )
            }
        }
        wast::NanPattern::Value(expected_value) => {
            if actual == expected_value.bits {
                Ok(())
            } else {
                bail!(
                    "expected {:18} / {:#018x}\n\
                     actual   {:18} / {:#018x}",
                    f64::from_bits(expected_value.bits),
                    expected_value.bits,
                    f64::from_bits(actual),
                    actual,
                )
            }
        }
    }
}

fn match_v128(actual: u128, expected: &wast::V128Pattern) -> Result<()> {
    match expected {
        wast::V128Pattern::I8x16(expected) => {
            let actual = [
                extract_lane_as_i8(actual, 0),
                extract_lane_as_i8(actual, 1),
                extract_lane_as_i8(actual, 2),
                extract_lane_as_i8(actual, 3),
                extract_lane_as_i8(actual, 4),
                extract_lane_as_i8(actual, 5),
                extract_lane_as_i8(actual, 6),
                extract_lane_as_i8(actual, 7),
                extract_lane_as_i8(actual, 8),
                extract_lane_as_i8(actual, 9),
                extract_lane_as_i8(actual, 10),
                extract_lane_as_i8(actual, 11),
                extract_lane_as_i8(actual, 12),
                extract_lane_as_i8(actual, 13),
                extract_lane_as_i8(actual, 14),
                extract_lane_as_i8(actual, 15),
            ];
            if actual == *expected {
                return Ok(());
            }
            bail!(
                "expected {:4?}\n\
                 actual   {:4?}\n\
                 \n\
                 expected (hex) {0:02x?}\n\
                 actual (hex)   {1:02x?}",
                expected,
                actual,
            )
        }
        wast::V128Pattern::I16x8(expected) => {
            let actual = [
                extract_lane_as_i16(actual, 0),
                extract_lane_as_i16(actual, 1),
                extract_lane_as_i16(actual, 2),
                extract_lane_as_i16(actual, 3),
                extract_lane_as_i16(actual, 4),
                extract_lane_as_i16(actual, 5),
                extract_lane_as_i16(actual, 6),
                extract_lane_as_i16(actual, 7),
            ];
            if actual == *expected {
                return Ok(());
            }
            bail!(
                "expected {:6?}\n\
                 actual   {:6?}\n\
                 \n\
                 expected (hex) {0:04x?}\n\
                 actual (hex)   {1:04x?}",
                expected,
                actual,
            )
        }
        wast::V128Pattern::I32x4(expected) => {
            let actual = [
                extract_lane_as_i32(actual, 0),
                extract_lane_as_i32(actual, 1),
                extract_lane_as_i32(actual, 2),
                extract_lane_as_i32(actual, 3),
            ];
            if actual == *expected {
                return Ok(());
            }
            bail!(
                "expected {:11?}\n\
                 actual   {:11?}\n\
                 \n\
                 expected (hex) {0:08x?}\n\
                 actual (hex)   {1:08x?}",
                expected,
                actual,
            )
        }
        wast::V128Pattern::I64x2(expected) => {
            let actual = [
                extract_lane_as_i64(actual, 0),
                extract_lane_as_i64(actual, 1),
            ];
            if actual == *expected {
                return Ok(());
            }
            bail!(
                "expected {:20?}\n\
                 actual   {:20?}\n\
                 \n\
                 expected (hex) {0:016x?}\n\
                 actual (hex)   {1:016x?}",
                expected,
                actual,
            )
        }
        wast::V128Pattern::F32x4(expected) => {
            for (i, expected) in expected.iter().enumerate() {
                let a = extract_lane_as_i32(actual, i) as u32;
                match_f32(a, expected).with_context(|| format!("difference in lane {}", i))?;
            }
            Ok(())
        }
        wast::V128Pattern::F64x2(expected) => {
            for (i, expected) in expected.iter().enumerate() {
                let a = extract_lane_as_i64(actual, i) as u64;
                match_f64(a, expected).with_context(|| format!("difference in lane {}", i))?;
            }
            Ok(())
        }
    }
}
