use super::{fmtln, generate_derive, generate_derive_arbitrary_bounds, Formatter};
use crate::dsl;
use crate::dsl::OperandKind;

impl dsl::Inst {
    /// `struct <inst> { <op>: Reg, <op>: Reg, ... }`
    pub fn generate_struct(&self, f: &mut Formatter) {
        let struct_name = self.struct_name_with_generic();
        let where_clause = if self.requires_generic() {
            "where R: Registers"
        } else {
            ""
        };

        fmtln!(f, "/// `{self}`");
        generate_derive(f);
        if self.requires_generic() {
            generate_derive_arbitrary_bounds(f);
        }
        f.add_block(&format!("pub struct {struct_name} {where_clause}"), |f| {
            for k in &self.format.operands {
                if let Some(ty) = k.generate_type() {
                    let loc = k.location;
                    fmtln!(f, "pub {loc}: {ty},");
                }
            }
        });
    }

    fn requires_generic(&self) -> bool {
        self.format.uses_variable_register()
    }

    /// `<struct_name><R>`
    pub(crate) fn struct_name_with_generic(&self) -> String {
        let struct_name = self.name();
        if self.requires_generic() {
            format!("{struct_name}<R>")
        } else {
            struct_name
        }
    }

    /// `impl...`
    fn generate_impl_block_start(&self) -> &str {
        if self.requires_generic() {
            "impl<R: Registers>"
        } else {
            "impl"
        }
    }

    /// `impl <inst> { ... }`
    pub fn generate_struct_impl(&self, f: &mut Formatter) {
        let impl_block = self.generate_impl_block_start();
        let struct_name = self.struct_name_with_generic();
        f.add_block(&format!("{impl_block} {struct_name}"), |f| {
            self.generate_new_function(f);
            f.empty_line();
            self.generate_encode_function(f);
            f.empty_line();
            self.generate_visit_function(f);
            f.empty_line();
            self.generate_features_function(f);
        });
    }

    // `fn new(<params>) -> Self { ... }`
    pub fn generate_new_function(&self, f: &mut Formatter) {
        let params = comma_join(
            self.format
                .operands
                .iter()
                .filter_map(|o| o.generate_type().map(|t| format!("{}: {}", o.location, t))),
        );
        let args = comma_join(
            self.format
                .operands
                .iter()
                .filter(|o| !matches!(o.location.kind(), dsl::OperandKind::FixedReg(_)))
                .map(|o| o.location.to_string()),
        );

        fmtln!(f, "#[must_use]");
        f.add_block(&format!("pub fn new({params}) -> Self"), |f| {
            fmtln!(f, "Self {{ {args} }}",);
        });
    }

    /// `fn encode(&self, ...) { ... }`
    fn generate_encode_function(&self, f: &mut Formatter) {
        let off = if self.format.uses_memory().is_some() {
            "off"
        } else {
            "_"
        };
        f.add_block(
            &format!("pub fn encode(&self, buf: &mut impl CodeSink, {off}: &impl KnownOffsetTable)"),
            |f| {
                // Emit trap.
                if let Some(op) = self.format.uses_memory() {
                    f.empty_line();
                    f.comment("Emit trap.");
                    match op {
                        crate::dsl::Location::rm128 => {
                            fmtln!(f, "if let XmmMem::Mem({op}) = &self.{op} {{");
                        }
                        _ => {
                            fmtln!(f, "if let GprMem::Mem({op}) = &self.{op} {{");
                        }
                    }
                    f.indent(|f| {
                        f.add_block(&format!("if let Some(trap_code) = {op}.trap_code()"), |f| {
                            fmtln!(f, "buf.add_trap(trap_code);");
                        });
                    });
                    fmtln!(f, "}}");
                }

                match &self.encoding {
                    dsl::Encoding::Rex(rex) => self.format.generate_rex_encoding(f, rex),
                    dsl::Encoding::Vex(_) => todo!(),
                }
            },
        );
    }

    /// `fn visit(&self, ...) { ... }`
    fn generate_visit_function(&self, f: &mut Formatter) {
        use dsl::OperandKind::*;
        let extra_generic_bound = if self.requires_generic() {
            ""
        } else {
            "<R: Registers>"
        };
        f.add_block(&format!("pub fn visit{extra_generic_bound}(&mut self, visitor: &mut impl RegisterVisitor<R>)"), |f| {
            for o in &self.format.operands {
                match o.location.kind() {
                    Imm(_) => {
                        // Immediates do not need register allocation.
                    }
                    FixedReg(_) => {
                        let call = o.mutability.generate_regalloc_call();
                        let ty = o.mutability.generate_type();
                        let Some(fixed) = o.location.generate_fixed_reg() else {
                            unreachable!()
                        };
                        fmtln!(f, "visitor.fixed_{call}(&R::{ty}Gpr::new({fixed}));");
                    }
                    Reg(reg) => {
                        match reg.bits() {
                            128 => {
                                let call = o.mutability.generate_xmm_regalloc_call();
                                fmtln!(f, "visitor.{call}(self.{reg}.as_mut());");
                            }
                            _ => {
                                let call = o.mutability.generate_regalloc_call();
                                fmtln!(f, "visitor.{call}(self.{reg}.as_mut());");
                            }
                        };
                    }
                    RegMem(rm) => {
                        match rm.bits() {
                            128 => {
                                let call = o.mutability.generate_xmm_regalloc_call();
                                f.add_block(&format!("match &mut self.{rm}"), |f| {
                                    fmtln!(f, "XmmMem::Xmm(r) => visitor.{call}(r),");
                                    fmtln!(
                                        f,
                                        "XmmMem::Mem(m) => m.registers_mut().iter_mut().for_each(|r| visitor.read(r)),"
                                    );
                                });
                            }
                            _ => {
                                let call = o.mutability.generate_regalloc_call();
                                f.add_block(&format!("match &mut self.{rm}"), |f| {
                                    fmtln!(f, "GprMem::Gpr(r) => visitor.{call}(r),");
                                    fmtln!(
                                        f,
                                        "GprMem::Mem(m) => m.registers_mut().iter_mut().for_each(|r| visitor.read(r)),"
                                    );
                                });
                            }
                        };
                    }
                }
            }
        });
    }

    /// `fn features(&self) -> Vec<Flag> { ... }`
    fn generate_features_function(&self, f: &mut Formatter) {
        fmtln!(f, "#[must_use]");
        f.add_block("pub fn features(&self) -> Vec<Feature>", |f| {
            let flags = self
                .features
                .iter()
                .map(|f| format!("Feature::{f}"))
                .collect::<Vec<_>>();
            fmtln!(f, "vec![{}]", flags.join(", "));
        });
    }

    /// `impl Display for <inst> { ... }`
    pub fn generate_display_impl(&self, f: &mut Formatter) {
        let impl_block = self.generate_impl_block_start();
        let struct_name = self.struct_name_with_generic();
        f.add_block(&format!("{impl_block} std::fmt::Display for {struct_name}"), |f| {
            f.add_block("fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result", |f| {
                for op in &self.format.operands {
                    let location = op.location;
                    let to_string = location.generate_to_string(op.extension);
                    fmtln!(f, "let {location} = {to_string};");
                }
                let inst_name = &self.mnemonic;
                let ordered_ops = self.format.generate_att_style_operands();
                fmtln!(f, "write!(f, \"{inst_name} {ordered_ops}\")");
            });
        });
    }

    /// `impl From<struct> for Inst { ... }`
    pub fn generate_from_impl(&self, f: &mut Formatter) {
        let struct_name_r = self.struct_name_with_generic();
        let variant_name = self.name();
        f.add_block(&format!("impl<R: Registers> From<{struct_name_r}> for Inst<R>"), |f| {
            f.add_block(&format!("fn from(inst: {struct_name_r}) -> Self"), |f| {
                fmtln!(f, "Self::{variant_name}(inst)");
            });
        });
    }

    /// `fn x64_<inst>(&mut self, <params>) -> Inst<R> { ... }`
    ///
    /// # Panics
    ///
    /// This function panics if the instruction has no operands.
    pub fn generate_isle_macro(&self, f: &mut Formatter) {
        let struct_name = self.name();
        let params = self
            .format
            .operands
            .iter()
            .filter(|o| o.mutability.is_read())
            // FIXME(#10238) don't filter out fixed regs here
            .filter(|o| !matches!(o.location.kind(), OperandKind::FixedReg(_)))
            .collect::<Vec<_>>();
        let results = self
            .format
            .operands
            .iter()
            .filter(|o| o.mutability.is_write())
            .collect::<Vec<_>>();
        let rust_params = params
            .iter()
            .map(|o| format!("{}: {}", o.location, o.rust_param_raw()))
            .collect::<Vec<_>>()
            .join(", ");
        f.add_block(
            &format!("fn x64_{struct_name}_raw(&mut self, {rust_params}) -> AssemblerOutputs"),
            |f| {
                for o in params.iter() {
                    let l = o.location;
                    match o.rust_convert_isle_to_assembler() {
                        Some(cvt) => fmtln!(f, "let {l} = {cvt}({l});"),
                        None => fmtln!(f, "let {l} = {l}.clone();"),
                    }
                }
                let args = params
                    .iter()
                    .map(|o| format!("{}.clone()", o.location))
                    .collect::<Vec<_>>();
                let args = args.join(", ");
                fmtln!(f, "let inst = cranelift_assembler_x64::inst::{struct_name}::new({args}).into();");
                fmtln!(f, "let inst = MInst::External {{ inst }};");

                use dsl::Mutability::*;
                match results.as_slice() {
                    [] => fmtln!(f, "SideEffectNoResult::Inst(inst)"),
                    [one] => match one.mutability {
                        Read => unreachable!(),
                        ReadWrite => match one.location.kind() {
                            OperandKind::Imm(_) => unreachable!(),
                            // FIXME(#10238)
                            OperandKind::FixedReg(_) => fmtln!(f, "todo!()"),
                            // One read/write register output? Output the instruction
                            // and that register.
                            OperandKind::Reg(r) => match r.bits() {
                                128 => {
                                    fmtln!(f, "let xmm = {}.as_ref().write.to_reg();", results[0].location);
                                    fmtln!(f, "AssemblerOutputs::RetXmm {{ inst, xmm }}")
                                }
                                _ => {
                                    fmtln!(f, "let gpr = {}.as_ref().write.to_reg();", results[0].location);
                                    fmtln!(f, "AssemblerOutputs::RetGpr {{ inst, gpr }}")
                                }
                            },
                            // One read/write regmem output? We need to output
                            // everything and it'll internally disambiguate which was
                            // emitted (e.g. the mem variant or the register variant).
                            OperandKind::RegMem(_) => {
                                assert_eq!(results.len(), 1);
                                let l = results[0].location;
                                f.add_block(&format!("match {l}"), |f| match l.bits() {
                                    128 => {
                                        f.add_block("asm::XmmMem::Xmm(reg) => ", |f| {
                                            fmtln!(f, "let xmm = reg.write.to_reg();");
                                            fmtln!(f, "AssemblerOutputs::RetXmm {{ inst, xmm }} ");
                                        });
                                        f.add_block("asm::XmmMem::Mem(_) => ", |f| {
                                            fmtln!(f, "AssemblerOutputs::SideEffect {{ inst }} ");
                                        });
                                    }
                                    _ => {
                                        f.add_block("asm::GprMem::Gpr(reg) => ", |f| {
                                            fmtln!(f, "let gpr = reg.write.to_reg();");
                                            fmtln!(f, "AssemblerOutputs::RetGpr {{ inst, gpr }} ")
                                        });
                                        f.add_block("asm::GprMem::Mem(_) => ", |f| {
                                            fmtln!(f, "AssemblerOutputs::SideEffect {{ inst }} ");
                                        });
                                    }
                                });
                            }
                        },
                    },
                    _ => panic!("instruction has more than one result"),
                }
            },
        );
    }

    /// Generate a "raw" constructor that simply constructs, but does not emit
    /// the assembly instruction:
    ///
    /// ```text
    /// (decl x64_<inst>_raw (<params>) AssemblerOutputs)
    /// (extern constructor x64_<inst>_raw x64_<inst>_raw)
    /// ```
    ///
    /// Using the "raw" constructor, we also generate "emitter" constructors
    /// (see [`IsleConstructor`]). E.g., instructions that write to a register
    /// will return the register:
    ///
    /// ```text
    /// (decl x64_<inst> (<params>) Gpr)
    /// (rule (x64_<inst> <params>) (emit_ret_gpr (x64_<inst>_raw <params>)))
    /// ```
    ///
    /// For instructions that write to memory, we also generate an "emitter"
    /// constructor with the `_mem` suffix:
    ///
    /// ```text
    /// (decl x64_<inst>_mem (<params>) SideEffectNoResult)
    /// (rule (x64_<inst>_mem <params>) (defer_side_effect (x64_<inst>_raw <params>)))
    /// ```
    ///
    /// # Panics
    ///
    /// This function panics if the instruction has no operands.
    pub fn generate_isle_definition(&self, f: &mut Formatter) {
        // First declare the "raw" constructor which is implemented in Rust
        // with `generate_isle_macro` above. This is an "extern" constructor
        // with relatively raw types. This is not intended to be used by
        // general lowering rules in ISLE.
        let struct_name = self.name();
        let raw_name = format!("x64_{struct_name}_raw");
        let params = self
            .format
            .operands
            .iter()
            .filter(|o| o.mutability.is_read())
            // FIXME(#10238) don't filter out fixed regs here
            .filter(|o| !matches!(o.location.kind(), OperandKind::FixedReg(_)))
            .collect::<Vec<_>>();
        let raw_param_tys = params
            .iter()
            .map(|o| o.isle_param_raw())
            .collect::<Vec<_>>()
            .join(" ");
        fmtln!(f, "(decl {raw_name} ({raw_param_tys}) AssemblerOutputs)");
        fmtln!(f, "(extern constructor {raw_name} {raw_name})");

        // Next, for each "emitter" ISLE constructor being generated, synthesize
        // a pure-ISLE constructor which delegates appropriately to the `*_raw`
        // constructor above.
        //
        // The main purpose of these constructors is to have faithful type
        // signatures for the SSA nature of VCode/ISLE, effectively translating
        // x64's type system to ISLE/VCode's type system.
        for ctor in self.format.isle_constructors() {
            let suffix = ctor.suffix();
            let rule_name = format!("x64_{struct_name}{suffix}");
            let result_ty = ctor.result_ty();
            let param_tys = params
                .iter()
                .map(|o| o.isle_param_for_ctor(ctor))
                .collect::<Vec<_>>()
                .join(" ");
            let param_names = params
                .iter()
                .map(|o| o.location.to_string())
                .collect::<Vec<_>>()
                .join(" ");
            let convert = ctor.conversion_constructor();

            fmtln!(f, "(decl {rule_name} ({param_tys}) {result_ty})");
            fmtln!(f, "(rule ({rule_name} {param_names}) ({convert} ({raw_name} {param_names})))");
        }
    }
}

fn comma_join<I: Into<String>>(items: impl Iterator<Item = I>) -> String {
    items.map(Into::into).collect::<Vec<_>>().join(", ")
}

/// Different kinds of ISLE constructors generated for a particular instruction.
///
/// One instruction may generate a single constructor or multiple constructors.
/// For example an instruction that writes its result to a register will
/// generate only a single constructor. An instruction where the destination
/// read/write operand is `GprMem` will generate two constructors though, one
/// for memory and one for in registers.
#[derive(Copy, Clone, Debug)]
pub enum IsleConstructor {
    /// This constructor only produces a side effect, meaning that the
    /// instruction does not produce results in registers. This may produce
    /// a result in memory, however.
    RetMemorySideEffect,

    /// This constructor produces a `Gpr` value, meaning that it will write the
    /// result to a `Gpr`.
    RetGpr,

    /// This constructor produces an `Xmm` value, meaning that it will write the
    /// result to an `Xmm`.
    RetXmm,
}

impl IsleConstructor {
    /// Returns the result type, in ISLE, that this constructor generates.
    pub fn result_ty(&self) -> &'static str {
        match self {
            IsleConstructor::RetMemorySideEffect => "SideEffectNoResult",
            IsleConstructor::RetGpr => "Gpr",
            IsleConstructor::RetXmm => "Xmm",
        }
    }

    /// Returns the constructor used to convert an `AssemblerOutput` into the
    /// type returned by [`Self::result_ty`].
    pub fn conversion_constructor(&self) -> &'static str {
        match self {
            IsleConstructor::RetMemorySideEffect => "defer_side_effect",
            IsleConstructor::RetGpr => "emit_ret_gpr",
            IsleConstructor::RetXmm => "emit_ret_xmm",
        }
    }

    /// Returns the suffix used in the ISLE constructor name.
    pub fn suffix(&self) -> &'static str {
        match self {
            IsleConstructor::RetMemorySideEffect => "_mem",
            IsleConstructor::RetGpr => "",
            IsleConstructor::RetXmm => "",
        }
    }
}
