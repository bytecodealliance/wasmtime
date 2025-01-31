use super::{fmtln, generate_derive, Formatter};
use crate::dsl;

impl dsl::Inst {
    /// `struct <inst> { <op>: Reg, <op>: Reg, ... }`
    pub fn generate_struct(&self, f: &mut Formatter) {
        let struct_name = self.struct_name_with_generic();
        let where_clause = if self.requires_generic() {
            "where R: Registers"
        } else {
            ""
        };

        f.line(format!("/// `{self}`"), None);
        generate_derive(f);
        fmtln!(f, "pub struct {struct_name} {where_clause} {{");
        f.indent(|f| {
            for k in &self.format.operands {
                if let Some(ty) = k.generate_type() {
                    let loc = k.location;
                    fmtln!(f, "pub {loc}: {ty},");
                }
            }
        });
        fmtln!(f, "}}");
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

    // `fn <inst>(<params>) -> Inst { ... }`
    pub fn generate_variant_constructor(&self, f: &mut Formatter) {
        let variant_name = self.name();
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
        fmtln!(f, "pub fn {variant_name}<R: Registers>({params}) -> Inst<R> {{");
        f.indent(|f| {
            fmtln!(f, "Inst::{variant_name}({variant_name} {{ {args} }})",);
        });
        fmtln!(f, "}}");
    }

    /// `impl <inst> { ... }`
    pub fn generate_struct_impl(&self, f: &mut Formatter) {
        let impl_block = self.generate_impl_block_start();
        let struct_name = self.struct_name_with_generic();
        fmtln!(f, "{impl_block} {struct_name} {{");

        f.indent_push();
        self.generate_encode_function(f);
        f.empty_line();
        self.generate_visit_function(f);
        f.empty_line();
        self.generate_features_function(f);
        f.indent_pop();
        fmtln!(f, "}}");
    }

    /// `fn encode(&self, ...) { ... }`
    fn generate_encode_function(&self, f: &mut Formatter) {
        let off = if self.format.uses_memory().is_some() {
            "off"
        } else {
            "_"
        };
        fmtln!(f, "pub fn encode(&self, buf: &mut impl CodeSink, {off}: &impl KnownOffsetTable) {{");
        f.indent_push();

        // Emit trap.
        if let Some(op) = self.format.uses_memory() {
            f.empty_line();
            f.comment("Emit trap.");
            fmtln!(f, "if let GprMem::Mem({op}) = &self.{op} {{");
            f.indent(|f| {
                fmtln!(f, "if let Some(trap_code) = {op}.trap_code() {{");
                f.indent(|f| {
                    fmtln!(f, "buf.add_trap(trap_code);");
                });
                fmtln!(f, "}}");
            });
            fmtln!(f, "}}");
        }

        match &self.encoding {
            dsl::Encoding::Rex(rex) => self.format.generate_rex_encoding(f, rex),
            dsl::Encoding::Vex(_) => todo!(),
        }

        f.indent_pop();
        fmtln!(f, "}}");
    }

    /// `fn visit(&self, ...) { ... }`
    fn generate_visit_function(&self, f: &mut Formatter) {
        use dsl::OperandKind::*;
        let extra_generic_bound = if self.requires_generic() {
            ""
        } else {
            "<R: Registers>"
        };
        fmtln!(f, "pub fn visit{extra_generic_bound}(&mut self, visitor: &mut impl RegisterVisitor<R>) {{");
        f.indent(|f| {
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
                        let call = o.mutability.generate_regalloc_call();
                        fmtln!(f, "visitor.{call}(self.{reg}.as_mut());");
                    }
                    RegMem(rm) => {
                        let call = o.mutability.generate_regalloc_call();
                        fmtln!(f, "match &mut self.{rm} {{");
                        f.indent(|f| {
                            fmtln!(f, "GprMem::Gpr(r) => visitor.{call}(r),");
                            fmtln!(
                                f,
                                "GprMem::Mem(m) => m.registers_mut().iter_mut().for_each(|r| visitor.read(r)),"
                            );
                        });
                        fmtln!(f, "}}");
                    }
                }
            }
        });
        fmtln!(f, "}}");
    }

    /// `fn features(&self) -> Vec<Flag> { ... }`
    fn generate_features_function(&self, f: &mut Formatter) {
        fmtln!(f, "#[must_use]");
        fmtln!(f, "pub fn features(&self) -> Vec<Feature> {{");
        f.indent(|f| {
            let flags = self
                .features
                .iter()
                .map(|f| format!("Feature::{f}"))
                .collect::<Vec<_>>();
            fmtln!(f, "vec![{}]", flags.join(", "));
        });
        fmtln!(f, "}}");
    }

    /// `impl Display for <inst> { ... }`
    pub fn generate_display_impl(&self, f: &mut Formatter) {
        let impl_block = self.generate_impl_block_start();
        let struct_name = self.struct_name_with_generic();
        fmtln!(f, "{impl_block} std::fmt::Display for {struct_name} {{");
        f.indent_push();
        fmtln!(f, "fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {{");

        f.indent_push();
        for op in &self.format.operands {
            let location = op.location;
            let to_string = location.generate_to_string(op.extension);
            fmtln!(f, "let {location} = {to_string};");
        }

        let inst_name = &self.mnemonic;
        let ordered_ops = self.format.generate_att_style_operands();
        fmtln!(f, "write!(f, \"{inst_name} {ordered_ops}\")");
        f.indent_pop();
        fmtln!(f, "}}");

        f.indent_pop();
        fmtln!(f, "}}");
    }

    /// `fn x64_<inst>(&mut self, <params>) -> Inst<R> { ... }`
    ///
    /// # Panics
    ///
    /// This function panics if the instruction has no operands.
    pub fn generate_isle_macro(&self, f: &mut Formatter, read_ty: &str, read_write_ty: &str) {
        use dsl::OperandKind::*;
        let struct_name = self.name();
        let operands = self
            .format
            .operands
            .iter()
            .filter_map(|o| Some((o.location, o.generate_mut_ty(read_ty, read_write_ty)?)))
            .collect::<Vec<_>>();
        let ret_ty = match self.format.operands.first().unwrap().location.kind() {
            Imm(_) => unreachable!(),
            Reg(_) | FixedReg(_) => format!("cranelift_assembler_x64::Gpr<{read_write_ty}>"),
            RegMem(_) => format!("cranelift_assembler_x64::GprMem<{read_write_ty}, {read_ty}>"),
        };
        let ret_val = match self.format.operands.first().unwrap().location.kind() {
            Imm(_) => unreachable!(),
            FixedReg(_) => "todo!()".to_string(),
            Reg(loc) | RegMem(loc) => format!("{loc}.clone()"),
        };
        let params = comma_join(
            operands
                .iter()
                .map(|(l, ty)| format!("{l}: &cranelift_assembler_x64::{ty}")),
        );
        let args = comma_join(operands.iter().map(|(l, _)| format!("{l}.clone()")));

        // TODO: parameterize CraneliftRegisters?
        fmtln!(f, "fn x64_{struct_name}(&mut self, {params}) -> {ret_ty} {{",);
        f.indent(|f| {
            fmtln!(f, "let inst = cranelift_assembler_x64::build::{struct_name}({args});");
            fmtln!(f, "self.lower_ctx.emit(MInst::External {{ inst }});");
            fmtln!(f, "{ret_val}");
        });
        fmtln!(f, "}}");
    }

    /// `(decl x64_<inst> (<params>) <return>)
    ///  (extern constructor x64_<inst> x64_<inst>)`
    ///
    /// # Panics
    ///
    /// This function panics if the instruction has no operands.
    pub fn generate_isle_definition(&self, f: &mut Formatter) {
        use dsl::OperandKind::*;

        let struct_name = self.name();
        let rule_name = format!("x64_{struct_name}");
        let params = self
            .format
            .operands
            .iter()
            .filter_map(|o| match o.location.kind() {
                FixedReg(_) => None,
                Imm(loc) => Some(format!("AssemblerImm{}", loc.bits())),
                Reg(_) => Some(format!("Assembler{}Gpr", o.mutability.generate_type())),
                RegMem(_) => Some(format!("Assembler{}GprMem", o.mutability.generate_type())),
            })
            .collect::<Vec<_>>()
            .join(" ");
        let ret = match self.format.operands.first().unwrap().location.kind() {
            Imm(_) => unreachable!(),
            FixedReg(_) | Reg(_) => "AssemblerReadWriteGpr",
            RegMem(_) => "AssemblerReadWriteGprMem",
        };

        f.line(format!("(decl {rule_name} ({params}) {ret})"), None);
        f.line(format!("(extern constructor {rule_name} {rule_name})"), None);
    }
}

fn comma_join<I: Into<String>>(items: impl Iterator<Item = I>) -> String {
    items.map(Into::into).collect::<Vec<_>>().join(", ")
}
