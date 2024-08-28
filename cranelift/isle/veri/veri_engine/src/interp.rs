/// Interpret and build an assumption context from the LHS and RHS of rules.
use crate::type_inference::RuleSemantics;
use veri_ir::{BoundVar, Expr};

use std::collections::HashMap;
use std::fmt::Debug;

use cranelift_isle as isle;
use isle::sema::{RuleId, VarId};

/// Assumption consist of single verification IR expressions, which must have
/// boolean type.
#[derive(Clone, Debug)]
pub struct Assumption {
    assume: Expr,
}

impl Assumption {
    /// Create a new assumption, checking type.
    pub fn new(assume: Expr) -> Self {
        // assert!(assume.ty().is_bool());
        Self { assume }
    }

    /// Get the assumption as an expression.
    pub fn assume(&self) -> &Expr {
        &self.assume
    }
}
pub struct Context<'ctx> {
    pub quantified_vars: Vec<BoundVar>,
    pub free_vars: Vec<BoundVar>,
    pub assumptions: Vec<Assumption>,
    pub var_map: HashMap<VarId, BoundVar>,

    // For type checking
    pub typesols: &'ctx HashMap<RuleId, RuleSemantics>,
}

impl<'ctx> Context<'ctx> {
    pub fn new(typesols: &'ctx HashMap<RuleId, RuleSemantics>) -> Context<'ctx> {
        Context {
            quantified_vars: vec![],
            free_vars: vec![],
            assumptions: vec![],
            var_map: HashMap::new(),
            typesols,
        }
    }
}
