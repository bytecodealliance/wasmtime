use crate::cdsl::ast::{Def, DefIndex, DefPool, Var, VarIndex, VarPool};
use crate::cdsl::typevar::{DerivedFunc, TypeSet, TypeVar};

use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;

#[derive(Debug, Hash, PartialEq, Eq)]
pub(crate) enum Constraint {
    /// Constraint specifying that a type var tv1 must be wider than or equal to type var tv2 at
    /// runtime. This requires that:
    /// 1) They have the same number of lanes
    /// 2) In a lane tv1 has at least as many bits as tv2.
    WiderOrEq(TypeVar, TypeVar),

    /// Constraint specifying that two derived type vars must have the same runtime type.
    Eq(TypeVar, TypeVar),

    /// Constraint specifying that a type var must belong to some typeset.
    InTypeset(TypeVar, TypeSet),
}

impl Constraint {
    fn translate_with<F: Fn(&TypeVar) -> TypeVar>(&self, func: F) -> Constraint {
        match self {
            Constraint::WiderOrEq(lhs, rhs) => {
                let lhs = func(&lhs);
                let rhs = func(&rhs);
                Constraint::WiderOrEq(lhs, rhs)
            }
            Constraint::Eq(lhs, rhs) => {
                let lhs = func(&lhs);
                let rhs = func(&rhs);
                Constraint::Eq(lhs, rhs)
            }
            Constraint::InTypeset(tv, ts) => {
                let tv = func(&tv);
                Constraint::InTypeset(tv, ts.clone())
            }
        }
    }

    /// Creates a new constraint by replacing type vars by their hashmap equivalent.
    fn translate_with_map(
        &self,
        original_to_own_typevar: &HashMap<&TypeVar, TypeVar>,
    ) -> Constraint {
        self.translate_with(|tv| substitute(original_to_own_typevar, tv))
    }

    /// Creates a new constraint by replacing type vars by their canonical equivalent.
    fn translate_with_env(&self, type_env: &TypeEnvironment) -> Constraint {
        self.translate_with(|tv| type_env.get_equivalent(tv))
    }

    fn is_trivial(&self) -> bool {
        match self {
            Constraint::WiderOrEq(lhs, rhs) => {
                // Trivially true.
                if lhs == rhs {
                    return true;
                }

                let ts1 = lhs.get_typeset();
                let ts2 = rhs.get_typeset();

                // Trivially true.
                if ts1.is_wider_or_equal(&ts2) {
                    return true;
                }

                // Trivially false.
                if ts1.is_narrower(&ts2) {
                    return true;
                }

                // Trivially false.
                if (&ts1.lanes & &ts2.lanes).is_empty() {
                    return true;
                }

                self.is_concrete()
            }
            Constraint::Eq(lhs, rhs) => lhs == rhs || self.is_concrete(),
            Constraint::InTypeset(_, _) => {
                // The way InTypeset are made, they would always be trivial if we were applying the
                // same logic as the Python code did, so ignore this.
                self.is_concrete()
            }
        }
    }

    /// Returns true iff all the referenced type vars are singletons.
    fn is_concrete(&self) -> bool {
        match self {
            Constraint::WiderOrEq(lhs, rhs) => {
                lhs.singleton_type().is_some() && rhs.singleton_type().is_some()
            }
            Constraint::Eq(lhs, rhs) => {
                lhs.singleton_type().is_some() && rhs.singleton_type().is_some()
            }
            Constraint::InTypeset(tv, _) => tv.singleton_type().is_some(),
        }
    }

    fn typevar_args(&self) -> Vec<&TypeVar> {
        match self {
            Constraint::WiderOrEq(lhs, rhs) => vec![lhs, rhs],
            Constraint::Eq(lhs, rhs) => vec![lhs, rhs],
            Constraint::InTypeset(tv, _) => vec![tv],
        }
    }
}

#[derive(Clone, Copy)]
enum TypeEnvRank {
    Singleton = 5,
    Input = 4,
    Intermediate = 3,
    Output = 2,
    Temp = 1,
    Internal = 0,
}

/// Class encapsulating the necessary bookkeeping for type inference.
pub(crate) struct TypeEnvironment {
    vars: HashSet<VarIndex>,
    ranks: HashMap<TypeVar, TypeEnvRank>,
    equivalency_map: HashMap<TypeVar, TypeVar>,
    pub constraints: Vec<Constraint>,
}

impl TypeEnvironment {
    fn new() -> Self {
        TypeEnvironment {
            vars: HashSet::new(),
            ranks: HashMap::new(),
            equivalency_map: HashMap::new(),
            constraints: Vec::new(),
        }
    }

    fn register(&mut self, var_index: VarIndex, var: &mut Var) {
        self.vars.insert(var_index);
        let rank = if var.is_input() {
            TypeEnvRank::Input
        } else if var.is_intermediate() {
            TypeEnvRank::Intermediate
        } else if var.is_output() {
            TypeEnvRank::Output
        } else {
            assert!(var.is_temp());
            TypeEnvRank::Temp
        };
        self.ranks.insert(var.get_or_create_typevar(), rank);
    }

    fn add_constraint(&mut self, constraint: Constraint) {
        if self.constraints.iter().any(|item| *item == constraint) {
            return;
        }

        // Check extra conditions for InTypeset constraints.
        if let Constraint::InTypeset(tv, _) = &constraint {
            assert!(
                tv.base.is_none(),
                "type variable is {:?}, while expecting none",
                tv
            );
            assert!(
                tv.name.starts_with("typeof_"),
                "Name \"{}\" should start with \"typeof_\"",
                tv.name
            );
        }

        self.constraints.push(constraint);
    }

    /// Returns the canonical representative of the equivalency class of the given argument, or
    /// duplicates it if it's not there yet.
    pub fn get_equivalent(&self, tv: &TypeVar) -> TypeVar {
        let mut tv = tv;
        while let Some(found) = self.equivalency_map.get(tv) {
            tv = found;
        }
        match &tv.base {
            Some(parent) => self
                .get_equivalent(&parent.type_var)
                .derived(parent.derived_func),
            None => tv.clone(),
        }
    }

    /// Get the rank of tv in the partial order:
    /// - TVs directly associated with a Var get their rank from the Var (see register()).
    /// - Internally generated non-derived TVs implicitly get the lowest rank (0).
    /// - Derived variables get their rank from their free typevar.
    /// - Singletons have the highest rank.
    /// - TVs associated with vars in a source pattern have a higher rank than TVs associated with
    /// temporary vars.
    fn rank(&self, tv: &TypeVar) -> u8 {
        let actual_tv = match tv.base {
            Some(_) => tv.free_typevar(),
            None => Some(tv.clone()),
        };

        let rank = match actual_tv {
            Some(actual_tv) => match self.ranks.get(&actual_tv) {
                Some(rank) => Some(*rank),
                None => {
                    assert!(
                        !actual_tv.name.starts_with("typeof_"),
                        format!("variable {} should be explicitly ranked", actual_tv.name)
                    );
                    None
                }
            },
            None => None,
        };

        let rank = match rank {
            Some(rank) => rank,
            None => {
                if tv.singleton_type().is_some() {
                    TypeEnvRank::Singleton
                } else {
                    TypeEnvRank::Internal
                }
            }
        };

        rank as u8
    }

    /// Record the fact that the free tv1 is part of the same equivalence class as tv2. The
    /// canonical representative of the merged class is tv2's canonical representative.
    fn record_equivalent(&mut self, tv1: TypeVar, tv2: TypeVar) {
        assert!(tv1.base.is_none());
        assert!(self.get_equivalent(&tv1) == tv1);
        if let Some(tv2_base) = &tv2.base {
            // Ensure there are no cycles.
            assert!(self.get_equivalent(&tv2_base.type_var) != tv1);
        }
        self.equivalency_map.insert(tv1, tv2);
    }

    /// Get the free typevars in the current type environment.
    pub fn free_typevars(&self, var_pool: &mut VarPool) -> Vec<TypeVar> {
        let mut typevars = Vec::new();
        typevars.extend(self.equivalency_map.keys().cloned());
        typevars.extend(
            self.vars
                .iter()
                .map(|&var_index| var_pool.get_mut(var_index).get_or_create_typevar()),
        );

        let set: HashSet<TypeVar> = HashSet::from_iter(
            typevars
                .iter()
                .map(|tv| self.get_equivalent(tv).free_typevar())
                .filter(|opt_tv| {
                    // Filter out singleton types.
                    opt_tv.is_some()
                })
                .map(|tv| tv.unwrap()),
        );
        Vec::from_iter(set)
    }

    /// Normalize by collapsing any roots that don't correspond to a concrete type var AND have a
    /// single type var derived from them or equivalent to them.
    ///
    /// e.g. if we have a root of the tree that looks like:
    ///
    ///   typeof_a   typeof_b
    ///          \\  /
    ///       typeof_x
    ///           |
    ///         half_width(1)
    ///           |
    ///           1
    ///
    /// we want to collapse the linear path between 1 and typeof_x. The resulting graph is:
    ///
    ///   typeof_a   typeof_b
    ///          \\  /
    ///       typeof_x
    fn normalize(&mut self, var_pool: &mut VarPool) {
        let source_tvs: HashSet<TypeVar> = HashSet::from_iter(
            self.vars
                .iter()
                .map(|&var_index| var_pool.get_mut(var_index).get_or_create_typevar()),
        );

        let mut children: HashMap<TypeVar, HashSet<TypeVar>> = HashMap::new();

        // Insert all the parents found by the derivation relationship.
        for type_var in self.equivalency_map.values() {
            if type_var.base.is_none() {
                continue;
            }

            let parent_tv = type_var.free_typevar();
            if parent_tv.is_none() {
                // Ignore this type variable, it's a singleton.
                continue;
            }
            let parent_tv = parent_tv.unwrap();

            children
                .entry(parent_tv)
                .or_insert_with(HashSet::new)
                .insert(type_var.clone());
        }

        // Insert all the explicit equivalency links.
        for (equivalent_tv, canon_tv) in self.equivalency_map.iter() {
            children
                .entry(canon_tv.clone())
                .or_insert_with(HashSet::new)
                .insert(equivalent_tv.clone());
        }

        // Remove links that are straight paths up to typevar of variables.
        for free_root in self.free_typevars(var_pool) {
            let mut root = &free_root;
            while !source_tvs.contains(&root)
                && children.contains_key(&root)
                && children.get(&root).unwrap().len() == 1
            {
                let child = children.get(&root).unwrap().iter().next().unwrap();
                assert_eq!(self.equivalency_map[child], root.clone());
                self.equivalency_map.remove(child);
                root = child;
            }
        }
    }

    /// Extract a clean type environment from self, that only mentions type vars associated with
    /// real variables.
    fn extract(self, var_pool: &mut VarPool) -> TypeEnvironment {
        let vars_tv: HashSet<TypeVar> = HashSet::from_iter(
            self.vars
                .iter()
                .map(|&var_index| var_pool.get_mut(var_index).get_or_create_typevar()),
        );

        let mut new_equivalency_map: HashMap<TypeVar, TypeVar> = HashMap::new();
        for tv in &vars_tv {
            let canon_tv = self.get_equivalent(tv);
            if *tv != canon_tv {
                new_equivalency_map.insert(tv.clone(), canon_tv.clone());
            }

            // Sanity check: the translated type map should only refer to real variables.
            assert!(vars_tv.contains(tv));
            let canon_free_tv = canon_tv.free_typevar();
            assert!(canon_free_tv.is_none() || vars_tv.contains(&canon_free_tv.unwrap()));
        }

        let mut new_constraints: HashSet<Constraint> = HashSet::new();
        for constraint in &self.constraints {
            let constraint = constraint.translate_with_env(&self);
            if constraint.is_trivial() || new_constraints.contains(&constraint) {
                continue;
            }

            // Sanity check: translated constraints should refer only to real variables.
            for arg in constraint.typevar_args() {
                let arg_free_tv = arg.free_typevar();
                assert!(arg_free_tv.is_none() || vars_tv.contains(&arg_free_tv.unwrap()));
            }

            new_constraints.insert(constraint);
        }

        TypeEnvironment {
            vars: self.vars,
            ranks: self.ranks,
            equivalency_map: new_equivalency_map,
            constraints: Vec::from_iter(new_constraints),
        }
    }
}

/// Replaces an external type variable according to the following rules:
/// - if a local copy is present in the map, return it.
/// - or if it's derived, create a local derived one that recursively substitutes the parent.
/// - or return itself.
fn substitute(map: &HashMap<&TypeVar, TypeVar>, external_type_var: &TypeVar) -> TypeVar {
    match map.get(&external_type_var) {
        Some(own_type_var) => own_type_var.clone(),
        None => match &external_type_var.base {
            Some(parent) => {
                let parent_substitute = substitute(map, &parent.type_var);
                TypeVar::derived(&parent_substitute, parent.derived_func)
            }
            None => external_type_var.clone(),
        },
    }
}

/// Normalize a (potentially derived) typevar using the following rules:
///
/// - vector and width derived functions commute
///     {HALF,DOUBLE}VECTOR({HALF,DOUBLE}WIDTH(base)) ->
///     {HALF,DOUBLE}WIDTH({HALF,DOUBLE}VECTOR(base))
///
/// - half/double pairs collapse
///     {HALF,DOUBLE}WIDTH({DOUBLE,HALF}WIDTH(base)) -> base
///     {HALF,DOUBLE}VECTOR({DOUBLE,HALF}VECTOR(base)) -> base
fn canonicalize_derivations(tv: TypeVar) -> TypeVar {
    let base = match &tv.base {
        Some(base) => base,
        None => return tv,
    };

    let derived_func = base.derived_func;

    if let Some(base_base) = &base.type_var.base {
        let base_base_tv = &base_base.type_var;
        match (derived_func, base_base.derived_func) {
            (DerivedFunc::HalfWidth, DerivedFunc::DoubleWidth)
            | (DerivedFunc::DoubleWidth, DerivedFunc::HalfWidth)
            | (DerivedFunc::HalfVector, DerivedFunc::DoubleVector)
            | (DerivedFunc::DoubleVector, DerivedFunc::HalfVector) => {
                // Cancelling bijective transformations. This doesn't hide any overflow issues
                // since derived type sets are checked upon derivaion, and base typesets are only
                // allowed to shrink.
                return canonicalize_derivations(base_base_tv.clone());
            }
            (DerivedFunc::HalfWidth, DerivedFunc::HalfVector)
            | (DerivedFunc::HalfWidth, DerivedFunc::DoubleVector)
            | (DerivedFunc::DoubleWidth, DerivedFunc::DoubleVector)
            | (DerivedFunc::DoubleWidth, DerivedFunc::HalfVector) => {
                // Arbitrarily put WIDTH derivations before VECTOR derivations, since they commute.
                return canonicalize_derivations(
                    base_base_tv
                        .derived(derived_func)
                        .derived(base_base.derived_func),
                );
            }
            _ => {}
        };
    }

    canonicalize_derivations(base.type_var.clone()).derived(derived_func)
}

/// Given typevars tv1 and tv2 (which could be derived from one another), constrain their typesets
/// to be the same. When one is derived from the other, repeat the constrain process until
/// a fixed point is reached.
fn constrain_fixpoint(tv1: &TypeVar, tv2: &TypeVar) {
    loop {
        let old_tv1_ts = tv1.get_typeset().clone();
        tv2.constrain_types(tv1.clone());
        if tv1.get_typeset() == old_tv1_ts {
            break;
        }
    }

    let old_tv2_ts = tv2.get_typeset().clone();
    tv1.constrain_types(tv2.clone());
    // The above loop should ensure that all reference cycles have been handled.
    assert!(old_tv2_ts == tv2.get_typeset());
}

/// Unify tv1 and tv2 in the given type environment. tv1 must have a rank greater or equal to tv2's
/// one, modulo commutations.
fn unify(tv1: &TypeVar, tv2: &TypeVar, type_env: &mut TypeEnvironment) -> Result<(), String> {
    let tv1 = canonicalize_derivations(type_env.get_equivalent(tv1));
    let tv2 = canonicalize_derivations(type_env.get_equivalent(tv2));

    if tv1 == tv2 {
        // Already unified.
        return Ok(());
    }

    if type_env.rank(&tv2) < type_env.rank(&tv1) {
        // Make sure tv1 always has the smallest rank, since real variables have the higher rank
        // and we want them to be the canonical representatives of their equivalency classes.
        return unify(&tv2, &tv1, type_env);
    }

    constrain_fixpoint(&tv1, &tv2);

    if tv1.get_typeset().size() == 0 || tv2.get_typeset().size() == 0 {
        return Err(format!(
            "Error: empty type created when unifying {} and {}",
            tv1.name, tv2.name
        ));
    }

    let base = match &tv1.base {
        Some(base) => base,
        None => {
            type_env.record_equivalent(tv1, tv2);
            return Ok(());
        }
    };

    if let Some(inverse) = base.derived_func.inverse() {
        return unify(&base.type_var, &tv2.derived(inverse), type_env);
    }

    type_env.add_constraint(Constraint::Eq(tv1, tv2));
    Ok(())
}

/// Perform type inference on one Def in the current type environment and return an updated type
/// environment or error.
///
/// At a high level this works by creating fresh copies of each formal type var in the Def's
/// instruction's signature, and unifying the formal typevar with the corresponding actual typevar.
fn infer_definition(
    def: &Def,
    var_pool: &mut VarPool,
    type_env: TypeEnvironment,
    last_type_index: &mut usize,
) -> Result<TypeEnvironment, String> {
    let apply = &def.apply;
    let inst = &apply.inst;

    let mut type_env = type_env;
    let free_formal_tvs = inst.all_typevars();

    let mut original_to_own_typevar: HashMap<&TypeVar, TypeVar> = HashMap::new();
    for &tv in &free_formal_tvs {
        assert!(original_to_own_typevar
            .insert(
                tv,
                TypeVar::copy_from(tv, format!("own_{}", last_type_index))
            )
            .is_none());
        *last_type_index += 1;
    }

    // Update the mapping with any explicity bound type vars:
    for (i, value_type) in apply.value_types.iter().enumerate() {
        let singleton = TypeVar::new_singleton(value_type.clone());
        assert!(original_to_own_typevar
            .insert(free_formal_tvs[i], singleton)
            .is_some());
    }

    // Get fresh copies for each typevar in the signature (both free and derived).
    let mut formal_tvs = Vec::new();
    formal_tvs.extend(inst.value_results.iter().map(|&i| {
        substitute(
            &original_to_own_typevar,
            inst.operands_out[i].type_var().unwrap(),
        )
    }));
    formal_tvs.extend(inst.value_opnums.iter().map(|&i| {
        substitute(
            &original_to_own_typevar,
            inst.operands_in[i].type_var().unwrap(),
        )
    }));

    // Get the list of actual vars.
    let mut actual_vars = Vec::new();
    actual_vars.extend(inst.value_results.iter().map(|&i| def.defined_vars[i]));
    actual_vars.extend(
        inst.value_opnums
            .iter()
            .map(|&i| apply.args[i].unwrap_var()),
    );

    // Get the list of the actual TypeVars.
    let mut actual_tvs = Vec::new();
    for var_index in actual_vars {
        let var = var_pool.get_mut(var_index);
        type_env.register(var_index, var);
        actual_tvs.push(var.get_or_create_typevar());
    }

    // Make sure we start unifying with the control type variable first, by putting it at the
    // front of both vectors.
    if let Some(poly) = &inst.polymorphic_info {
        let own_ctrl_tv = &original_to_own_typevar[&poly.ctrl_typevar];
        let ctrl_index = formal_tvs.iter().position(|tv| tv == own_ctrl_tv).unwrap();
        if ctrl_index != 0 {
            formal_tvs.swap(0, ctrl_index);
            actual_tvs.swap(0, ctrl_index);
        }
    }

    // Unify each actual type variable with the corresponding formal type variable.
    for (actual_tv, formal_tv) in actual_tvs.iter().zip(&formal_tvs) {
        if let Err(msg) = unify(actual_tv, formal_tv, &mut type_env) {
            return Err(format!(
                "fail ti on {} <: {}: {}",
                actual_tv.name, formal_tv.name, msg
            ));
        }
    }

    // Add any instruction specific constraints.
    for constraint in &inst.constraints {
        type_env.add_constraint(constraint.translate_with_map(&original_to_own_typevar));
    }

    Ok(type_env)
}

/// Perform type inference on an transformation. Return an updated type environment or error.
pub(crate) fn infer_transform(
    src: DefIndex,
    dst: &[DefIndex],
    def_pool: &DefPool,
    var_pool: &mut VarPool,
) -> Result<TypeEnvironment, String> {
    let mut type_env = TypeEnvironment::new();
    let mut last_type_index = 0;

    // Execute type inference on the source pattern.
    type_env = infer_definition(def_pool.get(src), var_pool, type_env, &mut last_type_index)
        .map_err(|err| format!("In src pattern: {}", err))?;

    // Collect the type sets once after applying the source patterm; we'll compare the typesets
    // after we've also considered the destination pattern, and will emit supplementary InTypeset
    // checks if they don't match.
    let src_typesets = type_env
        .vars
        .iter()
        .map(|&var_index| {
            let var = var_pool.get_mut(var_index);
            let tv = type_env.get_equivalent(&var.get_or_create_typevar());
            (var_index, tv.get_typeset().clone())
        })
        .collect::<Vec<_>>();

    // Execute type inference on the destination pattern.
    for (i, &def_index) in dst.iter().enumerate() {
        let def = def_pool.get(def_index);
        type_env = infer_definition(def, var_pool, type_env, &mut last_type_index)
            .map_err(|err| format!("line {}: {}", i, err))?;
    }

    for (var_index, src_typeset) in src_typesets {
        let var = var_pool.get(var_index);
        if !var.has_free_typevar() {
            continue;
        }
        let tv = type_env.get_equivalent(&var.get_typevar().unwrap());
        let new_typeset = tv.get_typeset();
        assert!(
            new_typeset.is_subset(&src_typeset),
            "type sets can only get narrower"
        );
        if new_typeset != src_typeset {
            type_env.add_constraint(Constraint::InTypeset(tv.clone(), new_typeset.clone()));
        }
    }

    type_env.normalize(var_pool);

    Ok(type_env.extract(var_pool))
}
