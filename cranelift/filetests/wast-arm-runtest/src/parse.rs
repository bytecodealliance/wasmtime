use crate::cases::Case;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

pub struct ParsedWast {
    pub cases: Vec<Case>,
    #[allow(dead_code)]
    pub skipped: u32,
    pub command_count: usize,
    pub module_count: usize,
}

pub fn parse_wast_file(path: &Path, verbose: bool) -> Result<ParsedWast> {
    let bytes =
        std::fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let text = std::str::from_utf8(&bytes).context("wast file not utf-8")?;

    let buf = wast::parser::ParseBuffer::new(text).context("lex wast")?;
    let ast = wast::parser::parse::<wast::Wast>(&buf).context("parse wast")?;
    let filename = path.to_string_lossy();

    // Convert and extract data while text is still alive
    let wast = json_from_wast::Opts::default()
        .convert(&filename, text, ast)
        .map_err(|e| anyhow::anyhow!("convert wast: {e:?}"))?;

    // Move wasms (owned) into a name -> bytes map
    let wasms: HashMap<String, Vec<u8>> = wast.wasms.into_iter().collect();

    let command_count = wast.commands.len();
    let module_count = wasms.len();
    let mut cases = Vec::new();
    let mut skipped = 0u32;
    let mut current: Option<&[u8]> = None;

    // Walk commands while borrowed data is alive
    for cmd in &wast.commands {
        match cmd {
            json_from_wast::Command::Module { file, .. } => {
                current = wasms.get(file.filename.as_ref()).map(|v| v.as_slice());
            }
            json_from_wast::Command::AssertReturn {
                action, expected, ..
            } => {
                let json_from_wast::Action::Invoke { field, args, .. } = action else {
                    skipped += 1;
                    continue;
                };
                let Some(module) = current else {
                    skipped += 1;
                    continue;
                };
                match Case::try_build(module, field.as_ref(), args, expected) {
                    Ok(Some(case)) => cases.push(case),
                    Ok(None) => skipped += 1,
                    Err(reason) => {
                        if verbose {
                            eprintln!("skip: {reason}");
                        }
                        skipped += 1;
                    }
                }
            }
            _ => skipped += 1,
        }
    }

    Ok(ParsedWast {
        cases,
        skipped,
        command_count,
        module_count,
    })
}
