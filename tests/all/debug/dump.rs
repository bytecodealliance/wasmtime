use anyhow::{bail, Result};
use std::env;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum DwarfDumpSection {
    DebugInfo,
    DebugLine,
}

pub fn get_dwarfdump(obj: &str, section: DwarfDumpSection) -> Result<String> {
    let dwarfdump = env::var("DWARFDUMP").unwrap_or("llvm-dwarfdump".to_string());
    let section_flag = match section {
        DwarfDumpSection::DebugInfo => "-debug-info",
        DwarfDumpSection::DebugLine => "-debug-line",
    };
    let output = Command::new(&dwarfdump)
        .args(&[section_flag, obj])
        .output()
        .expect("success");
    if !output.status.success() {
        bail!(
            "failed to execute {}: {}",
            dwarfdump,
            String::from_utf8_lossy(&output.stderr),
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
