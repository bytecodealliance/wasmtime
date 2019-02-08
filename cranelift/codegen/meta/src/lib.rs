#[macro_use]
mod cdsl;

pub mod error;
pub mod isa;

mod gen_registers;
mod gen_settings;
mod gen_types;

mod base;
mod constant_hash;
mod srcgen;
mod unique_table;

pub fn isa_from_arch(arch: &str) -> Result<Vec<isa::Isa>, String> {
    isa::Isa::from_arch(arch)
        .ok_or_else(|| format!("no supported isa found for arch `{}`", arch))
        .and_then(|isa| Ok(vec![isa]))
}

pub fn isas_from_targets(targets: Vec<&str>) -> Result<Vec<isa::Isa>, String> {
    type R<'a> = Vec<(&'a str, Option<isa::Isa>)>;

    let (known, unknown): (R, R) = targets
        .into_iter()
        .map(|target| (target, isa::Isa::from_name(target)))
        .partition(|(_, opt_isa)| opt_isa.is_some());

    if !unknown.is_empty() {
        let unknown_targets = unknown
            .into_iter()
            .map(|(target, _)| target)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!("unknown isa targets: {}", unknown_targets));
    }

    let isas = if known.is_empty() {
        isa::Isa::all().to_vec()
    } else {
        known
            .into_iter()
            .map(|(_, opt_isa)| opt_isa.unwrap())
            .collect()
    };

    Ok(isas)
}

pub fn all_isas() -> Result<Vec<isa::Isa>, String> {
    isas_from_targets(vec![])
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
