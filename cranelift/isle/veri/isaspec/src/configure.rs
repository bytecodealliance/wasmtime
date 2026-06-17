use std::collections::HashMap;

use anyhow::{Result, bail};
use cranelift_codegen::{Reg, RegClass, isa::aarch64::inst::Inst};

use crate::{
    aarch64::{self, pstate_field},
    bits::Bits,
    builder::{MappingBuilder, Mappings},
    constraints::Target,
    spec::{spec_as_bit_vector_width, spec_conv_to, spec_var},
};
use cranelift_isle::ast::SpecExpr;

#[macro_export(local_inner_macros)]
macro_rules! spec_config {
    (($family:ident $($arg:ident)+) { $($conf:tt)* }) => {
        __mappings! { @init mappings $($conf)* };
        SpecConfig {
            term: std::concat!("MInst.", std::stringify!($family)).to_string(),
            args: [$(std::stringify!($arg),)+]
                .map(String::from)
                .to_vec(),
            cases: __cases! {
                (
                    family: $family,
                    args: ($($arg)+),
                    mappings: mappings
                ),
                $($conf)*
            },
        }
    };
}

#[macro_export(local_inner_macros)]
macro_rules! __mappings {
    (@init $mappings:ident $($conf:tt)*) => {
        let mut $mappings = Mappings::default();
        // TODO: move boolean literals to a library function.
        $mappings
            .reads
            .insert(aarch64::literal("TRUE"), Mapping::allow(spec_true()));
        $mappings
            .reads
            .insert(aarch64::literal("FALSE"), Mapping::allow(spec_false()));
        __mappings! { @conf $mappings $($conf)* }
    };

    // Read and write mappings for general-purpose registers.
    (@conf $mappings:ident register ($name:ident, read, gp, $id:literal); $($conf:tt)*) => {
        $mappings.reads.insert(
            aarch64::gpreg($id),
            Mapping::require(spec_var(std::stringify!($name).to_string())),
        );
        let $name = xreg($id);
        __mappings! { @conf $mappings $($conf)* }
    };

    (@conf $mappings:ident register ($name:ident, read, gp64, $id:literal); $($conf:tt)*) => {
        $mappings.reads.insert(
            aarch64::gpreg($id),
            Mapping::require(spec_as_bit_vector_width(spec_var(std::stringify!($name).to_string()), 64)),
        );
        let $name = xreg($id);
        __mappings! { @conf $mappings $($conf)* }
    };

    (@conf $mappings:ident register ($name:ident, write, gp, $id:literal); $($conf:tt)*) => {
        $mappings.writes.insert(
            aarch64::gpreg($id),
            Mapping::require(spec_var(std::stringify!($name).to_string())),
        );
        let $name = writable_xreg($id);
        __mappings! { @conf $mappings $($conf)* }
    };

    // Read and write mappings for floating-point registers.
    (@conf $mappings:ident register ($name:ident, read, fp, $id:literal); $($conf:tt)*) => {
        $mappings.reads.insert(
            aarch64::vreg($id),
            Mapping::require($crate::configure::spec_fp_reg(std::stringify!($name))),
        );
        let $name = vreg($id);
        __mappings! { @conf $mappings $($conf)* }
    };

    (@conf $mappings:ident register ($name:ident, write, fp, $id:literal); $($conf:tt)*) => {
        $mappings.writes.insert(
            aarch64::vreg($id),
            Mapping::require($crate::configure::spec_fp_reg(std::stringify!($name))),
        );
        let $name = writable_vreg($id);
        __mappings! { @conf $mappings $($conf)* }
    };

    (@conf $mappings:ident fpcr (); $($conf:tt)*) => {
        $mappings
            .reads
            .insert(aarch64::fpcr(), MappingBuilder::var("fpcr").allow().build());
        __mappings! { @conf $mappings $($conf)* }
    };

    // Read and write mappings for full vector/floating-point registers.
    (@conf $mappings:ident register ($name:ident, read, vec, $id:literal); $($conf:tt)*) => {
        $mappings.reads.insert(
            aarch64::vreg($id),
            Mapping::require(spec_var(std::stringify!($name).to_string())),
        );
        let $name = vreg($id);
        __mappings! { @conf $mappings $($conf)* }
    };

    (@conf $mappings:ident register ($name:ident, write, vec, $id:literal); $($conf:tt)*) => {
        $mappings.writes.insert(
            aarch64::vreg($id),
            Mapping::require(spec_var(std::stringify!($name).to_string())),
        );
        let $name = writable_vreg($id);
        __mappings! { @conf $mappings $($conf)* }
    };

    // Flags mappings.
    (@conf $mappings:ident flags (); $($conf:tt)*) => {
        $crate::configure::configure_flags_mappings(&mut $mappings);
        __mappings! { @conf $mappings $($conf)* }
    };

    (@conf $mappings:ident $directive:ident $args:tt; $($conf:tt)*) => {
        __mappings! { @conf $mappings $($conf)* }
    };

    (@conf $mappings:ident) => {};
}

#[macro_export(local_inner_macros)]
macro_rules! __cases {
    ($meta:tt, enumerate ($it:ident, $arms:ident); $($tt:tt)+) => {
        Cases::Match(Match {
            on: spec_var(std::stringify!($it).to_string()),
            arms: $arms
                .iter()
                .copied()
                .flat_map(|$it| {
                    let body = __cases! { $meta, $($tt)* };
                    Some(Arm {
                    variant: std::format!("{:?}", $it),
                    args: Vec::new(),
                    body,
                })
            }).collect(),
        })
    };

    ($meta:tt, filter ($expr:expr); $($tt:tt)+) => {
        if $expr {
            __cases! { $meta, $($tt)* }
        } else {
            return None;
        }
    };

    ((family: $family:ident, args: ($($arg:ident)+), mappings: $mappings:ident), instruction ();) => {
        Cases::Instruction(InstConfig {
            opcodes: Opcodes::Instruction(Inst::$family{
                $($arg,)+
            }),
            scope: aarch64::state(),
            mappings: $mappings.clone(),
        })
    };

    ($meta:tt, $directive:ident $args:tt; $($tt:tt)*) => {
        __cases! { $meta, $($tt)* }
    };
}

pub fn flags_mappings() -> Mappings {
    let mut mappings = Mappings::default();
    configure_flags_mappings(&mut mappings);
    mappings
}

pub fn configure_flags_mappings(mappings: &mut Mappings) {
    // Instruction model is the MInst value itself, which is considered the result of the variant term.
    let inst = MappingBuilder::var("result").allow();

    // Input and output flags of the instruction are fields of the MInst model.
    let flags_in = inst.clone().field("flags_in");
    let flags_out = inst.clone().field("flags_out");

    // Construct read and write mappings for each NZCV field.
    for field in &["N", "Z", "C", "V"] {
        // Read
        mappings
            .reads
            .insert(pstate_field(field), flags_in.clone().field(field).build());

        // Write
        mappings
            .writes
            .insert(pstate_field(field), flags_out.clone().field(field).build());
    }
}

// Spec expression for the lower 64 bits of a 128-bit floating-point register.
pub fn spec_fp_reg(name: &str) -> SpecExpr {
    spec_conv_to(
        128,
        spec_as_bit_vector_width(spec_var(name.to_string()), 64),
    )
}

// Compare an opcode template against the instruction we expect it to represent.
pub fn verify_opcode_template<F>(template: &Bits, expect: F) -> Result<()>
where
    F: Fn(&HashMap<String, u32>) -> Result<Inst>,
{
    // Iterate over all template values.
    for concrete in template.into_iter() {
        let inst = expect(&concrete.assignment)?;
        let opcode = aarch64::opcode(&inst);
        let got = concrete.eval()?;
        if got != opcode {
            bail!(
                "template mismatch: opcode {:#x}, template {:#x}",
                opcode,
                got,
            );
        }
    }
    Ok(())
}

// Convert a Cranelift register to the corresponding element of AArch64 state in
// ASLp.
pub fn reg_target(reg: Reg) -> Result<Target> {
    let Some(preg) = reg.to_real_reg() else {
        bail!("not physical register")
    };
    let index = preg.hw_enc().into();
    Ok(match preg.class() {
        RegClass::Int => aarch64::gpreg(index),
        RegClass::Float | RegClass::Vector => aarch64::vreg(index),
    })
}
