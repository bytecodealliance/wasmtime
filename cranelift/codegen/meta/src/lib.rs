#[macro_use]
mod cdsl;
mod srcgen;

pub mod error;
pub mod isa;

mod gen_registers;
mod gen_settings;
mod gen_types;

mod constant_hash;
mod shared;
mod unique_table;

pub fn isa_from_arch(arch: &str) -> Result<isa::Isa, String> {
    isa::Isa::from_arch(arch).ok_or_else(|| format!("no supported isa found for arch `{}`", arch))
}

/// Generates all the Rust source files used in Cranelift from the meta-language.
pub fn generate(isas: &Vec<isa::Isa>, out_dir: &str) -> Result<(), error::Error> {
    // Common definitions.
    let shared_settings = gen_settings::generate_common("new_settings.rs", &out_dir)?;

    gen_types::generate("types.rs", &out_dir)?;

    // Per ISA definitions.
    let isas = isa::define(isas, &shared_settings);

    for isa in isas {
        gen_registers::generate(&isa, "registers", &out_dir)?;
        gen_settings::generate(&isa, "new_settings", &out_dir)?;
    }

    Ok(())
}
