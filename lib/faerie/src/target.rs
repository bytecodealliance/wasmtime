use cretonne_codegen::isa;
use cretonne_module::ModuleError;
use faerie::Target;

/// Translate from a Cretonne `TargetIsa` to a Faerie `Target`.
pub fn translate(isa: &isa::TargetIsa) -> Result<Target, ModuleError> {
    let name = isa.name();
    match name {
        "x86" => Ok(if isa.flags().is_64bit() {
            Target::X86_64
        } else {
            Target::X86
        }),
        "arm32" => Ok(Target::ARMv7),
        "arm64" => Ok(Target::ARM64),
        _ => Err(ModuleError::Backend(format!(
            "unsupported faerie isa: {}",
            name
        ))),
    }
}
