use super::inst::IsleConstructor;
use crate::dsl::{self, Mutability, OperandKind};

impl dsl::Operand {
    #[must_use]
    pub fn generate_type(&self) -> Option<String> {
        use dsl::OperandKind::*;
        match self.location.kind() {
            FixedReg(_) => None,
            Imm(loc) => {
                let bits = loc.bits();
                if self.extension.is_sign_extended() {
                    Some(format!("Simm{bits}"))
                } else {
                    Some(format!("Imm{bits}"))
                }
            }
            Reg(r) => match r.bits() {
                128 => Some(format!("Xmm<R::{}Xmm>", self.mutability.generate_type())),
                _ => Some(format!("Gpr<R::{}Gpr>", self.mutability.generate_type())),
            },
            RegMem(rm) => match rm.bits() {
                128 => Some(format!("XmmMem<R::{}Xmm, R::ReadGpr>", self.mutability.generate_type())),
                _ => Some(format!("GprMem<R::{}Gpr, R::ReadGpr>", self.mutability.generate_type())),
            },
        }
    }

    #[must_use]
    pub fn generate_mut_ty(&self, read_ty: &str, read_write_ty: &str) -> Option<String> {
        use dsl::Mutability::*;
        use dsl::OperandKind::*;
        let pick_ty = match self.mutability {
            Read => read_ty,
            ReadWrite => read_write_ty,
        };
        match self.location.kind() {
            FixedReg(_) => None,
            Imm(loc) => {
                let bits = loc.bits();
                if self.extension.is_sign_extended() {
                    Some(format!("Simm{bits}"))
                } else {
                    Some(format!("Imm{bits}"))
                }
            }
            Reg(r) => match r.bits() {
                128 => Some(format!("Xmm<{pick_ty}>")),
                _ => Some(format!("Gpr<{pick_ty}>")),
            },
            RegMem(rm) => match rm.bits() {
                128 => Some(format!("XmmMem<{pick_ty}, Gpr>")),
                _ => Some(format!("GprMem<{pick_ty}, {read_ty}>")),
            },
        }
    }

    /// Returns the type of this operand in ISLE as part of the
    /// `IsleConstructorRaw` variants.
    pub fn isle_param_raw(&self) -> String {
        match self.location.kind() {
            OperandKind::Imm(loc) => {
                let bits = loc.bits();
                if self.extension.is_sign_extended() {
                    format!("AssemblerSimm{bits}")
                } else {
                    format!("AssemblerImm{bits}")
                }
            }
            OperandKind::Reg(r) => match r.bits() {
                128 => "Xmm".to_string(),
                _ => "Gpr".to_string(),
            },
            OperandKind::FixedReg(_) => "Gpr".to_string(),
            OperandKind::RegMem(rm) => match rm.bits() {
                128 => "XmmMem".to_string(),
                _ => "GprMem".to_string(),
            },
        }
    }

    /// Returns the parameter type used for the `IsleConstructor` variant
    /// provided.
    pub fn isle_param_for_ctor(&self, ctor: IsleConstructor) -> String {
        match self.location.kind() {
            // Writable `RegMem` operands are special here: in one constructor
            // it's operating on memory so the argument is `Amode` and in the
            // other constructor it's operating on registers so the argument is
            // a `Gpr`.
            OperandKind::RegMem(_) if self.mutability.is_write() => match ctor {
                IsleConstructor::RetMemorySideEffect => "Amode".to_string(),
                IsleConstructor::RetGpr => "Gpr".to_string(),
                IsleConstructor::RetXmm => "Xmm".to_string(),
            },

            // everything else is the same as the "raw" variant
            _ => self.isle_param_raw(),
        }
    }

    /// Returns the Rust type used for the `IsleConstructorRaw` variants.
    pub fn rust_param_raw(&self) -> String {
        match self.location.kind() {
            OperandKind::Imm(loc) => {
                let bits = loc.bits();
                if self.extension.is_sign_extended() {
                    format!("&cranelift_assembler_x64::Simm{bits}")
                } else {
                    format!("&cranelift_assembler_x64::Imm{bits}")
                }
            }
            OperandKind::RegMem(rm) => match rm.bits() {
                128 => "&XmmMem".to_string(),
                _ => "&GprMem".to_string(),
            },
            OperandKind::Reg(r) => match r.bits() {
                128 => "Xmm".to_string(),
                _ => "Gpr".to_string(),
            },
            OperandKind::FixedReg(_) => "Gpr".to_string(),
        }
    }

    /// Returns the conversion function, if any, when converting the ISLE type
    /// for this parameter to the assembler type for this parameter.
    /// Effectively converts `self.rust_param_raw()` to the assembler type.
    pub fn rust_convert_isle_to_assembler(&self) -> Option<&'static str> {
        match self.location.kind() {
            OperandKind::Reg(r) => match r.bits() {
                128 => Some(match self.mutability {
                    Mutability::Read => "cranelift_assembler_x64::Xmm::new",
                    Mutability::ReadWrite => "self.convert_xmm_to_assembler_read_write_xmm",
                }),
                _ => Some(match self.mutability {
                    Mutability::Read => "cranelift_assembler_x64::Gpr::new",
                    Mutability::ReadWrite => "self.convert_gpr_to_assembler_read_write_gpr",
                }),
            },
            OperandKind::RegMem(rm) => match rm.bits() {
                128 => Some(match self.mutability {
                    Mutability::Read => "self.convert_xmm_mem_to_assembler_read_xmm_mem",
                    Mutability::ReadWrite => "self.convert_xmm_mem_to_assembler_read_write_xmm_mem",
                }),
                _ => Some(match self.mutability {
                    Mutability::Read => "self.convert_gpr_mem_to_assembler_read_gpr_mem",
                    Mutability::ReadWrite => "self.convert_gpr_mem_to_assembler_read_write_gpr_mem",
                }),
            },
            OperandKind::FixedReg(_) | OperandKind::Imm(_) => None,
        }
    }
}

impl dsl::Location {
    /// `<operand type>`, if the operand has a type (i.e., not fixed registers).
    #[must_use]
    pub fn generate_type(&self, generic: Option<String>) -> Option<String> {
        use dsl::Location::*;
        let generic = match generic {
            Some(ty) => format!("<{ty}>"),
            None => String::new(),
        };
        match self {
            al | ax | eax | rax | cl => None,
            imm8 => Some("Imm8".into()),
            imm16 => Some("Imm16".into()),
            imm32 => Some("Imm32".into()),
            r8 | r16 | r32 | r64 => Some(format!("Gpr{generic}")),
            rm8 | rm16 | rm32 | rm64 => Some(format!("GprMem{generic}")),
            xmm => Some(format!("Xmm{generic}")),
            rm128 => Some(format!("XmmMem{generic}")),
        }
    }

    /// `self.<operand>.to_string(...)`
    #[must_use]
    pub fn generate_to_string(&self, extension: dsl::Extension) -> String {
        use dsl::Location::*;
        match self {
            al => "\"%al\"".into(),
            ax => "\"%ax\"".into(),
            eax => "\"%eax\"".into(),
            rax => "\"%rax\"".into(),
            cl => "\"%cl\"".into(),
            imm8 | imm16 | imm32 => {
                if extension.is_sign_extended() {
                    let variant = extension.generate_variant();
                    format!("self.{self}.to_string({variant})")
                } else {
                    format!("self.{self}.to_string()")
                }
            }
            r8 | r16 | r32 | r64 | rm8 | rm16 | rm32 | rm64 => match self.generate_size() {
                Some(size) => format!("self.{self}.to_string({size})"),
                None => unreachable!(),
            },
            xmm | rm128 => format!("self.{self}.to_string()"),
        }
    }

    /// `Size::<operand size>`
    #[must_use]
    pub fn generate_size(&self) -> Option<&str> {
        use dsl::Location::*;
        match self {
            al | ax | eax | rax | cl | imm8 | imm16 | imm32 => None,
            r8 | rm8 => Some("Size::Byte"),
            r16 | rm16 => Some("Size::Word"),
            r32 | rm32 => Some("Size::Doubleword"),
            r64 | rm64 => Some("Size::Quadword"),
            xmm | rm128 => panic!("no need to generate a size for XMM-sized access"),
        }
    }

    /// `Gpr(regs::...)`
    #[must_use]
    pub fn generate_fixed_reg(&self) -> Option<&str> {
        use dsl::Location::*;
        match self {
            al | ax | eax | rax => Some("reg::enc::RAX"),
            cl => Some("reg::enc::RCX"),
            imm8 | imm16 | imm32 | r8 | r16 | r32 | r64 | xmm | rm8 | rm16 | rm32 | rm64 | rm128 => None,
        }
    }
}

impl dsl::Mutability {
    #[must_use]
    pub fn generate_regalloc_call(&self) -> &str {
        match self {
            dsl::Mutability::Read => "read",
            dsl::Mutability::ReadWrite => "read_write",
        }
    }

    #[must_use]
    pub fn generate_type(&self) -> &str {
        match self {
            dsl::Mutability::Read => "Read",
            dsl::Mutability::ReadWrite => "ReadWrite",
        }
    }

    #[must_use]
    pub fn generate_xmm_regalloc_call(&self) -> &str {
        match self {
            dsl::Mutability::Read => "read_xmm",
            dsl::Mutability::ReadWrite => "read_write_xmm",
        }
    }
}

impl dsl::Extension {
    /// `Extension::...`
    #[must_use]
    pub fn generate_variant(&self) -> &str {
        use dsl::Extension::*;
        match self {
            None => "Extension::None",
            SignExtendWord => "Extension::SignExtendWord",
            SignExtendLong => "Extension::SignExtendLong",
            SignExtendQuad => "Extension::SignExtendQuad",
        }
    }
}
