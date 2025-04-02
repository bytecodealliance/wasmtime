//! Contains the code-generation logic to emit for the DSL-defined instructions.

mod features;
mod format;
mod inst;
mod operand;

use crate::dsl;
use cranelift_srcgen::{fmtln, Formatter};

/// Generate the Rust assembler code; e.g., `enum Inst { ... }`.
pub fn rust_assembler(f: &mut Formatter, insts: &[dsl::Inst]) {
    // Generate "all instructions" enum.
    generate_inst_enum(f, insts);
    generate_inst_display_impl(f, insts);
    generate_inst_encode_impl(f, insts);
    generate_inst_visit_impl(f, insts);
    generate_inst_features_impl(f, insts);

    // Generate per-instruction structs.
    f.empty_line();
    for inst in insts {
        inst.generate_struct(f);
        inst.generate_struct_impl(f);
        inst.generate_display_impl(f);
        inst.generate_from_impl(f);
        f.empty_line();
    }

    // Generate the `Feature` enum.
    dsl::Feature::generate_enum(f);
}

/// `enum Inst { ... }`
fn generate_inst_enum(f: &mut Formatter, insts: &[dsl::Inst]) {
    fmtln!(f, "#[doc(hidden)]");
    generate_derive(f);
    generate_derive_arbitrary_bounds(f);
    fmtln!(f, "pub enum Inst<R: Registers> {{");
    f.indent_push();
    for inst in insts {
        let variant_name = inst.name();
        let struct_name = inst.struct_name_with_generic();
        fmtln!(f, "{variant_name}({struct_name}),");
    }
    f.indent_pop();
    fmtln!(f, "}}");
}

/// `#[derive(...)]`
fn generate_derive(f: &mut Formatter) {
    fmtln!(f, "#[derive(Clone, Debug)]");
    fmtln!(f, "#[cfg_attr(any(test, feature = \"fuzz\"), derive(arbitrary::Arbitrary))]");
}

/// Adds a custom bound to the `Arbitrary` implementation which ensures that
/// the associated registers are all `Arbitrary` as well.
fn generate_derive_arbitrary_bounds(f: &mut Formatter) {
    fmtln!(
        f,
        "#[cfg_attr(any(test, feature = \"fuzz\"), arbitrary(bound = \"R: crate::fuzz::RegistersArbitrary\"))]"
    );
}

/// `impl std::fmt::Display for Inst { ... }`
fn generate_inst_display_impl(f: &mut Formatter, insts: &[dsl::Inst]) {
    fmtln!(f, "impl<R: Registers> std::fmt::Display for Inst<R> {{");
    f.indent(|f| {
        fmtln!(f, "fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{");
        f.indent(|f| {
            fmtln!(f, "match self {{");
            f.indent_push();
            for inst in insts {
                let variant_name = inst.name();
                fmtln!(f, "Self::{variant_name}(i) => write!(f, \"{{i}}\"),");
            }
            f.indent_pop();
            fmtln!(f, "}}");
        });
        fmtln!(f, "}}");
    });
    fmtln!(f, "}}");
}

/// `impl Inst { fn encode... }`
fn generate_inst_encode_impl(f: &mut Formatter, insts: &[dsl::Inst]) {
    fmtln!(f, "impl<R: Registers> Inst<R> {{");
    f.indent(|f| {
        fmtln!(f, "pub fn encode(&self, b: &mut impl CodeSink, o: &impl KnownOffsetTable) {{");
        f.indent(|f| {
            fmtln!(f, "match self {{");
            f.indent_push();
            for inst in insts {
                let variant_name = inst.name();
                fmtln!(f, "Self::{variant_name}(i) => i.encode(b, o),");
            }
            f.indent_pop();
            fmtln!(f, "}}");
        });
        fmtln!(f, "}}");
    });
    fmtln!(f, "}}");
}

/// `impl Inst { fn visit... }`
fn generate_inst_visit_impl(f: &mut Formatter, insts: &[dsl::Inst]) {
    fmtln!(f, "impl<R: Registers> Inst<R> {{");
    f.indent(|f| {
        fmtln!(f, "pub fn visit(&mut self, v: &mut impl RegisterVisitor<R>) {{");
        f.indent(|f| {
            fmtln!(f, "match self {{");
            f.indent_push();
            for inst in insts {
                let variant_name = inst.name();
                fmtln!(f, "Self::{variant_name}(i) => i.visit(v),");
            }
            f.indent_pop();
            fmtln!(f, "}}");
        });
        fmtln!(f, "}}");
    });
    fmtln!(f, "}}");
}

/// `impl Inst { fn features... }`
fn generate_inst_features_impl(f: &mut Formatter, insts: &[dsl::Inst]) {
    fmtln!(f, "impl<R: Registers> Inst<R> {{");
    f.indent(|f| {
        fmtln!(f, "#[must_use]");
        fmtln!(f, "pub fn features(&self) -> Vec<Feature> {{");
        f.indent(|f| {
            fmtln!(f, "match self {{");
            f.indent_push();
            for inst in insts {
                let variant_name = inst.name();
                fmtln!(f, "Self::{variant_name}(i) => i.features(),");
            }
            f.indent_pop();
            fmtln!(f, "}}");
        });
        fmtln!(f, "}}");
    });
    fmtln!(f, "}}");
}
