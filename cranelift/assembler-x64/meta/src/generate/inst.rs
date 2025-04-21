use super::{fmtln, generate_derive, generate_derive_arbitrary_bounds, Formatter};
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

        fmtln!(f, "/// `{self}`");
        generate_derive(f);
        if self.requires_generic() {
            generate_derive_arbitrary_bounds(f);
        }
        f.add_block(&format!("pub struct {struct_name} {where_clause}"), |f| {
            for k in &self.format.operands {
                let loc = k.location;
                let ty = k.generate_type();
                fmtln!(f, "pub {loc}: {ty},");
            }
        });
    }

    fn requires_generic(&self) -> bool {
        self.format.uses_register()
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
            self.generate_operands_function(f);
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
                .map(|o| format!("{}: {}", o.location, o.generate_type())),
        );
        let args = comma_join(self.format.operands.iter().map(|o| o.location.to_string()));

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
        f.add_block(&format!("pub fn encode(&self, buf: &mut impl CodeSink, {off}: &impl KnownOffsetTable)"), |f| {
            // Emit trap.
            if let Some(op) = self.format.uses_memory() {
                use dsl::OperandKind::*;
                f.comment("Emit trap.");
                match op.kind() {
                    Mem(_) => {
                        f.add_block(&format!("if let Some(trap_code) = self.{op}.trap_code()"), |f| {
                            fmtln!(f, "buf.add_trap(trap_code);");
                        });
                    }
                    RegMem(_) => {
                        let ty = match op.bits() {
                            128 => "XmmMem",
                            _ => "GprMem",
                        };
                        f.add_block(&format!("if let {ty}::Mem({op}) = &self.{op}"), |f| {
                            f.add_block(&format!("if let Some(trap_code) = {op}.trap_code()"), |f| {
                                fmtln!(f, "buf.add_trap(trap_code);");
                            });
                        });
                    }
                    _ => unreachable!(),
                }
            }

            match &self.encoding {
                dsl::Encoding::Rex(rex) => self.format.generate_rex_encoding(f, rex),
                dsl::Encoding::Vex(_) => todo!(),
            }
        });
    }

    /// `fn operands(&mut self, ...) { ... }`
    fn generate_operands_function(&self, f: &mut Formatter) {
        use dsl::Location::*;
        use dsl::Mutability::*;

        f.add_block(&format!("pub fn operands<'a>(&'a mut self) -> Vec<Operand<'a, R>>"), |f| {
            let mut names = vec![];
            for o in &self.format.operands {
                let mutability = match o.mutability {
                    Read => "read",
                    ReadWrite => "read_write",
                };
                let loc = &o.location;

                let constructor = match loc {
                    al | ax | eax | rax | cl => {
                        format!("from_{mutability}_fixed_gpr(&mut self.{loc})")
                    }
                    r8 | r16 | r32 | r64 => {
                        format!("from_{mutability}_gpr(&mut self.{loc})")
                    }
                    xmm => format!("from_{mutability}_xmm(&mut self.{loc})"),
                    rm8 | rm16 | rm32 | rm64 => {
                        format!("from_{mutability}_gpr_mem(&mut self.{loc})")
                    }
                    rm128 => {
                        format!("from_{mutability}_xmm_mem(&mut self.{loc})")
                    }
                    m8 | m16 | m32 | m64 => {
                        format!("from_amode(&mut self.{loc})")
                    }
                    imm8 | imm16 | imm32 => {
                        let bits = loc.bits();
                        let ty = if o.extension.is_sign_extended() {
                            format!("Simm{bits}")
                        } else {
                            format!("Imm{bits}")
                        };
                        format!("{ty}(&mut self.{loc})")
                    }
                };
                fmtln!(f, "let {loc} = Operand::{constructor};");
                names.push(loc.to_string());
            }
            fmtln!(f, "vec![{}]", comma_join(names.iter()));
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
                // Fix up the mnemonic for locked instructions: we want to print
                // "lock <inst>", not "lock_<inst>".
                let inst_name = if self.mnemonic.starts_with("lock_") {
                    &format!("lock {}", &self.mnemonic[5..])
                } else {
                    &self.mnemonic
                };
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
}

fn comma_join<S: Into<String>>(items: impl Iterator<Item = S>) -> String {
    items.map(Into::into).collect::<Vec<_>>().join(", ")
}
