use crate::dsl;

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
            Mem(_) => Some(format!("Amode<R::ReadGpr>")),
        }
    }
}

impl dsl::Location {
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
            xmm | rm128 | m8 | m16 | m32 | m64 => format!("self.{self}.to_string()"),
        }
    }

    /// `Size::<operand size>`
    #[must_use]
    fn generate_size(&self) -> Option<&str> {
        use dsl::Location::*;
        match self {
            al | ax | eax | rax | cl | imm8 | imm16 | imm32 => None,
            r8 | rm8 => Some("Size::Byte"),
            r16 | rm16 => Some("Size::Word"),
            r32 | rm32 => Some("Size::Doubleword"),
            r64 | rm64 => Some("Size::Quadword"),
            m8 | m16 | m32 | m64 => {
                panic!("no need to generate a size for memory-only access")
            }
            xmm | rm128 => {
                panic!("no need to generate a size for XMM-sized access")
            }
        }
    }

    /// `Gpr(regs::...)`
    #[must_use]
    pub fn generate_fixed_reg(&self) -> Option<&str> {
        use dsl::Location::*;
        match self {
            al | ax | eax | rax => Some("gpr::enc::RAX"),
            cl => Some("gpr::enc::RCX"),
            imm8 | imm16 | imm32 | r8 | r16 | r32 | r64 | xmm | rm8 | rm16 | rm32 | rm64 | rm128 | m8 | m16 | m32
            | m64 => None,
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
