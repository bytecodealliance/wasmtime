//! Contains the code-generation logic to emit for the DSL-defined instructions.

mod features;
mod format;
mod inst;
mod operand;

use crate::dsl;
use cranelift_srcgen::{Formatter, fmtln};

/// Generate the Rust assembler code; e.g., `enum Inst { ... }`.
pub fn rust_assembler(f: &mut Formatter, insts: &[dsl::Inst]) {
    // Generate "all instructions" enum.
    generate_inst_enum(f, insts);
    generate_inst_impls(f, insts);
    generate_inst_display_impl(f, insts);

    // Generate per-instruction structs.
    f.empty_line();
    for inst in insts {
        inst.generate_struct(f);
        inst.generate_struct_impl(f);
        inst.generate_display_impl(f);
        inst.generate_from_impl(f);
        f.empty_line();
    }

    // Generate the `Feature` trait.
    dsl::Feature::generate_macro(f);
}

/// `enum Inst { ... }`
fn generate_inst_enum(f: &mut Formatter, insts: &[dsl::Inst]) {
    fmtln!(f, "#[doc(hidden)]");
    generate_derive(f);
    generate_derive_arbitrary_bounds(f);
    f.add_block("pub enum Inst<R: Registers>", |f| {
        for inst in insts {
            let variant_name = inst.name();
            let struct_name = inst.struct_name_with_generic();
            fmtln!(f, "{variant_name}({struct_name}),");
        }
    });
}

/// Helper for emitting `match self { ... }` blocks over all instructions. For each instruction in
/// `insts`, this generate a separate match arm containing `invoke`.
fn match_variants(f: &mut Formatter, insts: &[dsl::Inst], invoke: &str) {
    f.add_block("match self", |f| {
        for inst in insts.iter().map(|i| i.name()) {
            fmtln!(f, "Self::{inst}(i) => i.{invoke},");
        }
    });
}

/// `impl core::fmt::Display for Inst { ... }`
fn generate_inst_display_impl(f: &mut Formatter, insts: &[dsl::Inst]) {
    f.add_block("impl<R: Registers> core::fmt::Display for Inst<R>", |f| {
        f.add_block(
            "fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result",
            |f| {
                match_variants(f, insts, "fmt(f)");
            },
        );
    });
}

fn generate_inst_impls(f: &mut Formatter, insts: &[dsl::Inst]) {
    f.add_block("impl<R: Registers> Inst<R>", |f| {
        f.add_block("pub fn encode(&self, b: &mut impl CodeSink)", |f| {
            match_variants(f, insts, "encode(b)");
        });
        f.add_block(
            "pub fn visit(&mut self, v: &mut impl RegisterVisitor<R>)",
            |f| {
                match_variants(f, insts, "visit(v)");
            },
        );
        f.add_block(
            "pub fn is_available(&self, f: &impl AvailableFeatures) -> bool",
            |f| {
                match_variants(f, insts, "is_available(f)");
            },
        );
        f.add_block("pub fn features(&self) -> &'static Features", |f| {
            match_variants(f, insts, "features()");
        });
        f.add_block("pub fn num_registers_available(&self) -> usize", |f| {
            match_variants(f, insts, "num_registers_available()");
        });
    });
}

/// `#[derive(...)]`
fn generate_derive(f: &mut Formatter) {
    fmtln!(f, "#[derive(Copy, Clone, Debug)]");
    fmtln!(
        f,
        "#[cfg_attr(any(test, feature = \"fuzz\"), derive(arbitrary::Arbitrary))]"
    );
}

/// Adds a custom bound to the `Arbitrary` implementation which ensures that
/// the associated registers are all `Arbitrary` as well.
fn generate_derive_arbitrary_bounds(f: &mut Formatter) {
    fmtln!(
        f,
        "#[cfg_attr(any(test, feature = \"fuzz\"), arbitrary(bound = \"R: crate::fuzz::RegistersArbitrary\"))]"
    );
}
