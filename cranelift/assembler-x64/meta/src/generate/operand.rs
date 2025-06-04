use crate::dsl;

impl dsl::Operand {
    #[must_use]
    pub fn generate_type(&self) -> String {
        use dsl::Location::*;
        let mut_ = self.mutability.generate_camel_case();
        match self.location {
            imm8 | imm16 | imm32 | imm64 => {
                let bits = self.location.bits();
                if self.extension.is_sign_extended() {
                    format!("Simm{bits}")
                } else {
                    format!("Imm{bits}")
                }
            }
            al | ax | eax | rax | cl | dx | edx | rdx => {
                let enc = match self.location {
                    al | ax | eax | rax => "{ gpr::enc::RAX }",
                    cl => "{ gpr::enc::RCX }",
                    dx | edx | rdx => "{ gpr::enc::RDX }",
                    _ => unreachable!(),
                };
                format!("Fixed<R::{mut_}Gpr, {enc}>")
            }
            r8 | r16 | r32 | r32a | r32b | r64 | r64a | r64b => format!("Gpr<R::{mut_}Gpr>"),
            rm8 | rm16 | rm32 | rm64 => format!("GprMem<R::{mut_}Gpr, R::ReadGpr>"),
            xmm1 | xmm2 | xmm3 => {
                format!("Xmm<R::{mut_}Xmm>")
            }
            xmm_m16 | xmm_m32 | xmm_m64 | xmm_m128 => {
                format!("XmmMem<R::{mut_}Xmm, R::ReadGpr>")
            }
            m8 | m16 | m32 | m64 => format!("Amode<R::ReadGpr>"),
        }
    }
}

impl dsl::Location {
    /// `self.<operand>.to_string(...)`
    #[must_use]
    pub fn generate_to_string(&self, extension: dsl::Extension) -> String {
        use dsl::Location::*;
        match self {
            imm8 | imm16 | imm32 | imm64 => {
                if extension.is_sign_extended() {
                    let variant = extension.generate_variant();
                    format!("self.{self}.to_string({variant})")
                } else {
                    format!("self.{self}.to_string()")
                }
            }
            al | ax | eax | rax | cl | dx | edx | rdx | r8 | r16 | r32 | r32a | r32b | r64
            | r64a | r64b | rm8 | rm16 | rm32 | rm64 => match self.generate_size() {
                Some(size) => format!("self.{self}.to_string({size})"),
                None => unreachable!(),
            },
            xmm_m16 | xmm_m32 | xmm_m64 | xmm1 | xmm2 | xmm3 | xmm_m128 | m8 | m16 | m32 | m64 => {
                format!("self.{self}.to_string()")
            }
        }
    }

    /// `Size::<operand size>`
    #[must_use]
    fn generate_size(&self) -> Option<&str> {
        use dsl::Location::*;
        match self {
            imm8 | imm16 | imm32 | imm64 => None,
            al | cl | r8 | rm8 => Some("Size::Byte"),
            ax | dx | r16 | rm16 => Some("Size::Word"),
            eax | edx | r32 | r32a | r32b | rm32 => Some("Size::Doubleword"),
            rax | rdx | r64 | r64a | r64b | rm64 => Some("Size::Quadword"),
            m8 | m16 | m32 | m64 => {
                panic!("no need to generate a size for memory-only access")
            }
            xmm1 | xmm2 | xmm3 | xmm_m16 | xmm_m32 | xmm_m64 | xmm_m128 => {
                panic!("no need to generate a size for XMM-sized access")
            }
        }
    }
}

impl dsl::Mutability {
    #[must_use]
    pub fn generate_camel_case(&self) -> &str {
        match self {
            dsl::Mutability::Read => "Read",
            dsl::Mutability::ReadWrite => "ReadWrite",
            dsl::Mutability::Write => "Write",
        }
    }

    #[must_use]
    pub fn generate_snake_case(&self) -> &str {
        match self {
            dsl::Mutability::Read => "read",
            dsl::Mutability::ReadWrite => "read_write",
            dsl::Mutability::Write => "write",
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
