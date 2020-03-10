//! Templates are a low-level expression language which can be used to describe
//! interactions with runtime systems such as computing addresses of data
//! structures, GC barriers, or sandboxing sequences in a way that lets JITs
//! emit inline code sequences.

use crate::ir::immediates::{Imm64, Offset32};
use crate::ir::{ExternalName, Template, Type};
use crate::isa::TargetIsa;
use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::fmt;
use cranelift_entity::PrimaryMap;

/// A template is an expression which can be expanded into code or
/// interpreted directly. `Template` is an index into a template table, while
/// `TemplateData` is the table element type.
#[derive(Clone)]
pub enum TemplateData {
    /// The address of the VM context struct.
    VMContext,

    /// Load a value from memory.
    ///
    /// The effective address of the load is `base` plus `offset`. The memory must be accessible
    /// and naturally aligned to hold a value of the type.
    Load {
        /// The base pointer value.
        base: Template,

        /// Offset added to the base pointer before doing the load.
        offset: Offset32,

        /// Type of the loaded value.
        result_type: Type,

        /// Specifies whether the memory that this refers to is effectively readonly throughout
        /// any code where this template is in use, allowing for the elimination of redundant
        /// loads.
        readonly: bool,
    },

    /// Add an immediate constant to a value.
    IAddImm {
        /// The base value.
        base: Template,

        /// Offset to be added to the value.
        offset: Imm64,
    },

    /// The value of a symbol, which is a name which will be resolved to an
    /// actual value later (eg. by linking).
    ///
    /// For now, symbolic values always have pointer type, and represent
    /// addresses, however in the future they could be used to represent other
    /// things as well.
    Symbol {
        /// The symbolic name.
        name: ExternalName,

        /// Offset from the symbol. This can be used instead of IAddImm to represent folding an
        /// offset into a symbol.
        offset: Imm64,

        /// Will this symbol be defined nearby, such that it will always be a certain distance
        /// away, after linking? If so, references to it can avoid going through a GOT. Note that
        /// symbols meant to be preemptible cannot be colocated.
        colocated: bool,

        /// Does this symbol refer to a thread local storage value?
        tls: bool,
    },

    /// A function call.
    Call {
        /// The expression to call. May be a `Symbol` for a direct call, or
        /// other kinds of expression for indirect calls.
        callee: Template,

        /// Arguments to pass to the call.
        args: Box<[Template]>,

        /// The result type of the call.
        result_type: Type,
    },

    /// An "if-then-else".
    IfElse {
        /// The boolean cond.
        cond: Template,

        /// Code to execute if `cond` is true.
        then: Template,

        /// Code to execute if `cond` is false.
        else_: Template,

        /// When this is true, it means that the condition is expected to usually
        /// be true, so that `else_` arm is rarely evaluated at runtime, so JITs
        /// may choose to emit it out of line.
        else_is_cold: bool,
    },
}

impl TemplateData {
    /// Assume that `self` is an `TemplateData::Symbol` and return its name.
    pub fn symbol_name(&self) -> &ExternalName {
        match *self {
            Self::Symbol { ref name, .. } => name,
            _ => panic!("only symbols have names"),
        }
    }

    /// Return the type of this template.
    pub fn result_type(
        &self,
        isa: &dyn TargetIsa,
        templates: &PrimaryMap<Template, TemplateData>,
    ) -> Type {
        match *self {
            Self::VMContext { .. } | Self::Symbol { .. } => isa.pointer_type(),
            Self::Load { result_type, .. } | Self::Call { result_type, .. } => result_type,
            Self::IAddImm { base, .. } => templates[base].result_type(isa, templates),
            Self::IfElse {
                cond: _,
                then,
                else_,
                else_is_cold: _,
            } => {
                let type_ = templates[then].result_type(isa, templates);
                assert_eq!(
                    type_,
                    templates[else_].result_type(isa, templates),
                    "then and else arms should have the same type"
                );
                type_
            }
        }
    }
}

impl fmt::Display for TemplateData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::VMContext => write!(f, "vmctx"),
            Self::Load {
                base,
                offset,
                result_type,
                readonly,
            } => write!(
                f,
                "load.{} notrap aligned {}{}{}",
                result_type,
                if readonly { "readonly " } else { "" },
                base,
                offset
            ),
            Self::IAddImm { base, offset } => write!(f, "iadd_imm {}, {}", base, offset),
            Self::Symbol {
                ref name,
                offset,
                colocated,
                tls,
            } => {
                write!(
                    f,
                    "symbol {}{}{}",
                    if colocated { "colocated " } else { "" },
                    if tls { "tls " } else { "" },
                    name
                )?;
                let offset_val: i64 = offset.into();
                if offset_val > 0 {
                    write!(f, "+")?;
                }
                if offset_val != 0 {
                    write!(f, "{}", offset)?;
                }
                Ok(())
            }
            Self::Call {
                callee,
                ref args,
                result_type,
            } => write!(
                f,
                "call.{} {}({})",
                result_type,
                callee,
                args.iter()
                    .map(|arg| arg.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::IfElse {
                cond,
                then,
                else_,
                else_is_cold,
            } => write!(
                f,
                "if {} {{ {} }} else{} {{ {} }}",
                cond,
                then,
                if else_is_cold { " cold" } else { "" },
                else_
            ),
        }
    }
}
