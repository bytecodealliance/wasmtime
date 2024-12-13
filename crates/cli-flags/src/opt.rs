//! Support for parsing Wasmtime's `-O`, `-W`, etc "option groups"
//!
//! This builds up a clap-derive-like system where there's ideally a single
//! macro `wasmtime_option_group!` which is invoked per-option which enables
//! specifying options in a struct-like syntax where all other boilerplate about
//! option parsing is contained exclusively within this module.

use crate::{KeyValuePair, WasiNnGraph};
use anyhow::{bail, Result};
use clap::builder::{StringValueParser, TypedValueParser, ValueParserFactory};
use clap::error::{Error, ErrorKind};
use serde::de::{self, Visitor};
use std::time::Duration;
use std::{fmt, marker};

/// Characters which can be safely ignored while parsing numeric options to wasmtime
const IGNORED_NUMBER_CHARS: [char; 1] = ['_'];

#[macro_export]
macro_rules! wasmtime_option_group {
    (
        $(#[$attr:meta])*
        pub struct $opts:ident {
            $(
                $(#[doc = $doc:tt])*
                $(#[serde($serde_attr:meta)])*
                pub $opt:ident: $container:ident<$payload:ty>,
            )+

            $(
                #[prefixed = $prefix:tt]
                $(#[serde($serde_attr2:meta)])*
                $(#[doc = $prefixed_doc:tt])*
                pub $prefixed:ident: Vec<(String, Option<String>)>,
            )?
        }
        enum $option:ident {
            ...
        }
    ) => {
        #[derive(Default, Debug)]
        $(#[$attr])*
        pub struct $opts {
            $(
                $(#[serde($serde_attr)])*
                pub $opt: $container<$payload>,
            )+
            $(
                $(#[serde($serde_attr2)])*
                pub $prefixed: Vec<(String, Option<String>)>,
            )?
        }

        #[derive(Clone, PartialEq)]
        #[expect(non_camel_case_types, reason = "macro-generated code")]
        enum $option {
            $(
                $opt($payload),
            )+
            $(
                $prefixed(String, Option<String>),
            )?
        }

        impl $crate::opt::WasmtimeOption for $option {
            const OPTIONS: &'static [$crate::opt::OptionDesc<$option>] = &[
                $(
                    $crate::opt::OptionDesc {
                        name: $crate::opt::OptName::Name(stringify!($opt)),
                        parse: |_, s| {
                            Ok($option::$opt(
                                $crate::opt::WasmtimeOptionValue::parse(s)?
                            ))
                        },
                        val_help: <$payload as $crate::opt::WasmtimeOptionValue>::VAL_HELP,
                        docs: concat!($($doc, "\n",)*),
                    },
                 )+
                $(
                    $crate::opt::OptionDesc {
                        name: $crate::opt::OptName::Prefix($prefix),
                        parse: |name, val| {
                            Ok($option::$prefixed(
                                name.to_string(),
                                val.map(|v| v.to_string()),
                            ))
                        },
                        val_help: "[=val]",
                        docs: concat!($($prefixed_doc, "\n",)*),
                    },
                 )?
            ];
        }

        impl core::fmt::Display for $option {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                match self {
                    $(
                        $option::$opt(val) => {
                            write!(f, "{}=", stringify!($opt).replace('_', "-"))?;
                            $crate::opt::WasmtimeOptionValue::display(val, f)
                        }
                    )+
                    $(
                        $option::$prefixed(key, val) => {
                            write!(f, "{}-{key}", stringify!($prefixed))?;
                            if let Some(val) = val {
                                write!(f, "={val}")?;
                            }
                            Ok(())
                        }
                    )?
                }
            }
        }

        impl $opts {
            fn configure_with(&mut self, opts: &[$crate::opt::CommaSeparated<$option>]) {
                for opt in opts.iter().flat_map(|o| o.0.iter()) {
                    match opt {
                        $(
                            $option::$opt(val) => {
                                $crate::opt::OptionContainer::push(&mut self.$opt, val.clone());
                            }
                        )+
                        $(
                            $option::$prefixed(key, val) => self.$prefixed.push((key.clone(), val.clone())),
                        )?
                    }
                }
            }

            fn to_options(&self) -> Vec<$option> {
                let mut ret = Vec::new();
                $(
                    for item in $crate::opt::OptionContainer::get(&self.$opt) {
                        ret.push($option::$opt(item.clone()));
                    }
                )+
                $(
                    for (key,val) in self.$prefixed.iter() {
                        ret.push($option::$prefixed(key.clone(), val.clone()));
                    }
                )?
                ret
            }
        }
    };
}

/// Parser registered with clap which handles parsing the `...` in `-O ...`.
#[derive(Clone, Debug, PartialEq)]
pub struct CommaSeparated<T>(pub Vec<T>);

impl<T> ValueParserFactory for CommaSeparated<T>
where
    T: WasmtimeOption,
{
    type Parser = CommaSeparatedParser<T>;

    fn value_parser() -> CommaSeparatedParser<T> {
        CommaSeparatedParser(marker::PhantomData)
    }
}

#[derive(Clone)]
pub struct CommaSeparatedParser<T>(marker::PhantomData<T>);

impl<T> TypedValueParser for CommaSeparatedParser<T>
where
    T: WasmtimeOption,
{
    type Value = CommaSeparated<T>;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, Error> {
        let val = StringValueParser::new().parse_ref(cmd, arg, value)?;

        let options = T::OPTIONS;
        let arg = arg.expect("should always have an argument");
        let arg_long = arg.get_long().expect("should have a long name specified");
        let arg_short = arg.get_short().expect("should have a short name specified");

        // Handle `-O help` which dumps all the `-O` options, their messages,
        // and then exits.
        if val == "help" {
            let mut max = 0;
            for d in options {
                max = max.max(d.name.display_string().len() + d.val_help.len());
            }
            println!("Available {arg_long} options:\n");
            for d in options {
                print!(
                    "  -{arg_short} {:>1$}",
                    d.name.display_string(),
                    max - d.val_help.len()
                );
                print!("{}", d.val_help);
                print!(" --");
                if val == "help" {
                    for line in d.docs.lines().map(|s| s.trim()) {
                        if line.is_empty() {
                            break;
                        }
                        print!(" {line}");
                    }
                    println!();
                } else {
                    println!();
                    for line in d.docs.lines().map(|s| s.trim()) {
                        let line = line.trim();
                        println!("        {line}");
                    }
                }
            }
            println!("\npass `-{arg_short} help-long` to see longer-form explanations");
            std::process::exit(0);
        }
        if val == "help-long" {
            println!("Available {arg_long} options:\n");
            for d in options {
                println!(
                    "  -{arg_short} {}{} --",
                    d.name.display_string(),
                    d.val_help
                );
                println!();
                for line in d.docs.lines().map(|s| s.trim()) {
                    let line = line.trim();
                    println!("        {line}");
                }
            }
            std::process::exit(0);
        }

        let mut result = Vec::new();
        for val in val.split(',') {
            // Split `k=v` into `k` and `v` where `v` is optional
            let mut iter = val.splitn(2, '=');
            let key = iter.next().unwrap();
            let key_val = iter.next();

            // Find `key` within `T::OPTIONS`
            let option = options
                .iter()
                .filter_map(|d| match d.name {
                    OptName::Name(s) => {
                        let s = s.replace('_', "-");
                        if s == key {
                            Some((d, s))
                        } else {
                            None
                        }
                    }
                    OptName::Prefix(s) => {
                        let name = key.strip_prefix(s)?.strip_prefix("-")?;
                        Some((d, name.to_string()))
                    }
                })
                .next();

            let (desc, key) = match option {
                Some(pair) => pair,
                None => {
                    let err = Error::raw(
                        ErrorKind::InvalidValue,
                        format!("unknown -{arg_short} / --{arg_long} option: {key}\n"),
                    );
                    return Err(err.with_cmd(cmd));
                }
            };

            result.push((desc.parse)(&key, key_val).map_err(|e| {
                Error::raw(
                    ErrorKind::InvalidValue,
                    format!("failed to parse -{arg_short} option `{val}`: {e:?}\n"),
                )
                .with_cmd(cmd)
            })?)
        }

        Ok(CommaSeparated(result))
    }
}

/// Helper trait used by `CommaSeparated` which contains a list of all options
/// supported by the option group.
pub trait WasmtimeOption: Sized + Send + Sync + Clone + 'static {
    const OPTIONS: &'static [OptionDesc<Self>];
}

pub struct OptionDesc<T> {
    pub name: OptName,
    pub docs: &'static str,
    pub parse: fn(&str, Option<&str>) -> Result<T>,
    pub val_help: &'static str,
}

pub enum OptName {
    /// A named option. Note that the `str` here uses `_` instead of `-` because
    /// it's derived from Rust syntax.
    Name(&'static str),

    /// A prefixed option which strips the specified `name`, then `-`.
    Prefix(&'static str),
}

impl OptName {
    fn display_string(&self) -> String {
        match self {
            OptName::Name(s) => s.replace('_', "-"),
            OptName::Prefix(s) => format!("{s}-<KEY>"),
        }
    }
}

/// A helper trait for all types of options that can be parsed. This is what
/// actually parses the `=val` in `key=val`
pub trait WasmtimeOptionValue: Sized {
    /// Help text for the value to be specified.
    const VAL_HELP: &'static str;

    /// Parses the provided value, if given, returning an error on failure.
    fn parse(val: Option<&str>) -> Result<Self>;

    /// Write the value to `f` that would parse to `self`.
    fn display(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

impl WasmtimeOptionValue for String {
    const VAL_HELP: &'static str = "=val";
    fn parse(val: Option<&str>) -> Result<Self> {
        match val {
            Some(val) => Ok(val.to_string()),
            None => bail!("value must be specified with `key=val` syntax"),
        }
    }

    fn display(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self)
    }
}

impl WasmtimeOptionValue for u32 {
    const VAL_HELP: &'static str = "=N";
    fn parse(val: Option<&str>) -> Result<Self> {
        let val = String::parse(val)?.replace(IGNORED_NUMBER_CHARS, "");
        match val.strip_prefix("0x") {
            Some(hex) => Ok(u32::from_str_radix(hex, 16)?),
            None => Ok(val.parse()?),
        }
    }

    fn display(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

impl WasmtimeOptionValue for u64 {
    const VAL_HELP: &'static str = "=N";
    fn parse(val: Option<&str>) -> Result<Self> {
        let val = String::parse(val)?.replace(IGNORED_NUMBER_CHARS, "");
        match val.strip_prefix("0x") {
            Some(hex) => Ok(u64::from_str_radix(hex, 16)?),
            None => Ok(val.parse()?),
        }
    }

    fn display(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

impl WasmtimeOptionValue for usize {
    const VAL_HELP: &'static str = "=N";
    fn parse(val: Option<&str>) -> Result<Self> {
        let val = String::parse(val)?.replace(IGNORED_NUMBER_CHARS, "");
        match val.strip_prefix("0x") {
            Some(hex) => Ok(usize::from_str_radix(hex, 16)?),
            None => Ok(val.parse()?),
        }
    }

    fn display(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

impl WasmtimeOptionValue for bool {
    const VAL_HELP: &'static str = "[=y|n]";
    fn parse(val: Option<&str>) -> Result<Self> {
        match val {
            None | Some("y") | Some("yes") | Some("true") => Ok(true),
            Some("n") | Some("no") | Some("false") => Ok(false),
            Some(s) => bail!("unknown boolean flag `{s}`, only yes,no,<nothing> accepted"),
        }
    }

    fn display(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self {
            f.write_str("y")
        } else {
            f.write_str("n")
        }
    }
}

impl WasmtimeOptionValue for Duration {
    const VAL_HELP: &'static str = "=N|Ns|Nms|..";
    fn parse(val: Option<&str>) -> Result<Duration> {
        let s = String::parse(val)?;
        // assume an integer without a unit specified is a number of seconds ...
        if let Ok(val) = s.parse() {
            return Ok(Duration::from_secs(val));
        }
        // ... otherwise try to parse it with units such as `3s` or `300ms`
        let dur = humantime::parse_duration(&s)?;
        Ok(dur)
    }

    fn display(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", humantime::format_duration(*self))
    }
}

impl WasmtimeOptionValue for wasmtime::OptLevel {
    const VAL_HELP: &'static str = "=0|1|2|s";
    fn parse(val: Option<&str>) -> Result<Self> {
        match String::parse(val)?.as_str() {
            "0" => Ok(wasmtime::OptLevel::None),
            "1" => Ok(wasmtime::OptLevel::Speed),
            "2" => Ok(wasmtime::OptLevel::Speed),
            "s" => Ok(wasmtime::OptLevel::SpeedAndSize),
            other => bail!(
                "unknown optimization level `{}`, only 0,1,2,s accepted",
                other
            ),
        }
    }

    fn display(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            wasmtime::OptLevel::None => f.write_str("0"),
            wasmtime::OptLevel::Speed => f.write_str("2"),
            wasmtime::OptLevel::SpeedAndSize => f.write_str("s"),
            _ => unreachable!(),
        }
    }
}

impl WasmtimeOptionValue for wasmtime::RegallocAlgorithm {
    const VAL_HELP: &'static str = "=backtracking|single-pass";
    fn parse(val: Option<&str>) -> Result<Self> {
        match String::parse(val)?.as_str() {
            "backtracking" => Ok(wasmtime::RegallocAlgorithm::Backtracking),
            "single-pass" => Ok(wasmtime::RegallocAlgorithm::SinglePass),
            other => bail!(
                "unknown regalloc algorithm`{}`, only backtracking,single-pass accepted",
                other
            ),
        }
    }

    fn display(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            wasmtime::RegallocAlgorithm::Backtracking => f.write_str("backtracking"),
            wasmtime::RegallocAlgorithm::SinglePass => f.write_str("single-pass"),
            _ => unreachable!(),
        }
    }
}

impl WasmtimeOptionValue for wasmtime::Strategy {
    const VAL_HELP: &'static str = "=winch|cranelift";
    fn parse(val: Option<&str>) -> Result<Self> {
        match String::parse(val)?.as_str() {
            "cranelift" => Ok(wasmtime::Strategy::Cranelift),
            "winch" => Ok(wasmtime::Strategy::Winch),
            other => bail!("unknown compiler `{other}` only `cranelift` and `winch` accepted",),
        }
    }

    fn display(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            wasmtime::Strategy::Cranelift => f.write_str("cranelift"),
            wasmtime::Strategy::Winch => f.write_str("winch"),
            _ => unreachable!(),
        }
    }
}

impl WasmtimeOptionValue for wasmtime::Collector {
    const VAL_HELP: &'static str = "=drc|null";
    fn parse(val: Option<&str>) -> Result<Self> {
        match String::parse(val)?.as_str() {
            "drc" => Ok(wasmtime::Collector::DeferredReferenceCounting),
            "null" => Ok(wasmtime::Collector::Null),
            other => bail!("unknown collector `{other}` only `drc` and `null` accepted",),
        }
    }

    fn display(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            wasmtime::Collector::DeferredReferenceCounting => f.write_str("drc"),
            wasmtime::Collector::Null => f.write_str("null"),
            _ => unreachable!(),
        }
    }
}

impl WasmtimeOptionValue for wasmtime::MpkEnabled {
    const VAL_HELP: &'static str = "[=y|n|auto]";
    fn parse(val: Option<&str>) -> Result<Self> {
        match val {
            None | Some("y") | Some("yes") | Some("true") => Ok(wasmtime::MpkEnabled::Enable),
            Some("n") | Some("no") | Some("false") => Ok(wasmtime::MpkEnabled::Disable),
            Some("auto") => Ok(wasmtime::MpkEnabled::Auto),
            Some(s) => bail!("unknown mpk flag `{s}`, only yes,no,auto,<nothing> accepted"),
        }
    }

    fn display(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            wasmtime::MpkEnabled::Enable => f.write_str("y"),
            wasmtime::MpkEnabled::Disable => f.write_str("n"),
            wasmtime::MpkEnabled::Auto => f.write_str("auto"),
        }
    }
}

impl WasmtimeOptionValue for WasiNnGraph {
    const VAL_HELP: &'static str = "=<format>::<dir>";
    fn parse(val: Option<&str>) -> Result<Self> {
        let val = String::parse(val)?;
        let mut parts = val.splitn(2, "::");
        Ok(WasiNnGraph {
            format: parts.next().unwrap().to_string(),
            dir: match parts.next() {
                Some(part) => part.into(),
                None => bail!("graph does not contain `::` separator for directory"),
            },
        })
    }

    fn display(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}", self.format, self.dir)
    }
}

impl WasmtimeOptionValue for KeyValuePair {
    const VAL_HELP: &'static str = "=<name>=<val>";
    fn parse(val: Option<&str>) -> Result<Self> {
        let val = String::parse(val)?;
        let mut parts = val.splitn(2, "=");
        Ok(KeyValuePair {
            key: parts.next().unwrap().to_string(),
            value: match parts.next() {
                Some(part) => part.into(),
                None => "".to_string(),
            },
        })
    }

    fn display(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.key)?;
        if !self.value.is_empty() {
            f.write_str("=")?;
            f.write_str(&self.value)?;
        }
        Ok(())
    }
}

pub trait OptionContainer<T> {
    fn push(&mut self, val: T);
    fn get<'a>(&'a self) -> impl Iterator<Item = &'a T>
    where
        T: 'a;
}

impl<T> OptionContainer<T> for Option<T> {
    fn push(&mut self, val: T) {
        *self = Some(val);
    }
    fn get<'a>(&'a self) -> impl Iterator<Item = &'a T>
    where
        T: 'a,
    {
        self.iter()
    }
}

impl<T> OptionContainer<T> for Vec<T> {
    fn push(&mut self, val: T) {
        Vec::push(self, val);
    }
    fn get<'a>(&'a self) -> impl Iterator<Item = &'a T>
    where
        T: 'a,
    {
        self.iter()
    }
}

// Used to parse toml values into string so that we can reuse the `WasmtimeOptionValue::parse`
// for parsing toml values the same way we parse command line values.
//
// Used for wasmtime::Strategy, wasmtime::Collector, wasmtime::OptLevel, wasmtime::RegallocAlgorithm
struct ToStringVisitor {}

impl<'de> Visitor<'de> for ToStringVisitor {
    type Value = String;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "&str, u64, or i64")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(s.to_owned())
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(v.to_string())
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(v.to_string())
    }
}

// Deserializer that uses the `WasmtimeOptionValue::parse` to parse toml values
pub(crate) fn cli_parse_wrapper<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: WasmtimeOptionValue,
    D: serde::Deserializer<'de>,
{
    let to_string_visitor = ToStringVisitor {};
    let str = deserializer.deserialize_any(to_string_visitor)?;

    T::parse(Some(&str))
        .map(Some)
        .map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
    use super::WasmtimeOptionValue;

    #[test]
    fn numbers_with_underscores() {
        assert!(<u32 as WasmtimeOptionValue>::parse(Some("123")).is_ok_and(|v| v == 123));
        assert!(<u32 as WasmtimeOptionValue>::parse(Some("1_2_3")).is_ok_and(|v| v == 123));
    }
}
