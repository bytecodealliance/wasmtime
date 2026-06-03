//! Construction of VeriISLE specifications from ASLp semantics.

use std::collections::{HashMap, HashSet};
use std::vec;

use anyhow::{Result, bail};
use itertools::Itertools;

use cranelift_codegen::isa::aarch64::inst::Inst;
use cranelift_isle::ast::{self, Def, Modifies, Spec, SpecExpr};
use cranelift_isle::lexer::Pos;
use cranelift_isle_veri_aslp::client::Client;

use crate::bits::Bits;
use crate::constraints::{Binding, Scope, Target, Translator};
use crate::spec::spec_field;
use crate::{
    aarch64,
    spec::{Conditions, spec_all, spec_ident, spec_idents, spec_var, spec_with, substitute},
};

pub struct SpecConfig {
    pub term: String,
    pub args: Vec<String>,
    pub cases: Cases,
}

pub enum Cases {
    Instruction(InstConfig),
    Cases(Vec<Case>),
    Match(Match),
}

pub struct Case {
    pub conds: Vec<SpecExpr>,
    pub cases: Cases,
}

pub struct Match {
    pub on: SpecExpr,
    pub arms: Vec<Arm>,
}

pub struct Arm {
    pub variant: String,
    pub args: Vec<String>,
    pub body: Cases,
}

#[derive(Clone)]
pub enum Expectation {
    Require,
    Allow,
}

#[derive(Clone)]
pub struct Mapping {
    expr: SpecExpr,
    expect: Expectation,
    modifies: Vec<String>,
}

impl Mapping {
    pub fn new(expr: SpecExpr, expect: Expectation) -> Self {
        Self {
            expr,
            expect,
            modifies: Vec::new(),
        }
    }

    pub fn require(expr: SpecExpr) -> Self {
        Self::new(expr, Expectation::Require)
    }

    pub fn allow(expr: SpecExpr) -> Self {
        Self::new(expr, Expectation::Allow)
    }
}

#[derive(Clone)]
pub struct MappingBuilder(Mapping);

impl MappingBuilder {
    pub fn new(expr: SpecExpr) -> Self {
        Self(Mapping::new(expr, Expectation::Require))
    }

    pub fn var(name: &str) -> Self {
        Self::new(spec_var(name.to_string()))
    }

    pub fn state(name: &str) -> Self {
        Self::new(spec_var(name.to_string())).modifies(name)
    }

    pub fn field(mut self, field: &str) -> Self {
        self.0.expr = spec_field(field.to_string(), self.0.expr);
        self
    }

    pub fn allow(mut self) -> Self {
        self.0.expect = Expectation::Allow;
        self
    }

    pub fn modifies(mut self, state: &str) -> Self {
        self.0.modifies.push(state.to_string());
        self
    }

    pub fn build(self) -> Mapping {
        self.0
    }
}

#[derive(Clone, Default)]
pub struct Mappings {
    pub reads: HashMap<Target, Mapping>,
    pub writes: HashMap<Target, Mapping>,
}

impl Mappings {
    fn required_reads(&self) -> HashSet<Target> {
        Self::required_targets(&self.reads)
    }

    fn required_writes(&self) -> HashSet<Target> {
        Self::required_targets(&self.writes)
    }

    fn required_targets(target_mapping: &HashMap<Target, Mapping>) -> HashSet<Target> {
        target_mapping
            .iter()
            .filter_map(|(target, mapping)| match mapping.expect {
                Expectation::Require => Some(target.clone()),
                Expectation::Allow => None,
            })
            .collect()
    }
}

pub enum Opcodes {
    Instruction(Inst),
    Template(Bits),
}

impl Opcodes {
    pub fn bits(&self) -> Bits {
        match self {
            Opcodes::Instruction(inst) => {
                let opcode = aarch64::opcode(inst);
                Bits::from_u32(opcode)
            }
            Opcodes::Template(bits) => bits.clone(),
        }
    }
}

pub struct InstConfig {
    pub opcodes: Opcodes,
    pub scope: Scope,
    pub mappings: Mappings,
}

pub struct Builder<'a> {
    cfg: SpecConfig,
    client: &'a Client<'a>,
}

impl<'a> Builder<'a> {
    pub fn new(cfg: SpecConfig, client: &'a Client<'a>) -> Self {
        Self { cfg, client }
    }

    pub fn build(&self) -> Result<Def> {
        let spec = self.spec()?;
        let def = Def::Spec(spec);
        Ok(def)
    }

    fn spec(&self) -> Result<Spec> {
        let cond = self.cases(&self.cfg.cases)?;
        let modifies = spec_idents(&cond.modifies.iter().sorted().cloned().collect::<Vec<_>>());
        let spec = Spec {
            term: spec_ident(self.cfg.term.clone()),
            args: spec_idents(&self.cfg.args),
            requires: cond.requires,
            provides: cond.provides,
            matches: Vec::new(),
            modifies: modifies
                .into_iter()
                .map(|state| Modifies { state, cond: None })
                .collect(),
            pos: Pos::default(),
        };
        Ok(spec)
    }

    fn cases(&self, cases: &Cases) -> Result<Conditions> {
        match cases {
            Cases::Instruction(case) => self.case(case),
            Cases::Cases(cases) => {
                let conds = cases
                    .iter()
                    .map(|case| {
                        let mut cond = self.cases(&case.cases)?;
                        cond.requires.extend(case.conds.clone());
                        Ok(cond)
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok(Conditions::merge(conds))
            }
            Cases::Match(m) => {
                let mut require_arms = Vec::new();
                let mut arms = Vec::new();
                let mut modifies = HashSet::new();
                for arm in &m.arms {
                    // Build conditions for the arm body.
                    let cond = self.cases(&arm.body)?;

                    // Provides form the body of the arm.
                    arms.push(ast::Arm {
                        variant: spec_ident(arm.variant.clone()),
                        args: spec_idents(&arm.args),
                        body: spec_all(cond.provides),
                        pos: Pos::default(),
                    });

                    // This arm requires a match on the variant, as well as
                    // requirements from the body.
                    require_arms.push(ast::Arm {
                        variant: spec_ident(arm.variant.clone()),
                        args: spec_idents(&arm.args),
                        body: spec_all(cond.requires),
                        pos: Pos::default(),
                    });

                    // Merge modifies.
                    modifies.extend(cond.modifies);
                }

                Ok(Conditions {
                    requires: vec![SpecExpr::Match {
                        x: Box::new(m.on.clone()),
                        arms: require_arms,
                        pos: Pos::default(),
                    }],
                    provides: vec![SpecExpr::Match {
                        x: Box::new(m.on.clone()),
                        arms,
                        pos: Pos::default(),
                    }],
                    modifies,
                })
            }
        }
    }

    fn case(&self, case: &InstConfig) -> Result<Conditions> {
        // Semantics.
        let opcode_bits = case.opcodes.bits();
        let block = self.client.opcode(opcode_bits.into())?;

        // Translation.
        let mut translator = Translator::new(case.scope.clone(), "t".to_string());
        translator.translate(&block)?;

        let global = translator.global();

        // Reads mapping.
        let mut substitutions = HashMap::new();
        let mut modifies = HashSet::new();
        let reads = global.reads();
        let init = global.init();
        for target in reads.iter().sorted() {
            // Expect mapping for the read.
            let Some(mapping) = case.mappings.reads.get(target) else {
                bail!("read of {target} is unmapped");
            };

            // Lookup variable holding the initial read value.
            let v = &init[target];

            // Substitute variable for mapped expression.
            substitutions.insert(v.clone(), mapping.expr.clone());

            // Read operations should not modify state.
            if !mapping.modifies.is_empty() {
                bail!("read of {target} should not modify state");
            }
        }

        if let Some(target) = case.mappings.required_reads().difference(reads).next() {
            bail!("{target} should have been read");
        }

        // Writes mapping.
        let writes = global.writes();
        let bindings = global.bindings();
        for target in writes.iter().sorted() {
            // Expect mapping for the write.
            let Some(mapping) = case.mappings.writes.get(target) else {
                bail!("write to {target} is unmapped");
            };

            // Lookup bound variable.
            let Some(Binding::Var(v)) = bindings.get(target) else {
                bail!("{target} not bound to variable");
            };

            // Substitute variable for mapped expression.
            substitutions.insert(v.clone(), mapping.expr.clone());

            // Update modifies list.
            modifies.extend(mapping.modifies.clone());
        }

        if let Some(target) = case.mappings.required_writes().difference(writes).next() {
            bail!("{target} should have been written");
        }

        // Finalize provided constraints.
        let mut provides = Vec::new();
        for constraint in global.constraints() {
            provides.push(substitute(constraint.clone(), &substitutions)?);
        }

        // Determine remaining temporaries and encapsulate in a scope.
        let temporaries: Vec<_> = global
            .vars()
            .iter()
            .filter(|v| !substitutions.contains_key(*v))
            .sorted()
            .cloned()
            .collect();
        if !temporaries.is_empty() {
            let with_scope = spec_with(spec_idents(&temporaries), spec_all(provides));
            provides = vec![with_scope];
        }

        // Conditions.
        Ok(Conditions {
            requires: Vec::new(),
            provides,
            modifies,
        })
    }
}
