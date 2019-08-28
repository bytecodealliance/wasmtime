use faerie::{Artifact, Decl};

/// Declares data segment symbol
pub fn declare_table(obj: &mut Artifact, index: usize) -> Result<(), String> {
    let name = format!("_table_{}", index);
    obj.declare(name, Decl::data())
        .map_err(|err| format!("{}", err))?;
    Ok(())
}

/// Emit segment data and initialization location
pub fn emit_table(obj: &mut Artifact, index: usize) -> Result<(), String> {
    let name = format!("_table_{}", index);
    // FIXME: We need to initialize table using function symbols
    obj.define(name, Vec::new())
        .map_err(|err| format!("{}", err))?;
    Ok(())
}
