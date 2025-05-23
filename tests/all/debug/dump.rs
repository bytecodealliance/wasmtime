use anyhow::{Context, Result, bail};
use std::env;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DwarfDumpSection {
    DebugInfo,
}

pub fn get_dwarfdump(obj: &str, section: DwarfDumpSection) -> Result<String> {
    let dwarfdump = env::var("DWARFDUMP").unwrap_or("llvm-dwarfdump".to_string());
    let section_flag = match section {
        DwarfDumpSection::DebugInfo => "-debug-info",
    };
    let output = Command::new(&dwarfdump)
        .args(&[section_flag, obj])
        .output()
        .context(format!("failed to spawn `{dwarfdump}`"))?;
    if !output.status.success() {
        bail!(
            "failed to execute {}: {}",
            dwarfdump,
            String::from_utf8_lossy(&output.stderr),
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
