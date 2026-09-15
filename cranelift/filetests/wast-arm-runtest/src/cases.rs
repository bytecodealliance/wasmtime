use json_from_wast::{Const, CoreConst, FloatConst};
use wasmparser::Parser;

#[derive(Debug)]
pub struct Case {
    pub module: Vec<u8>,
    pub export: String,
    pub args: Vec<CaseValue>,
    pub expected: CaseValue,
}

impl Case {
    pub fn try_build(
        module: &[u8],
        field: &str,
        args: &[Const],
        expected: &[Const],
    ) -> Result<Option<Case>, String> {
        let args = args
            .iter()
            .map(CaseValue::from_const)
            .collect::<Result<Vec<_>, _>>()?;

        if expected.is_empty() {
            return Err("expected at least one result, got 0".to_string());
        }
        if expected.len() != 1 {
            return Err(format!(
                "unsupported result arity: expected 1 result, got {}",
                expected.len()
            ));
        }
        let expected = CaseValue::from_const(&expected[0])?;

        if !module_exports_func(module, field) {
            return Err(format!("no func export named {field:?}"));
        }

        Ok(Some(Case {
            module: module.to_vec(),
            export: field.to_string(),
            args,
            expected,
        }))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CaseValue {
    I32(i32),
    I64(i64),
    F32(FloatConst<f32>),
    F64(FloatConst<f64>),
}

impl PartialEq for CaseValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::I32(a), Self::I32(b)) => a == b,
            (Self::I64(a), Self::I64(b)) => a == b,
            (Self::F32(a), Self::F32(b)) => a.to_bits() == b.to_bits(),
            (Self::F64(a), Self::F64(b)) => a.to_bits() == b.to_bits(),
            _ => false,
        }
    }
}

impl CaseValue {
    fn from_const(value: &Const) -> Result<Self, String> {
        match value {
            Const::Core(CoreConst::I32 { value }) => Ok(Self::I32(value.0)),
            Const::Core(CoreConst::I64 { value }) => Ok(Self::I64(value.0)),
            Const::Core(CoreConst::F32 { value }) => Ok(Self::F32(*value)),
            Const::Core(CoreConst::F64 { value }) => Ok(Self::F64(*value)),
            other => Err(format!("unsupported value: {other:?}")),
        }
    }

    pub(crate) fn to_wast_literal(self) -> String {
        match self {
            Self::F32(value) => match value {
                FloatConst::ArithmeticNan => "nan:arithmetic".to_string(),
                FloatConst::CanonicalNan => "nan:canonical".to_string(),
                FloatConst::Value(bits) => format_f32_literal(bits.to_bits()),
            },
            Self::F64(value) => match value {
                FloatConst::ArithmeticNan => "nan:arithmetic".to_string(),
                FloatConst::CanonicalNan => "nan:canonical".to_string(),
                FloatConst::Value(bits) => format_f64_literal(bits.to_bits()),
            },
            Self::I32(value) => value.to_string(),
            Self::I64(value) => value.to_string(),
        }
    }
}

fn format_float_literal(sign: &str, exponent: i32, fraction_bits: &str) -> String {
    let mut hex_digits = String::new();
    for chunk in fraction_bits.chars().collect::<Vec<_>>().chunks(4) {
        let nibble = chunk.iter().collect::<String>();
        let value = u8::from_str_radix(&nibble, 2).unwrap_or_default();
        let digit = match value {
            0..=9 => char::from(b'0' + value),
            10 => 'a',
            11 => 'b',
            12 => 'c',
            13 => 'd',
            14 => 'e',
            15 => 'f',
            _ => unreachable!(),
        };
        hex_digits.push(digit);
    }

    let trimmed = hex_digits.trim_start_matches('0');
    if trimmed.is_empty() {
        format!("{sign}0x1p{exponent:+}")
    } else {
        format!("{sign}0x1.{trimmed}p{exponent:+}")
    }
}

#[derive(Debug, Clone, Copy)]
struct FloatLiteralFormatConfig {
    sign_mask: u64,
    exponent_mask: u64,
    fraction_mask: u64,
    exponent_shift: u32,
    exponent_bias: i32,
    fraction_width: usize,
    quiet_nan_mask: u64,
    exponent_all_ones: u64,
}

const F32_FLOAT_LITERAL_CONFIG: FloatLiteralFormatConfig = FloatLiteralFormatConfig {
    sign_mask: 0x8000_0000,
    exponent_mask: 0x7f80_0000,
    fraction_mask: 0x007f_ffff,
    exponent_shift: 23,
    exponent_bias: 127,
    fraction_width: 23,
    quiet_nan_mask: 0x0040_0000,
    exponent_all_ones: 0x7f80_0000,
};

const F64_FLOAT_LITERAL_CONFIG: FloatLiteralFormatConfig = FloatLiteralFormatConfig {
    sign_mask: 0x8000_0000_0000_0000,
    exponent_mask: 0x7ff0_0000_0000_0000,
    fraction_mask: 0x000f_ffff_ffff_ffff,
    exponent_shift: 52,
    exponent_bias: 1023,
    fraction_width: 52,
    quiet_nan_mask: 0x0008_0000_0000_0000,
    exponent_all_ones: 0x7ff0_0000_0000_0000,
};

fn format_float_literal_from_bits(bits: u64, config: FloatLiteralFormatConfig) -> String {
    if bits == 0 {
        return "0x0p+0".to_string();
    }
    if bits == config.sign_mask {
        return "-0x0p+0".to_string();
    }

    let sign = if bits & config.sign_mask != 0 {
        "-"
    } else {
        ""
    };
    let signless = bits & !config.sign_mask;

    if signless == config.exponent_all_ones {
        return format!("{sign}inf");
    }
    if signless & config.exponent_all_ones == config.exponent_all_ones {
        return if signless & config.quiet_nan_mask == 0 {
            format!("{sign}nan:canonical")
        } else {
            format!("{sign}nan:arithmetic")
        };
    }

    let exponent =
        ((signless & config.exponent_mask) >> config.exponent_shift) as i32 - config.exponent_bias;
    let fraction_bits = format!(
        "{:0width$b}",
        signless & config.fraction_mask,
        width = config.fraction_width
    );
    format_float_literal(sign, exponent, &fraction_bits)
}

fn format_f32_literal(bits: u32) -> String {
    format_float_literal_from_bits(bits as u64, F32_FLOAT_LITERAL_CONFIG)
}

fn format_f64_literal(bits: u64) -> String {
    format_float_literal_from_bits(bits, F64_FLOAT_LITERAL_CONFIG)
}

fn module_exports_func(module: &[u8], name: &str) -> bool {
    let parser = Parser::new(0);
    for payload in parser.parse_all(module).flatten() {
        if let wasmparser::Payload::ExportSection(reader) = payload {
            for export in reader.into_iter().flatten() {
                if export.kind == wasmparser::ExternalKind::Func && export.name == name {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use json_from_wast::{Const, CoreConst, IntString};

    #[test]
    fn accepts_i64_args_and_results() {
        let arg = Const::Core(CoreConst::I64 {
            value: IntString(7),
        });
        let expected = Const::Core(CoreConst::I64 {
            value: IntString(8),
        });

        assert_eq!(CaseValue::from_const(&arg).unwrap(), CaseValue::I64(7));
        assert_eq!(CaseValue::from_const(&expected).unwrap(), CaseValue::I64(8));
    }

    #[test]
    fn accepts_float_args_and_results() {
        let arg = Const::Core(CoreConst::F32 {
            value: FloatConst::Value(1.5_f32),
        });
        let expected = Const::Core(CoreConst::F64 {
            value: FloatConst::Value(2.5_f64),
        });

        assert_eq!(
            CaseValue::from_const(&arg).unwrap(),
            CaseValue::F32(FloatConst::Value(1.5_f32))
        );
        assert_eq!(
            CaseValue::from_const(&expected).unwrap(),
            CaseValue::F64(FloatConst::Value(2.5_f64))
        );
    }

    #[test]
    fn formats_special_float_literals_consistently() {
        assert_eq!(format_f32_literal(0x8000_0000), "-0x0p+0");
        assert_eq!(format_f32_literal(0x7fc0_0000), "nan:arithmetic");
        assert_eq!(format_f64_literal(0x8000_0000_0000_0000), "-0x0p+0");
        assert_eq!(format_f64_literal(0x7ff8_0000_0000_0000), "nan:arithmetic");
    }
}
