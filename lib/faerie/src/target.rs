use cretonne_codegen::isa;
use faerie::Target;
use failure::Error;

/// Translate from a Cretonne `TargetIsa` to a Faerie `Target`.
pub fn translate(isa: &isa::TargetIsa) -> Result<Target, Error> {
    let name = isa.name();
    match name {
        "x86" => Ok(if isa.flags().is_64bit() {
            Target::X86_64
        } else {
            Target::X86
        }),
        "arm32" => Ok(Target::ARMv7),
        "arm64" => Ok(Target::ARM64),
        _ => Err(format_err!("unsupported isa: {}", name)),
    }
}
