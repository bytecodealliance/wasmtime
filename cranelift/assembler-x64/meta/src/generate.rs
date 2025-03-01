//! Contains the code-generation logic to emit for the DSL-defined instructions.

mod features;
mod format;
mod formatter;
mod inst;
mod operand;

use crate::dsl;
use formatter::fmtln;
pub use formatter::{maybe_file_loc, Formatter};

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

/// Generate the `isle_assembler_methods!` macro.
pub fn isle_macro(f: &mut Formatter, insts: &[dsl::Inst]) {
    fmtln!(f, "#[macro_export]");
    fmtln!(f, "macro_rules! isle_assembler_methods {{");
    f.indent(|f| {
        fmtln!(f, "() => {{");
        f.indent(|f| {
            for inst in insts {
                inst.generate_isle_macro(f);
            }
        });
        fmtln!(f, "}};");
    });
    fmtln!(f, "}}");
}

/// Generate the ISLE definitions that match the `isle_assembler_methods!` macro
/// above.
pub fn isle_definitions(f: &mut Formatter, insts: &[dsl::Inst]) {
    f.line("(type AssemblerImm8 extern (enum))", None);
    f.line("(type AssemblerSimm8 extern (enum))", None);
    f.line("(type AssemblerImm16 extern (enum))", None);
    f.line("(type AssemblerSimm16 extern (enum))", None);
    f.line("(type AssemblerImm32 extern (enum))", None);
    f.line("(type AssemblerSimm32 extern (enum))", None);
    f.line("(type AssemblerReadGpr extern (enum))", None);
    f.line("(type AssemblerReadWriteGpr extern (enum))", None);
    f.line("(type AssemblerReadGprMem extern (enum))", None);
    f.line("(type AssemblerReadWriteGprMem extern (enum))", None);
    f.line("(type AssemblerInst extern (enum))", None);
    f.line("(type AssemblerReadXmm extern (enum))", None);
    f.line("(type AssemblerReadWriteXmm extern (enum))", None);
    f.line("(type AssemblerReadXmmMem extern (enum))", None);
    f.line("(type AssemblerReadWriteXmmMem extern (enum))", None);
    f.empty_line();

    f.line("(type AssemblerOutputs (enum", None);
    f.line("    ;; Used for instructions that have ISLE `SideEffect`s (memory stores, traps,", None);
    f.line("    ;; etc.) and do not return a `Value`.", None);
    f.line("    (SideEffect (inst MInst))", None);
    f.line("    ;; Used for instructions that return a GPR (including `GprMem` variants with", None);
    f.line("    ;; a GPR as the first argument).", None);
    f.line("    (RetGpr (inst MInst) (gpr Gpr))", None);
    f.line("    ;; Used for instructions that return an XMM register.", None);
    f.line("    (RetXmm (inst MInst) (xmm Xmm))", None);
    f.line("    ;; TODO: eventually add more variants for multi-return, XMM, etc.; see", None);
    f.line("    ;; https://github.com/bytecodealliance/wasmtime/pull/10276", None);
    f.line("))", None);
    f.empty_line();

    f.line(";; Directly emit instructions that return a GPR.", None);
    f.line("(decl emit_ret_gpr (AssemblerOutputs) Gpr)", None);
    f.line("(rule (emit_ret_gpr (AssemblerOutputs.RetGpr inst gpr))", None);
    f.line("    (let ((_ Unit (emit inst))) gpr))", None);
    f.empty_line();

    f.line(";; Directly emit instructions that return an XMM register.", None);
    f.line("(decl emit_ret_xmm (AssemblerOutputs) Xmm)", None);
    f.line("(rule (emit_ret_xmm (AssemblerOutputs.RetXmm inst xmm))", None);
    f.line("    (let ((_ Unit (emit inst))) xmm))", None);
    f.empty_line();

    f.line(";; Pass along the side-effecting instruction for later emission.", None);
    f.line("(decl defer_side_effect (AssemblerOutputs) SideEffectNoResult)", None);
    f.line("(rule (defer_side_effect (AssemblerOutputs.SideEffect inst))", None);
    f.line("    (SideEffectNoResult.Inst inst))", None);
    f.empty_line();

    for inst in insts {
        inst.generate_isle_definition(f);
        f.empty_line();
    }
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
    f.line("#[derive(Clone, Debug)]", None);
    f.line("#[cfg_attr(any(test, feature = \"fuzz\"), derive(arbitrary::Arbitrary))]", None);
}

/// Adds a custom bound to the `Arbitrary` implementation which ensures that
/// the associated registers are all `Arbitrary` as well.
fn generate_derive_arbitrary_bounds(f: &mut Formatter) {
    f.line(
        "#[cfg_attr(any(test, feature = \"fuzz\"), arbitrary(bound = \"R: crate::fuzz::RegistersArbitrary\"))]",
        None,
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
