use anyhow::Result;
use faerie::{Artifact, Decl};

/// Declares data segment symbol
pub fn declare_table(obj: &mut Artifact, index: usize) -> Result<()> {
    let name = format!("_table_{}", index);
    obj.declare(name, Decl::data())?;
    Ok(())
}

/// Emit segment data and initialization location
pub fn emit_table(obj: &mut Artifact, index: usize) -> Result<()> {
    let name = format!("_table_{}", index);
    // FIXME: We need to initialize table using function symbols
    obj.define(name, Vec::new())?;
    Ok(())
}
