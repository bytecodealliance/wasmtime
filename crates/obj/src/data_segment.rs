use anyhow::Result;
use faerie::{Artifact, Decl};
use wasmtime_environ::DataInitializer;

/// Declares data segment symbol
pub fn declare_data_segment(
    obj: &mut Artifact,
    _data_initaliazer: &DataInitializer,
    index: usize,
) -> Result<()> {
    let name = format!("_memory_{}", index);
    obj.declare(name, Decl::data())?;
    Ok(())
}

/// Emit segment data and initialization location
pub fn emit_data_segment(
    obj: &mut Artifact,
    data_initaliazer: &DataInitializer,
    index: usize,
) -> Result<()> {
    let name = format!("_memory_{}", index);
    obj.define(name, Vec::from(data_initaliazer.data))?;
    Ok(())
}
