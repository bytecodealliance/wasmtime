//! Analysis of which of a core module's function imports must always have the
//! same `vmctx`.
//!
//! Calling an imported function requires passing the callee's `vmctx`, which is
//! loaded from that import's slot in the `VMFunctionImport` array. Imports that
//! are always satisfied by functions from the same instance always hold the
//! same pointer, so compiling all of them to load from the lowest-numbered such
//! slot lets GVN collapse those loads, and every subsequent load hanging off
//! the callee context, into one.
//!
//! This is a *must* analysis: putting two imports in one set claims that they
//! share a `vmctx` in *every* instantiation. Splitting sets apart is always
//! sound; merging them when there is some possible scenario that they aren't
//! the same `vmctx` is unsound.
//!
//! Each core module's lattice is the same-vmctx partitions of its function
//! imports, ordered by refinement. Top is the single set containing everything,
//! bottom is all singletons, and meet is the coarsest common refinement. Every
//! module starts at top, so one that is never instantiated stays there
//! vacuously. Meet is applied once per instantiation, and once per module that
//! escapes to the host, which may instantiate it with anything. See
//! `SameVmctxPartition`.
//!
//! An instantiation's arguments are always exports of instances created before
//! it, so the DFG is a DAG and one pass reaches the fixpoint.

use crate::compile::ModuleTranslation;
use crate::component::ExportItem;
use crate::component::dfg::{
    AdapterId, AdapterModuleId, ComponentDfg, CoreDef, Export, Instance, InstanceId, SideEffect,
};
use crate::prelude::*;
use crate::union_find::UnionFind;
use crate::{EntityIndex, EntityRef, FuncIndex, PrimaryMap, SecondaryMap, StaticModuleIndex};
use core::mem;
use std::collections::HashMap;
use std::collections::hash_map::Entry;

/// An identity for the `vmctx` that a core definition's `VMFuncRef` carries.
///
/// Equal keys guarantee equal `vmctx` pointers at runtime.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum VmctxKey {
    /// The component's own `VMComponentContext`.
    ///
    /// Every trampoline and unsafe intrinsic takes its `VMFuncRef::vmctx` from
    /// the component's single `ComponentInstance`.
    Component,

    /// The `VMContext` of a particular core Wasm instance in this component.
    CoreInstance(InstanceId),

    /// The `VMContext` of a particular adapter module's instance.
    ///
    /// Adapter modules are instantiated at most once each, so the module id
    /// also names the instance.
    AdapterModule(AdapterModuleId),
}

/// A partition of one core module's function imports into sets that must share
/// a `vmctx`.
#[derive(Clone, Debug, Default)]
enum SameVmctxPartition {
    /// One set containing every function import: they all share a `vmctx`.
    ///
    /// The lattice's top element, and every module's initial state.
    #[default]
    Top,

    /// Any other partition.
    ///
    /// Function imports absent from the union-find are singletons, so an empty
    /// union-find is all singletons: the lattice's bottom element.
    Known(UnionFind<FuncIndex>),
}

impl SameVmctxPartition {
    /// The lattice's bottom element: every function import in its own
    /// singleton set, so nothing is known.
    fn bottom() -> Self {
        SameVmctxPartition::Known(UnionFind::new())
    }

    /// Build the partition induced by one instantiation, where `keys[i]` is
    /// the `vmctx` key of the definition satisfying the module's `i`th
    /// function import.
    ///
    /// `None` represents "unknown" and makes that import a singleton.
    ///
    /// `first` is scratch space, reused across calls to avoid reallocating a
    /// hash map per instantiation.
    fn from_keys(first: &mut HashMap<VmctxKey, FuncIndex>, keys: &[Option<VmctxKey>]) -> Self {
        first.clear();
        let mut sets = UnionFind::new();
        for (i, key) in keys.iter().enumerate() {
            let Some(key) = *key else { continue };
            let func = FuncIndex::new(i);
            match first.entry(key) {
                Entry::Occupied(e) => {
                    sets.union(*e.get(), func);
                }
                Entry::Vacant(e) => {
                    e.insert(func);
                }
            }
        }
        SameVmctxPartition::Known(sets)
    }

    /// The coarsest common refinement of `a` and `b`: the greatest lower
    /// bound in this lattice.
    ///
    /// `groups` is scratch space, reused across calls to avoid reallocating a
    /// hash map per meet.
    fn meet(groups: &mut HashMap<(FuncIndex, FuncIndex), FuncIndex>, a: Self, b: Self) -> Self {
        use SameVmctxPartition::*;
        match (a, b) {
            // Top is the identity.
            (Top, p) | (p, Top) => p,

            (Known(ref a), Known(ref b)) => {
                // Two imports share a block of the result exactly when they
                // share a block of `a` *and* a block of `b`, so an import that
                // is a singleton in either operand is a singleton in the
                // result. Only imports that are non-singletons in *both* need
                // looking at, which is why this walks the smaller operand and
                // probes the larger: `O(min(|a|, |b|))`, not anything
                // proportional to the number of function imports.
                let (small, large) = if a.len() <= b.len() { (a, b) } else { (b, a) };
                let mut sets = UnionFind::new();

                // Group by the pair of blocks an element belongs to, then
                // union together everything that lands in the same group.
                groups.clear();
                for func in small.elems() {
                    if !large.contains(func) {
                        continue;
                    }
                    let group = (
                        a.find_without_path_compression(func),
                        b.find_without_path_compression(func),
                    );
                    match groups.entry(group) {
                        Entry::Occupied(e) => {
                            sets.union(*e.get(), func);
                        }
                        Entry::Vacant(e) => {
                            e.insert(func);
                        }
                    }
                }

                Known(sets)
            }
        }
    }

    /// Canonicalize to the lowest-numbered function import in `func`'s set,
    /// whose `vmctx` slot every member of the set can share.
    ///
    /// Lowest-numbered canonicalization ultimately allows for better codegen,
    /// because the constant can fit in fewer instructions and/or smaller
    /// immediate encodings.
    fn representative(&self, func: FuncIndex) -> FuncIndex {
        match self {
            // One set of everything, whose least member is import 0.
            SameVmctxPartition::Top => FuncIndex::new(0),

            // We rely on the union-find caching a set's min, rather than
            // walking the set's elements to find the min on demand, to avoid
            // accidentally-quadratic runtimes.
            SameVmctxPartition::Known(sets) => sets.set_min(func),
        }
    }
}

/// Accumulates observations to produce a `SameVmctxImports`.
#[derive(Default)]
struct SameVmctxBuilder {
    /// The current lattice state of each static module.
    ///
    /// An unobserved module reads as `Top`.
    partitions: SecondaryMap<StaticModuleIndex, SameVmctxPartition>,

    /// Scratch space for `SameVmctxPartition::from_keys` so that its allocation
    /// is reused across instantiations.
    scratch_first: HashMap<VmctxKey, FuncIndex>,

    /// Scratch space for `SameVmctxPartition::meet` so that its allocation is
    /// reused across meets.
    scratch_groups: HashMap<(FuncIndex, FuncIndex), FuncIndex>,
}

impl SameVmctxBuilder {
    /// Record an instantiation of `module` in which the definition satisfying
    /// its `i`th function import has `vmctx` key `keys[i]`.
    fn observe_instantiation(&mut self, module: StaticModuleIndex, keys: &[Option<VmctxKey>]) {
        let partition = SameVmctxPartition::from_keys(&mut self.scratch_first, keys);
        self.observe(module, partition);
    }

    /// Record that `module` may also be instantiated in ways we cannot see,
    /// for example because the component exports it to the host.
    fn observe_unknown_instantiation(&mut self, module: StaticModuleIndex) {
        self.observe(module, SameVmctxPartition::bottom());
    }

    fn observe(&mut self, module: StaticModuleIndex, partition: SameVmctxPartition) {
        let current = mem::take(&mut self.partitions[module]);
        let met = SameVmctxPartition::meet(&mut self.scratch_groups, current, partition);
        self.partitions[module] = met;
    }

    /// Finish observing and produce the queryable analysis results.
    fn finish(self) -> SameVmctxImports {
        SameVmctxImports {
            partitions: self.partitions,
        }
    }
}

/// A completed same-`vmctx` analysis, ready to be queried.
struct SameVmctxImports {
    partitions: SecondaryMap<StaticModuleIndex, SameVmctxPartition>,
}

impl SameVmctxImports {
    /// The function import of `module` whose `vmctx` slot `func` should be
    /// compiled to use.
    fn representative(&self, module: StaticModuleIndex, func: FuncIndex) -> FuncIndex {
        self.partitions[module].representative(func)
    }
}

/// Run the same-`vmctx` analysis over `dfg`, recording its results in each
/// module's `ModuleTranslation::same_vmctx_imported_functions`.
pub fn analyze_same_vmctx_imports(
    dfg: &ComponentDfg,
    static_modules: &mut PrimaryMap<StaticModuleIndex, ModuleTranslation<'_>>,
) {
    let mut builder = SameVmctxBuilder::default();

    // Scratch space, reused across the calls below.
    let mut keys = Vec::new();
    let mut stack = Vec::new();

    // Observe every core module instantiation that the component itself
    // performs.
    for effect in dfg.side_effects.iter() {
        let SideEffect::Instance(id, _) = effect else {
            continue;
        };

        let Instance::Static(module, args) = &dfg.instances[*id] else {
            // A module imported from the host is not one we are compiling.
            continue;
        };

        observe_instantiation(&mut keys, &mut builder, dfg, static_modules, *module, args);
    }

    // Adapter modules do not appear in `side_effects`; they are instantiated
    // lazily, at most once each, as their adapters are referenced.
    for (_, (module, args)) in dfg.adapter_modules.iter() {
        observe_instantiation(&mut keys, &mut builder, dfg, static_modules, *module, args);
    }

    // The host may instantiate an exported module with anything at all.
    for (_, (export, _)) in dfg.exports.iter() {
        observe_exported_modules(&mut stack, &mut builder, export);
    }

    let analysis = builder.finish();

    for (module, translation) in static_modules.iter_mut() {
        for i in 0..translation.module.num_imported_funcs {
            let func = FuncIndex::new(i);
            let representative = analysis.representative(module, func);
            if representative != func {
                translation.imported_func_vmctx_representative[func] = representative.into();
            }
        }
    }
}

/// Observe an instantiation of `module` with the given positional arguments.
///
/// `keys` is scratch space, reused across calls to avoid reallocating a vector
/// per instantiation. Its contents on entry are ignored.
fn observe_instantiation(
    keys: &mut Vec<Option<VmctxKey>>,
    builder: &mut SameVmctxBuilder,
    dfg: &ComponentDfg,
    static_modules: &PrimaryMap<StaticModuleIndex, ModuleTranslation<'_>>,
    module: StaticModuleIndex,
    args: &[CoreDef],
) {
    let translation = &static_modules[module];
    keys.clear();
    keys.resize(translation.module.num_imported_funcs, None);

    for (position, arg) in args.iter().enumerate() {
        let Some(EntityIndex::Function(func)) = translation.module.import_index(position) else {
            continue;
        };
        keys[func.index()] = vmctx_key(dfg, static_modules, arg);
    }

    builder.observe_instantiation(module, keys);
}

/// Observe every static module reachable from `export`, any of which the host
/// may instantiate however it likes.
///
/// `stack` is scratch space, reused across calls to avoid reallocating a
/// vector per export. Its contents on entry are ignored.
fn observe_exported_modules<'a>(
    stack: &mut Vec<&'a Export>,
    builder: &mut SameVmctxBuilder,
    export: &'a Export,
) {
    stack.clear();
    stack.push(export);

    while let Some(export) = stack.pop() {
        match export {
            Export::ModuleStatic { index, .. } => builder.observe_unknown_instantiation(*index),

            Export::Instance { exports, .. } => {
                stack.extend(exports.iter().map(|(_, (export, _))| export));
            }

            Export::LiftedFunction { .. } | Export::ModuleImport { .. } | Export::Type(_) => {}
        }
    }
}

/// The `vmctx` that `def`'s `VMFuncRef` carries, when we can see it
/// statically.
fn vmctx_key(
    dfg: &ComponentDfg,
    static_modules: &PrimaryMap<StaticModuleIndex, ModuleTranslation<'_>>,
    def: &CoreDef,
) -> Option<VmctxKey> {
    // A module may import a function and re-export it, in which case the
    // `vmctx` belongs to whichever instance actually defines the function, so
    // follow such chains back to the definition. This mirrors
    // `translate::resolve_core_export`.
    let mut def = def;

    // The instance most recently looked at. The walk strictly decreases
    // through this, which is what makes it terminate.
    let mut previous: Option<InstanceId> = None;

    loop {
        // NB: deliberately exhaustive so that new variants must be classified
        // here.
        let export = match def {
            // Trampolines and unsafe intrinsics take their `VMFuncRef` from
            // the component instance, so they all share its context.
            CoreDef::Trampoline(_) | CoreDef::UnsafeIntrinsic(..) => {
                return Some(VmctxKey::Component);
            }

            CoreDef::Adapter(id) => return adapter_vmctx_key(dfg, static_modules, *id),

            // Not a function, so it never satisfies a function import. Be
            // conservative anyway.
            CoreDef::InstanceFlags(_) => return None,

            CoreDef::Export(export) => export,
        };

        if previous.is_some_and(|p| export.instance.index() >= p.index()) {
            // Unreachable, since an instantiation's arguments are exports of
            // earlier instances. Give up rather than loop forever.
            return None;
        }
        previous = Some(export.instance);

        let Instance::Static(module, args) = &dfg.instances[export.instance] else {
            // An instance of a host module, whose exports we cannot see into.
            return None;
        };

        let ExportItem::Index(index) = &export.item else {
            // Names are only used for instances of modules whose shape is not
            // statically known, which the arm above filtered out.
            return None;
        };

        let module = &static_modules[*module].module;

        // The common case: this instance's module defines the function, so
        // the function's context is this instance's context.
        if !module.is_imported(*index) {
            return Some(VmctxKey::CoreInstance(export.instance));
        }

        // Otherwise it is a re-export of one of the module's imports, so keep
        // walking through whichever argument satisfied that import.
        let position = module
            .import_position(*index)
            .expect("imported entities always have an associated import initializer");
        def = &args[position];
    }
}

/// The `vmctx` of a fused adapter, which lives in the instance of the adapter
/// module it was compiled into.
fn adapter_vmctx_key(
    dfg: &ComponentDfg,
    static_modules: &PrimaryMap<StaticModuleIndex, ModuleTranslation<'_>>,
    id: AdapterId,
) -> Option<VmctxKey> {
    let (adapter_module, index) = *dfg.adapter_partitionings.get(id)?;
    let (static_module, _) = dfg.adapter_modules[adapter_module];

    debug_assert!(
        !static_modules[static_module].module.is_imported(index),
        "adapter modules always define their exported adapters",
    );

    Some(VmctxKey::AdapterModule(adapter_module))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::property_check;
    use mutatis::{Mutate, check::CheckResult, mutators as m};

    impl SameVmctxPartition {
        /// Are `a` and `b` in the same block of this partition?
        fn same_block(&self, a: FuncIndex, b: FuncIndex) -> bool {
            // A block's representative is a canonical name for it.
            self.representative(a) == self.representative(b)
        }

        /// This partition's blocks over the function imports `0..n`, in canonical
        /// form: each block is sorted, and the blocks are ordered by their least
        /// member.
        fn blocks(&self, n: u32) -> Vec<Vec<u32>> {
            let mut blocks = std::collections::BTreeMap::<u32, Vec<u32>>::new();
            for i in 0..n {
                let rep = self.representative(FuncIndex::from_u32(i)).as_u32();
                blocks.entry(rep).or_default().push(i);
            }
            blocks.into_values().collect()
        }

        /// Is `self <= other` in this lattice, i.e. is every block of `self`
        /// contained within a block of `other`?
        fn refines(&self, other: &Self, n: u32) -> bool {
            (0..n).all(|i| {
                (0..n).all(|j| {
                    let (a, b) = (FuncIndex::from_u32(i), FuncIndex::from_u32(j));
                    !self.same_block(a, b) || other.same_block(a, b)
                })
            })
        }
    }

    /// A `vmctx` key naming the `i`th core instance.
    fn key(i: u32) -> Option<VmctxKey> {
        Some(VmctxKey::CoreInstance(InstanceId::from_u32(i)))
    }

    /// The absence of a key, i.e. an import whose `vmctx` we cannot determine.
    fn unknown() -> Option<VmctxKey> {
        None
    }

    /// Partition one module's imports, given one entry per instantiation
    /// holding the `vmctx` key of each of the module's function imports.
    fn analyze(instantiations: &[&[Option<VmctxKey>]]) -> SameVmctxPartition {
        let module = StaticModuleIndex::from_u32(0);
        let mut builder = SameVmctxBuilder::default();
        for keys in instantiations {
            builder.observe_instantiation(module, keys);
        }
        builder.finish().partitions[module].clone()
    }

    /// Like `analyze`, but returning the canonical blocks over `n` function
    /// imports, which is what most assertions below are about.
    fn blocks(n: u32, instantiations: &[&[Option<VmctxKey>]]) -> Vec<Vec<u32>> {
        analyze(instantiations).blocks(n)
    }

    #[test]
    fn a_module_that_is_never_instantiated_stays_at_top() {
        // Vacuously, every import shares a `vmctx` with every other: there
        // is no instantiation to say otherwise.
        let p = analyze(&[]);
        assert!(matches!(p, SameVmctxPartition::Top));
        assert_eq!(p.blocks(4), vec![vec![0, 1, 2, 3]]);

        for i in 0..4 {
            assert_eq!(
                p.representative(FuncIndex::from_u32(i)),
                FuncIndex::from_u32(0),
                "import {i} should use import 0's `vmctx` slot",
            );
        }
    }

    #[test]
    fn one_instantiation_with_all_imports_from_one_instance() {
        assert_eq!(
            blocks(4, &[&[key(0), key(0), key(0), key(0)]]),
            vec![vec![0, 1, 2, 3]],
        );
    }

    #[test]
    fn one_instantiation_with_all_imports_from_distinct_instances() {
        assert_eq!(
            blocks(4, &[&[key(0), key(1), key(2), key(3)]]),
            vec![vec![0], vec![1], vec![2], vec![3]],
        );
    }

    #[test]
    fn one_instantiation_split_across_two_instances() {
        assert_eq!(
            blocks(4, &[&[key(9), key(9), key(5), key(5)]]),
            vec![vec![0, 1], vec![2, 3]],
        );
    }

    #[test]
    fn one_instantiation_with_interleaved_instances() {
        // Blocks need not be contiguous ranges of indices.
        assert_eq!(
            blocks(4, &[&[key(0), key(1), key(0), key(1)]]),
            vec![vec![0, 2], vec![1, 3]],
        );
    }

    #[test]
    fn unknown_keys_are_singletons() {
        // An unresolved import shares a `vmctx` with nothing, not even
        // another unresolved import.
        assert_eq!(
            blocks(5, &[&[key(0), unknown(), key(0), unknown(), key(0)]]),
            vec![vec![0, 2, 4], vec![1], vec![3]],
        );
    }

    #[test]
    fn distinct_key_variants_are_distinct_keys() {
        // These three keys all carry a `0`-ish payload but name three
        // different contexts.
        let component = Some(VmctxKey::Component);
        let instance = Some(VmctxKey::CoreInstance(InstanceId::from_u32(0)));
        let adapter = Some(VmctxKey::AdapterModule(AdapterModuleId::from_u32(0)));
        assert_eq!(
            blocks(
                6,
                &[&[component, instance, adapter, component, instance, adapter]]
            ),
            vec![vec![0, 3], vec![1, 4], vec![2, 5]],
        );
    }

    #[test]
    fn two_agreeing_instantiations_change_nothing() {
        // Different instances, but the same *partition*.
        assert_eq!(
            blocks(
                4,
                &[
                    &[key(0), key(0), key(1), key(1)],
                    &[key(7), key(7), key(8), key(8)],
                ],
            ),
            vec![vec![0, 1], vec![2, 3]],
        );
    }

    #[test]
    fn a_disagreeing_instantiation_breaks_everything_apart() {
        assert_eq!(
            blocks(
                4,
                &[
                    &[key(0), key(0), key(0), key(0)],
                    &[key(0), key(1), key(2), key(3)],
                ],
            ),
            vec![vec![0], vec![1], vec![2], vec![3]],
        );
    }

    #[test]
    fn a_partially_disagreeing_instantiation_keeps_what_it_agrees_on() {
        assert_eq!(
            blocks(
                4,
                &[
                    &[key(0), key(0), key(0), key(0)],
                    &[key(0), key(0), key(1), key(1)],
                ],
            ),
            vec![vec![0, 1], vec![2, 3]],
        );
    }

    #[test]
    fn crossing_splits_meet_to_all_singletons() {
        // `{0,1}{2,3}` and `{0,2}{1,3}` share no pair, so nothing survives
        // even though neither operand is near bottom.
        assert_eq!(
            blocks(
                4,
                &[
                    &[key(0), key(0), key(1), key(1)],
                    &[key(0), key(1), key(0), key(1)],
                ],
            ),
            vec![vec![0], vec![1], vec![2], vec![3]],
        );
    }

    #[test]
    fn partially_crossing_splits_keep_their_common_refinement() {
        // `{0,1,2}{3,4,5}` meet `{0,1}{2,3}{4,5}` == `{0,1}{2}{3}{4,5}`.
        assert_eq!(
            blocks(
                6,
                &[
                    &[key(0), key(0), key(0), key(1), key(1), key(1)],
                    &[key(0), key(0), key(1), key(1), key(2), key(2)],
                ],
            ),
            vec![vec![0, 1], vec![2], vec![3], vec![4, 5]],
        );
    }

    #[test]
    fn progressive_refinement_is_order_independent() {
        let a: &[Option<VmctxKey>] = &[key(0), key(0), key(0), key(0), key(0), key(0)];
        let b: &[Option<VmctxKey>] = &[key(0), key(0), key(0), key(1), key(1), key(1)];
        let c: &[Option<VmctxKey>] = &[key(0), key(0), key(1), key(1), key(2), key(2)];

        let expected = vec![vec![0, 1], vec![2], vec![3], vec![4, 5]];
        for order in [
            [a, b, c],
            [a, c, b],
            [b, a, c],
            [b, c, a],
            [c, a, b],
            [c, b, a],
        ] {
            assert_eq!(blocks(6, &order), expected, "order {order:?} disagreed");
        }
    }

    #[test]
    fn an_unknown_instantiation_forces_bottom_and_cannot_be_undone() {
        let module = StaticModuleIndex::from_u32(0);
        let mut builder = SameVmctxBuilder::default();

        builder.observe_instantiation(module, &[key(0), key(0), key(0)]);
        builder.observe_unknown_instantiation(module);
        // Nothing observed later can coarsen the partition back up.
        builder.observe_instantiation(module, &[key(0), key(0), key(0)]);

        let p = builder.finish().partitions[module].clone();
        assert_eq!(p.blocks(3), SameVmctxPartition::bottom().blocks(3));
    }

    #[test]
    fn modules_do_not_interfere_with_each_other() {
        let a = StaticModuleIndex::from_u32(0);
        let b = StaticModuleIndex::from_u32(3);
        let mut builder = SameVmctxBuilder::default();

        builder.observe_instantiation(a, &[key(0), key(0)]);
        builder.observe_instantiation(b, &[key(0), key(1)]);

        let analysis = builder.finish();
        assert_eq!(analysis.partitions[a].blocks(2), vec![vec![0, 1]]);
        assert_eq!(analysis.partitions[b].blocks(2), vec![vec![0], vec![1]]);
        // An untouched module in between is still at top.
        assert_eq!(
            analysis.partitions[StaticModuleIndex::from_u32(1)].blocks(2),
            vec![vec![0, 1]],
        );
    }

    #[test]
    fn degenerate_numbers_of_imports() {
        // Zero function imports: nothing to partition, and nothing that
        // could ask for a representative.
        assert_eq!(blocks(0, &[]), Vec::<Vec<u32>>::new());
        assert_eq!(blocks(0, &[&[]]), Vec::<Vec<u32>>::new());

        // One function import: it shares a `vmctx` with itself no matter
        // what we observe.
        assert_eq!(blocks(1, &[]), vec![vec![0]]);
        assert_eq!(blocks(1, &[&[key(0)]]), vec![vec![0]]);
        assert_eq!(blocks(1, &[&[unknown()]]), vec![vec![0]]);
        assert_eq!(blocks(1, &[&[key(0)], &[key(1)]]), vec![vec![0]]);
    }

    #[test]
    fn a_representative_is_always_its_blocks_least_member() {
        let p = analyze(&[&[key(1), key(0), key(1), key(0), key(1)]]);
        let rep = |i| p.representative(FuncIndex::from_u32(i)).as_u32();
        assert_eq!(rep(0), 0);
        assert_eq!(rep(2), 0);
        assert_eq!(rep(4), 0);
        assert_eq!(rep(1), 1);
        assert_eq!(rep(3), 1);

        // Even when import 0 is in no block but its own.
        let p = analyze(&[&[unknown(), key(0), key(1), key(0), key(1)]]);
        let rep = |i| p.representative(FuncIndex::from_u32(i)).as_u32();
        assert_eq!(rep(0), 0);
        assert_eq!(rep(1), 1);
        assert_eq!(rep(3), 1);
        assert_eq!(rep(2), 2);
        assert_eq!(rep(4), 2);
    }

    #[test]
    fn many_imports_in_many_blocks() {
        // Exercise the union-find with more than the handful of imports the
        // tests above use: 64 imports in 8 strided blocks of 8.
        let keys = (0..64).map(|i| key(i % 8)).collect::<Vec<_>>();
        let expected = (0..8)
            .map(|b| (0..8).map(|i| b + i * 8).collect::<Vec<u32>>())
            .collect::<Vec<_>>();
        assert_eq!(blocks(64, &[&keys]), expected);

        // Meeting that with 8 *contiguous* blocks of 8 leaves all
        // singletons: each contiguous block shares exactly one member with
        // each strided one.
        let contiguous = (0..64).map(|i| key(i / 8)).collect::<Vec<_>>();
        assert_eq!(
            blocks(64, &[&keys, &contiguous]),
            (0..64).map(|i| vec![i]).collect::<Vec<_>>(),
        );
    }

    // Lattice laws.

    /// The number of function imports the lattice-law tests partition.
    const N: u32 = 6;

    /// Build a partition of `0..N` from block labels, where `None` is an
    /// unknown key, i.e. a forced singleton.
    fn partition(labels: &[Option<u32>]) -> SameVmctxPartition {
        let keys = labels
            .iter()
            .map(|l| match l {
                Some(l) => key(*l),
                None => unknown(),
            })
            .collect::<Vec<_>>();
        SameVmctxPartition::from_keys(&mut HashMap::new(), &keys)
    }

    /// Decode a chunk of random bytes into a partition of `0..N`.
    ///
    /// The first byte chooses between the two extremal elements and a
    /// partition built from the remaining bytes, one label per function
    /// import, so that top and bottom come up often enough to exercise the
    /// identity and absorption laws.
    fn decode(bytes: &[u8]) -> SameVmctxPartition {
        let byte = |i: usize| bytes.get(i).copied().unwrap_or(0);
        match byte(0) % 8 {
            0 => SameVmctxPartition::Top,
            1 => SameVmctxPartition::bottom(),
            _ => {
                let labels = (0..N)
                    .map(|i| match byte(1 + i as usize) % 5 {
                        // One label in five is "unknown".
                        4 => None,
                        l => Some(u32::from(l)),
                    })
                    .collect::<Vec<_>>();
                partition(&labels)
            }
        }
    }

    /// Split a byte string into the three partitions the laws are checked on.
    fn decode3(bytes: &[u8]) -> (SameVmctxPartition, SameVmctxPartition, SameVmctxPartition) {
        let chunk = 1 + N as usize;
        let at = |i: usize| decode(bytes.get(i * chunk..).unwrap_or(&[]));
        (at(0), at(1), at(2))
    }

    /// `SameVmctxPartition::meet` on borrowed operands: the laws below apply
    /// it repeatedly to the same partitions, but it consumes them.
    fn meet(a: &SameVmctxPartition, b: &SameVmctxPartition) -> SameVmctxPartition {
        SameVmctxPartition::meet(&mut HashMap::new(), a.clone(), b.clone())
    }

    /// An obviously-correct meet to cross-check the real one against: two
    /// imports share a block of the result exactly when they share a block of
    /// both operands.
    fn reference_meet(a: &SameVmctxPartition, b: &SameVmctxPartition) -> Vec<Vec<u32>> {
        let mut assigned = vec![false; N as usize];
        let mut blocks = Vec::new();
        for i in 0..N {
            if assigned[i as usize] {
                continue;
            }
            let mut block = Vec::new();
            for j in i..N {
                let (x, y) = (FuncIndex::from_u32(i), FuncIndex::from_u32(j));
                if !assigned[j as usize] && a.same_block(x, y) && b.same_block(x, y) {
                    assigned[j as usize] = true;
                    block.push(j);
                }
            }
            blocks.push(block);
        }
        blocks
    }

    #[test]
    fn top_and_bottom_are_the_lattice_bounds() {
        let top = SameVmctxPartition::Top;
        let bottom = SameVmctxPartition::bottom();
        let p = partition(&[Some(0), Some(0), Some(1), Some(1), None, None]);

        assert_eq!(meet(&top, &p).blocks(N), p.blocks(N));
        assert_eq!(meet(&p, &top).blocks(N), p.blocks(N));
        assert_eq!(meet(&bottom, &p).blocks(N), bottom.blocks(N));
        assert_eq!(meet(&p, &bottom).blocks(N), bottom.blocks(N));

        assert!(p.refines(&top, N));
        assert!(bottom.refines(&p, N));
        assert!(!top.refines(&p, N));
        assert!(!p.refines(&bottom, N));
    }

    #[test]
    fn meet_is_idempotent() {
        for p in [
            SameVmctxPartition::Top,
            SameVmctxPartition::bottom(),
            partition(&[Some(0), Some(0), Some(1), Some(1), Some(2), None]),
            partition(&[Some(3), Some(3), Some(3), Some(3), Some(3), Some(3)]),
        ] {
            assert_eq!(meet(&p, &p).blocks(N), p.blocks(N));
        }
    }

    #[test]
    fn meet_laws_hold_on_random_partitions() -> CheckResult<Vec<u8>> {
        let mutator = m::default::<Vec<u8>>().map(|_ctx, bytes| {
            bytes.truncate(3 * (1 + N as usize));
            Ok(())
        });

        property_check().run_with(mutator, [Vec::new()], |bytes| {
            let (a, b, c) = decode3(bytes);

            // Commutative.
            let ab = meet(&a, &b);
            let ba = meet(&b, &a);
            assert_eq!(ab.blocks(N), ba.blocks(N), "meet is not commutative");

            // Associative.
            let ab_c = meet(&ab, &c);
            let a_bc = meet(&a, &meet(&b, &c));
            assert_eq!(ab_c.blocks(N), a_bc.blocks(N), "meet is not associative");

            // Idempotent.
            assert_eq!(
                meet(&a, &a).blocks(N),
                a.blocks(N),
                "meet is not idempotent"
            );

            // Agrees with the naive definition.
            assert_eq!(
                ab.blocks(N),
                reference_meet(&a, &b),
                "meet disagrees with the reference meet",
            );

            // A lower bound of both operands.
            assert!(ab.refines(&a, N), "meet does not refine its left operand");
            assert!(ab.refines(&b, N), "meet does not refine its right operand");

            // And the *greatest* one: anything below both is below it.
            if c.refines(&a, N) && c.refines(&b, N) {
                assert!(
                    c.refines(&ab, N),
                    "meet is not the greatest lower bound of its operands",
                );
            }

            // Monotone: `ab <= a`, so meeting both with `c` preserves that
            // ordering.
            assert!(
                meet(&ab, &c).refines(&meet(&a, &c), N),
                "meet is not monotone in its left argument",
            );
            assert!(
                meet(&c, &ab).refines(&meet(&c, &a), N),
                "meet is not monotone in its right argument",
            );

            // Refinement and meet define the same order.
            assert_eq!(
                a.refines(&b, N),
                ab.blocks(N) == a.blocks(N),
                "`a <= b` and `a /\\ b == a` disagree",
            );

            Ok::<_, String>(())
        })
    }

    #[test]
    fn representatives_are_least_members_on_random_partitions() -> CheckResult<Vec<u8>> {
        let mutator = m::default::<Vec<u8>>().map(|_ctx, bytes| {
            bytes.truncate(1 + N as usize);
            Ok(())
        });

        property_check().run_with(mutator, [Vec::new()], |bytes| {
            let p = decode(bytes);
            for block in p.blocks(N) {
                let least = *block.first().unwrap();
                for i in block {
                    assert_eq!(
                        p.representative(FuncIndex::from_u32(i)).as_u32(),
                        least,
                        "every member of a block must name the same representative",
                    );
                }
            }
            Ok::<_, String>(())
        })
    }
}
