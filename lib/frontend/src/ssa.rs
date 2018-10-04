//! A SSA-building API that handles incomplete CFGs.
//!
//! The algorithm is based upon Braun M., Buchwald S., Hack S., Leißa R., Mallon C.,
//! Zwinkau A. (2013) Simple and Efficient Construction of Static Single Assignment Form.
//! In: Jhala R., De Bosschere K. (eds) Compiler Construction. CC 2013.
//! Lecture Notes in Computer Science, vol 7791. Springer, Berlin, Heidelberg

use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::entity::{EntityRef, PrimaryMap, SecondaryMap};
use cranelift_codegen::ir::immediates::{Ieee32, Ieee64};
use cranelift_codegen::ir::instructions::BranchInfo;
use cranelift_codegen::ir::types::{F32, F64};
use cranelift_codegen::ir::{Ebb, Function, Inst, InstBuilder, InstructionData, Type, Value};
use cranelift_codegen::packed_option::PackedOption;
use cranelift_codegen::packed_option::ReservedValue;
use std::mem;
use std::u32;
use std::vec::Vec;
use Variable;

/// Structure containing the data relevant the construction of SSA for a given function.
///
/// The parameter struct `Variable` corresponds to the way variables are represented in the
/// non-SSA language you're translating from.
///
/// The SSA building relies on information about the variables used and defined, as well as
/// their position relative to basic blocks which are stricter than extended basic blocks since
/// they don't allow branching in the middle of them.
///
/// This SSA building module allows you to def and use variables on the fly while you are
/// constructing the CFG, no need for a separate SSA pass after the CFG is completed.
///
/// A basic block is said _filled_ if all the instruction that it contains have been translated,
/// and it is said _sealed_ if all of its predecessors have been declared. Only filled predecessors
/// can be declared.
pub struct SSABuilder {
    // Records for every variable and for every relevant block, the last definition of
    // the variable in the block.
    // TODO: Consider a sparse representation rather than SecondaryMap-of-SecondaryMap.
    variables: SecondaryMap<Variable, SecondaryMap<Block, PackedOption<Value>>>,
    // Records the position of the basic blocks and the list of values used but not defined in the
    // block.
    blocks: PrimaryMap<Block, BlockData>,
    // Records the basic blocks at the beginning of the `Ebb`s.
    ebb_headers: SecondaryMap<Ebb, PackedOption<Block>>,

    // Call and result stacks for use in the `use_var`/`predecessors_lookup` state machine.
    calls: Vec<Call>,
    results: Vec<Value>,
    // Side effects accumulated in the `use_var`/`predecessors_lookup` state machine.
    side_effects: SideEffects,
}

/// Side effects of a `use_var` or a `seal_ebb_header_block` method call.
pub struct SideEffects {
    /// When we want to append jump arguments to a `br_table` instruction, the critical edge is
    /// splitted and the newly created `Ebb`s are signaled here.
    pub split_ebbs_created: Vec<Ebb>,
    /// When a variable is used but has never been defined before (this happens in the case of
    /// unreachable code), a placeholder `iconst` or `fconst` value is added to the right `Ebb`.
    /// This field signals if it is the case and return the `Ebb` to which the initialization has
    /// been added.
    pub instructions_added_to_ebbs: Vec<Ebb>,
}

impl SideEffects {
    fn new() -> Self {
        Self {
            split_ebbs_created: Vec::new(),
            instructions_added_to_ebbs: Vec::new(),
        }
    }

    fn is_empty(&self) -> bool {
        self.split_ebbs_created.is_empty() && self.instructions_added_to_ebbs.is_empty()
    }
}

/// Describes the current position of a basic block in the control flow graph.
enum BlockData {
    /// A block at the top of an `Ebb`.
    EbbHeader(EbbHeaderBlockData),
    /// A block inside an `Ebb` with an unique other block as its predecessor.
    /// The block is implicitly sealed at creation.
    EbbBody { predecessor: Block },
}

impl BlockData {
    fn add_predecessor(&mut self, pred: Block, inst: Inst) {
        match *self {
            BlockData::EbbBody { .. } => panic!("you can't add a predecessor to a body block"),
            BlockData::EbbHeader(ref mut data) => {
                debug_assert!(!data.sealed, "sealed blocks cannot accept new predecessors");
                data.predecessors.push(PredBlock::new(pred, inst));
            }
        }
    }
    fn remove_predecessor(&mut self, inst: Inst) -> Block {
        match *self {
            BlockData::EbbBody { .. } => panic!("should not happen"),
            BlockData::EbbHeader(ref mut data) => {
                // This a linear complexity operation but the number of predecessors is low
                // in all non-pathological cases
                let pred: usize = data
                    .predecessors
                    .iter()
                    .position(|&PredBlock { branch, .. }| branch == inst)
                    .expect("the predecessor you are trying to remove is not declared");
                data.predecessors.swap_remove(pred).block
            }
        }
    }
}

struct PredBlock {
    block: Block,
    branch: Inst,
}

impl PredBlock {
    fn new(block: Block, branch: Inst) -> Self {
        Self { block, branch }
    }
}

struct EbbHeaderBlockData {
    // The predecessors of the Ebb header block, with the block and branch instruction.
    predecessors: Vec<PredBlock>,
    // A ebb header block is sealed if all of its predecessors have been declared.
    sealed: bool,
    // The ebb which this block is part of.
    ebb: Ebb,
    // List of current Ebb arguments for which an earlier def has not been found yet.
    undef_variables: Vec<(Variable, Value)>,
}

/// A opaque reference to a basic block.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Block(u32);
impl EntityRef for Block {
    fn new(index: usize) -> Self {
        debug_assert!(index < (u32::MAX as usize));
        Block(index as u32)
    }

    fn index(self) -> usize {
        self.0 as usize
    }
}

impl ReservedValue for Block {
    fn reserved_value() -> Self {
        Block(u32::MAX)
    }
}

impl SSABuilder {
    /// Allocate a new blank SSA builder struct. Use the API function to interact with the struct.
    pub fn new() -> Self {
        Self {
            variables: SecondaryMap::with_default(SecondaryMap::new()),
            blocks: PrimaryMap::new(),
            ebb_headers: SecondaryMap::new(),
            calls: Vec::new(),
            results: Vec::new(),
            side_effects: SideEffects::new(),
        }
    }

    /// Clears a `SSABuilder` from all its data, letting it in a pristine state without
    /// deallocating memory.
    pub fn clear(&mut self) {
        self.variables.clear();
        self.blocks.clear();
        self.ebb_headers.clear();
        debug_assert!(self.calls.is_empty());
        debug_assert!(self.results.is_empty());
        debug_assert!(self.side_effects.is_empty());
    }

    /// Tests whether an `SSABuilder` is in a cleared state.
    pub fn is_empty(&self) -> bool {
        self.variables.is_empty()
            && self.blocks.is_empty()
            && self.ebb_headers.is_empty()
            && self.calls.is_empty()
            && self.results.is_empty()
            && self.side_effects.is_empty()
    }
}

/// Small enum used for clarity in some functions.
#[derive(Debug)]
enum ZeroOneOrMore<T> {
    Zero(),
    One(T),
    More(),
}

#[derive(Debug)]
enum UseVarCases {
    Unsealed(Value),
    SealedOnePredecessor(Block),
    SealedMultiplePredecessors(Value, Ebb),
}

/// States for the `use_var`/`predecessors_lookup` state machine.
enum Call {
    UseVar(Block),
    FinishSealedOnePredecessor(Block),
    FinishPredecessorsLookup(Value, Ebb),
}

/// Emit instructions to produce a zero value in the given type.
fn emit_zero(ty: Type, mut cur: FuncCursor) -> Value {
    if ty.is_int() {
        cur.ins().iconst(ty, 0)
    } else if ty.is_bool() {
        cur.ins().bconst(ty, false)
    } else if ty == F32 {
        cur.ins().f32const(Ieee32::with_bits(0))
    } else if ty == F64 {
        cur.ins().f64const(Ieee64::with_bits(0))
    } else if ty.is_vector() {
        let scalar_ty = ty.lane_type();
        if scalar_ty.is_int() {
            cur.ins().iconst(ty, 0)
        } else if scalar_ty.is_bool() {
            cur.ins().bconst(ty, false)
        } else if scalar_ty == F32 {
            let scalar = cur.ins().f32const(Ieee32::with_bits(0));
            cur.ins().splat(ty, scalar)
        } else if scalar_ty == F64 {
            let scalar = cur.ins().f64const(Ieee64::with_bits(0));
            cur.ins().splat(ty, scalar)
        } else {
            panic!("unimplemented scalar type: {:?}", ty)
        }
    } else {
        panic!("unimplemented type: {:?}", ty)
    }
}
/// The following methods are the API of the SSA builder. Here is how it should be used when
/// translating to Cranelift IR:
///
/// - for each sequence of contiguous instructions (with no branches), create a corresponding
///   basic block with `declare_ebb_body_block` or `declare_ebb_header_block` depending on the
///   position of the basic block;
///
/// - while traversing a basic block and translating instruction, use `def_var` and `use_var`
///   to record definitions and uses of variables, these methods will give you the corresponding
///   SSA values;
///
/// - when all the instructions in a basic block have translated, the block is said _filled_ and
///   only then you can add it as a predecessor to other blocks with `declare_ebb_predecessor`;
///
/// - when you have constructed all the predecessor to a basic block at the beginning of an `Ebb`,
///   call `seal_ebb_header_block` on it with the `Function` that you are building.
///
/// This API will give you the correct SSA values to use as arguments of your instructions,
/// as well as modify the jump instruction and `Ebb` headers parameters to account for the SSA
/// Phi functions.
///
impl SSABuilder {
    /// Declares a new definition of a variable in a given basic block.
    /// The SSA value is passed as an argument because it should be created with
    /// `ir::DataFlowGraph::append_result`.
    pub fn def_var(&mut self, var: Variable, val: Value, block: Block) {
        self.variables[var][block] = PackedOption::from(val);
    }

    /// Declares a use of a variable in a given basic block. Returns the SSA value corresponding
    /// to the current SSA definition of this variable and a list of newly created Ebbs that
    /// are the results of critical edge splitting for `br_table` with arguments.
    ///
    /// If the variable has never been defined in this blocks or recursively in its predecessors,
    /// this method will silently create an initializer with `iconst` or `fconst`. You are
    /// responsible for making sure that you initialize your variables.
    pub fn use_var(
        &mut self,
        func: &mut Function,
        var: Variable,
        ty: Type,
        block: Block,
    ) -> (Value, SideEffects) {
        // First we lookup for the current definition of the variable in this block
        if let Some(var_defs) = self.variables.get(var) {
            if let Some(val) = var_defs[block].expand() {
                return (val, SideEffects::new());
            }
        }

        // Otherwise, we have to do a non-local lookup.
        debug_assert!(self.calls.is_empty());
        debug_assert!(self.results.is_empty());
        debug_assert!(self.side_effects.is_empty());
        self.use_var_nonlocal(func, var, ty, block);
        (
            self.run_state_machine(func, var, ty),
            mem::replace(&mut self.side_effects, SideEffects::new()),
        )
    }

    /// Resolve a use of `var` in `block` in the case where there's no prior def
    /// in `block`.
    fn use_var_nonlocal(&mut self, func: &mut Function, var: Variable, ty: Type, block: Block) {
        let case = match self.blocks[block] {
            BlockData::EbbHeader(ref mut data) => {
                // The block has multiple predecessors so we append an Ebb parameter that
                // will serve as a value.
                if data.sealed {
                    if data.predecessors.len() == 1 {
                        // Only one predecessor, straightforward case
                        UseVarCases::SealedOnePredecessor(data.predecessors[0].block)
                    } else {
                        let val = func.dfg.append_ebb_param(data.ebb, ty);
                        UseVarCases::SealedMultiplePredecessors(val, data.ebb)
                    }
                } else {
                    let val = func.dfg.append_ebb_param(data.ebb, ty);
                    data.undef_variables.push((var, val));
                    UseVarCases::Unsealed(val)
                }
            }
            BlockData::EbbBody { predecessor: pred } => UseVarCases::SealedOnePredecessor(pred),
        };
        match case {
            // The block has a single predecessor, we look into it.
            UseVarCases::SealedOnePredecessor(pred) => {
                self.calls.push(Call::FinishSealedOnePredecessor(block));
                self.calls.push(Call::UseVar(pred));
            }
            // The block has multiple predecessors, we register the EBB parameter as the current
            // definition for the variable.
            UseVarCases::Unsealed(val) => {
                self.def_var(var, val, block);
                self.results.push(val);
            }
            UseVarCases::SealedMultiplePredecessors(val, ebb) => {
                // If multiple predecessor we look up a use_var in each of them:
                // if they all yield the same value no need for an EBB parameter
                self.def_var(var, val, block);
                self.begin_predecessors_lookup(val, ebb);
            }
        }
    }

    /// For blocks with a single predecessor, once we've determined the value,
    /// record a local def for it for future queries to find.
    fn finish_sealed_one_predecessor(&mut self, var: Variable, block: Block) {
        let val = *self.results.last().unwrap();
        self.def_var(var, val, block);
    }

    /// Declares a new basic block belonging to the body of a certain `Ebb` and having `pred`
    /// as a predecessor. `pred` is the only predecessor of the block and the block is sealed
    /// at creation.
    ///
    /// To declare a `Ebb` header block, see `declare_ebb_header_block`.
    pub fn declare_ebb_body_block(&mut self, pred: Block) -> Block {
        self.blocks.push(BlockData::EbbBody { predecessor: pred })
    }

    /// Declares a new basic block at the beginning of an `Ebb`. No predecessors are declared
    /// here and the block is not sealed.
    /// Predecessors have to be added with `declare_ebb_predecessor`.
    pub fn declare_ebb_header_block(&mut self, ebb: Ebb) -> Block {
        let block = self.blocks.push(BlockData::EbbHeader(EbbHeaderBlockData {
            predecessors: Vec::new(),
            sealed: false,
            ebb,
            undef_variables: Vec::new(),
        }));
        self.ebb_headers[ebb] = block.into();
        block
    }
    /// Gets the header block corresponding to an Ebb, panics if the Ebb or the header block
    /// isn't declared.
    pub fn header_block(&self, ebb: Ebb) -> Block {
        self.ebb_headers
            .get(ebb)
            .expect("the ebb has not been declared")
            .expand()
            .expect("the header block has not been defined")
    }

    /// Declares a new predecessor for an `Ebb` header block and record the branch instruction
    /// of the predecessor that leads to it.
    ///
    /// Note that the predecessor is a `Block` and not an `Ebb`. This `Block` must be filled
    /// before added as predecessor. Note that you must provide no jump arguments to the branch
    /// instruction when you create it since `SSABuilder` will fill them for you.
    ///
    /// Callers are expected to avoid adding the same predecessor more than once in the case
    /// of a jump table.
    pub fn declare_ebb_predecessor(&mut self, ebb: Ebb, pred: Block, inst: Inst) {
        debug_assert!(!self.is_sealed(ebb));
        let header_block = self.header_block(ebb);
        self.blocks[header_block].add_predecessor(pred, inst)
    }

    /// Remove a previously declared Ebb predecessor by giving a reference to the jump
    /// instruction. Returns the basic block containing the instruction.
    ///
    /// Note: use only when you know what you are doing, this might break the SSA building problem
    pub fn remove_ebb_predecessor(&mut self, ebb: Ebb, inst: Inst) -> Block {
        debug_assert!(!self.is_sealed(ebb));
        let header_block = self.header_block(ebb);
        self.blocks[header_block].remove_predecessor(inst)
    }

    /// Completes the global value numbering for an `Ebb`, all of its predecessors having been
    /// already sealed.
    ///
    /// This method modifies the function's `Layout` by adding arguments to the `Ebb`s to
    /// take into account the Phi function placed by the SSA algorithm.
    ///
    /// Returns the list of newly created ebbs for critical edge splitting.
    pub fn seal_ebb_header_block(&mut self, ebb: Ebb, func: &mut Function) -> SideEffects {
        self.seal_one_ebb_header_block(ebb, func);
        mem::replace(&mut self.side_effects, SideEffects::new())
    }

    /// Completes the global value numbering for all `Ebb`s in `func`.
    ///
    /// It's more efficient to seal `Ebb`s as soon as possible, during
    /// translation, but for frontends where this is impractical to do, this
    /// function can be used at the end of translating all blocks to ensure
    /// that everything is sealed.
    pub fn seal_all_ebb_header_blocks(&mut self, func: &mut Function) -> SideEffects {
        // Seal all `Ebb`s currently in the function. This can entail splitting
        // and creation of new blocks, however such new blocks are sealed on
        // the fly, so we don't need to account for them here.
        for ebb in self.ebb_headers.keys() {
            self.seal_one_ebb_header_block(ebb, func);
        }
        mem::replace(&mut self.side_effects, SideEffects::new())
    }

    /// Helper function for `seal_ebb_header_block` and
    /// `seal_all_ebb_header_blocks`.
    fn seal_one_ebb_header_block(&mut self, ebb: Ebb, func: &mut Function) {
        let block = self.header_block(ebb);

        let undef_vars = match self.blocks[block] {
            BlockData::EbbBody { .. } => panic!("this should not happen"),
            BlockData::EbbHeader(ref mut data) => {
                debug_assert!(
                    !data.sealed,
                    "Attempting to seal {} which is already sealed.",
                    ebb
                );
                debug_assert_eq!(ebb, data.ebb);
                // Extract the undef_variables data from the block so that we
                // can iterate over it without borrowing the whole builder.
                mem::replace(&mut data.undef_variables, Vec::new())
            }
        };

        // For each undef var we look up values in the predecessors and create an EBB parameter
        // only if necessary.
        for (var, val) in undef_vars {
            let ty = func.dfg.value_type(val);
            self.predecessors_lookup(func, val, var, ty, ebb);
        }
        self.mark_ebb_header_block_sealed(block);
    }

    /// Set the `sealed` flag for `block`.
    fn mark_ebb_header_block_sealed(&mut self, block: Block) {
        // Then we mark the block as sealed.
        match self.blocks[block] {
            BlockData::EbbBody { .. } => panic!("this should not happen"),
            BlockData::EbbHeader(ref mut data) => {
                debug_assert!(!data.sealed);
                debug_assert!(data.undef_variables.is_empty());
                data.sealed = true;
                // We could call data.predecessors.shrink_to_fit() here, if
                // important, because no further predecessors will be added
                // to this block.
            }
        }
    }

    /// Look up in the predecessors of an Ebb the def for a value an decides whether or not
    /// to keep the eeb arg, and act accordingly. Returns the chosen value and optionally a
    /// list of Ebb that are the middle of newly created critical edges splits.
    fn predecessors_lookup(
        &mut self,
        func: &mut Function,
        temp_arg_val: Value,
        temp_arg_var: Variable,
        ty: Type,
        dest_ebb: Ebb,
    ) -> Value {
        debug_assert!(self.calls.is_empty());
        debug_assert!(self.results.is_empty());
        // self.side_effects may be non-empty here so that callers can
        // accumulate side effects over multiple calls.
        self.begin_predecessors_lookup(temp_arg_val, dest_ebb);
        self.run_state_machine(func, temp_arg_var, ty)
    }

    /// Initiate use lookups in all predecessors of `dest_ebb`, and arrange for a call
    /// to `finish_predecessors_lookup` once they complete.
    fn begin_predecessors_lookup(&mut self, temp_arg_val: Value, dest_ebb: Ebb) {
        self.calls
            .push(Call::FinishPredecessorsLookup(temp_arg_val, dest_ebb));
        // Iterate over the predecessors.
        let mut calls = mem::replace(&mut self.calls, Vec::new());
        calls.extend(
            self.predecessors(dest_ebb)
                .iter()
                .rev()
                .map(|&PredBlock { block: pred, .. }| Call::UseVar(pred)),
        );
        self.calls = calls;
    }

    /// Examine the values from the predecessors and compute a result value, creating
    /// block parameters as needed.
    fn finish_predecessors_lookup(
        &mut self,
        func: &mut Function,
        temp_arg_val: Value,
        temp_arg_var: Variable,
        dest_ebb: Ebb,
    ) {
        let mut pred_values: ZeroOneOrMore<Value> = ZeroOneOrMore::Zero();

        // Iterate over the predecessors.
        for _ in 0..self.predecessors(dest_ebb).len() {
            // For each predecessor, we query what is the local SSA value corresponding
            // to var and we put it as an argument of the branch instruction.
            let pred_val = self.results.pop().unwrap();
            match pred_values {
                ZeroOneOrMore::Zero() => {
                    if pred_val != temp_arg_val {
                        pred_values = ZeroOneOrMore::One(pred_val);
                    }
                }
                ZeroOneOrMore::One(old_val) => {
                    if pred_val != temp_arg_val && pred_val != old_val {
                        pred_values = ZeroOneOrMore::More();
                    }
                }
                ZeroOneOrMore::More() => {}
            }
        }
        let result_val = match pred_values {
            ZeroOneOrMore::Zero() => {
                // The variable is used but never defined before. This is an irregularity in the
                // code, but rather than throwing an error we silently initialize the variable to
                // 0. This will have no effect since this situation happens in unreachable code.
                if !func.layout.is_ebb_inserted(dest_ebb) {
                    func.layout.append_ebb(dest_ebb);
                }
                self.side_effects.instructions_added_to_ebbs.push(dest_ebb);
                let zero = emit_zero(
                    func.dfg.value_type(temp_arg_val),
                    FuncCursor::new(func).at_first_insertion_point(dest_ebb),
                );
                func.dfg.remove_ebb_param(temp_arg_val);
                func.dfg.change_to_alias(temp_arg_val, zero);
                zero
            }
            ZeroOneOrMore::One(pred_val) => {
                // Here all the predecessors use a single value to represent our variable
                // so we don't need to have it as an ebb argument.
                // We need to replace all the occurrences of val with pred_val but since
                // we can't afford a re-writing pass right now we just declare an alias.
                // Resolve aliases eagerly so that we can check for cyclic aliasing,
                // which can occur in unreachable code.
                let mut resolved = func.dfg.resolve_aliases(pred_val);
                if temp_arg_val == resolved {
                    // Cycle detected. Break it by creating a zero value.
                    resolved = emit_zero(
                        func.dfg.value_type(temp_arg_val),
                        FuncCursor::new(func).at_first_insertion_point(dest_ebb),
                    );
                }
                func.dfg.remove_ebb_param(temp_arg_val);
                func.dfg.change_to_alias(temp_arg_val, resolved);
                resolved
            }
            ZeroOneOrMore::More() => {
                // There is disagreement in the predecessors on which value to use so we have
                // to keep the ebb argument. To avoid borrowing `self` for the whole loop,
                // temporarily detach the predecessors list and replace it with an empty list.
                let mut preds = mem::replace(self.predecessors_mut(dest_ebb), Vec::new());
                for &mut PredBlock {
                    block: ref mut pred_block,
                    branch: ref mut last_inst,
                } in &mut preds
                {
                    // We already did a full `use_var` above, so we can do just the fast path.
                    let pred_val = self
                        .variables
                        .get(temp_arg_var)
                        .unwrap()
                        .get(*pred_block)
                        .unwrap()
                        .unwrap();
                    let jump_arg = self.append_jump_argument(
                        func,
                        *last_inst,
                        *pred_block,
                        dest_ebb,
                        pred_val,
                        temp_arg_var,
                    );
                    if let Some((middle_ebb, middle_block, middle_jump_inst)) = jump_arg {
                        *pred_block = middle_block;
                        *last_inst = middle_jump_inst;
                        self.side_effects.split_ebbs_created.push(middle_ebb);
                    }
                }
                // Now that we're done, move the predecessors list back.
                debug_assert!(self.predecessors(dest_ebb).is_empty());
                *self.predecessors_mut(dest_ebb) = preds;

                temp_arg_val
            }
        };

        self.results.push(result_val);
    }

    /// Appends a jump argument to a jump instruction, returns ebb created in case of
    /// critical edge splitting.
    fn append_jump_argument(
        &mut self,
        func: &mut Function,
        jump_inst: Inst,
        jump_inst_block: Block,
        dest_ebb: Ebb,
        val: Value,
        var: Variable,
    ) -> Option<(Ebb, Block, Inst)> {
        match func.dfg.analyze_branch(jump_inst) {
            BranchInfo::NotABranch => {
                panic!("you have declared a non-branch instruction as a predecessor to an ebb");
            }
            // For a single destination appending a jump argument to the instruction
            // is sufficient.
            BranchInfo::SingleDest(_, _) => {
                func.dfg.append_inst_arg(jump_inst, val);
                None
            }
            BranchInfo::Table(jt, default_ebb) => {
                // In the case of a jump table, the situation is tricky because br_table doesn't
                // support arguments.
                // We have to split the critical edge
                let middle_ebb = func.dfg.make_ebb();
                func.layout.append_ebb(middle_ebb);
                let middle_block = self.declare_ebb_header_block(middle_ebb);
                self.blocks[middle_block].add_predecessor(jump_inst_block, jump_inst);
                self.mark_ebb_header_block_sealed(middle_block);

                if let Some(default_ebb) = default_ebb {
                    if dest_ebb == default_ebb {
                        match func.dfg[jump_inst] {
                            InstructionData::BranchTable {
                                destination: ref mut dest,
                                ..
                            } => {
                                *dest = middle_ebb;
                            }
                            _ => panic!("should not happen"),
                        }
                    }
                }

                for old_dest in func.jump_tables[jt].as_mut_slice() {
                    if *old_dest == dest_ebb {
                        *old_dest = middle_ebb;
                    }
                }
                let mut cur = FuncCursor::new(func).at_bottom(middle_ebb);
                let middle_jump_inst = cur.ins().jump(dest_ebb, &[val]);
                self.def_var(var, val, middle_block);
                Some((middle_ebb, middle_block, middle_jump_inst))
            }
        }
    }

    /// Returns the list of `Ebb`s that have been declared as predecessors of the argument.
    fn predecessors(&self, ebb: Ebb) -> &[PredBlock] {
        let block = self.header_block(ebb);
        match self.blocks[block] {
            BlockData::EbbBody { .. } => panic!("should not happen"),
            BlockData::EbbHeader(ref data) => &data.predecessors,
        }
    }

    /// Returns whether the given Ebb has any predecessor or not.
    pub fn has_any_predecessors(&self, ebb: Ebb) -> bool {
        !self.predecessors(ebb).is_empty()
    }

    /// Same as predecessors, but for &mut.
    fn predecessors_mut(&mut self, ebb: Ebb) -> &mut Vec<PredBlock> {
        let block = self.header_block(ebb);
        match self.blocks[block] {
            BlockData::EbbBody { .. } => panic!("should not happen"),
            BlockData::EbbHeader(ref mut data) => &mut data.predecessors,
        }
    }

    /// Returns `true` if and only if `seal_ebb_header_block` has been called on the argument.
    pub fn is_sealed(&self, ebb: Ebb) -> bool {
        match self.blocks[self.header_block(ebb)] {
            BlockData::EbbBody { .. } => panic!("should not happen"),
            BlockData::EbbHeader(ref data) => data.sealed,
        }
    }

    /// The main algorithm is naturally recursive: when there's a `use_var` in a
    /// block with no corresponding local defs, it recurses and performs a
    /// `use_var` in each predecessor. To avoid risking running out of callstack
    /// space, we keep an explicit stack and use a small state machine rather
    /// than literal recursion.
    fn run_state_machine(&mut self, func: &mut Function, var: Variable, ty: Type) -> Value {
        // Process the calls scheduled in `self.calls` until it is empty.
        while let Some(call) = self.calls.pop() {
            match call {
                Call::UseVar(block) => {
                    // First we lookup for the current definition of the variable in this block
                    if let Some(var_defs) = self.variables.get(var) {
                        if let Some(val) = var_defs[block].expand() {
                            self.results.push(val);
                            continue;
                        }
                    }
                    self.use_var_nonlocal(func, var, ty, block);
                }
                Call::FinishSealedOnePredecessor(block) => {
                    self.finish_sealed_one_predecessor(var, block);
                }
                Call::FinishPredecessorsLookup(temp_arg_val, dest_ebb) => {
                    self.finish_predecessors_lookup(func, temp_arg_val, var, dest_ebb);
                }
            }
        }
        debug_assert_eq!(self.results.len(), 1);
        self.results.pop().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use cranelift_codegen::cursor::{Cursor, FuncCursor};
    use cranelift_codegen::entity::EntityRef;
    use cranelift_codegen::ir::instructions::BranchInfo;
    use cranelift_codegen::ir::types::*;
    use cranelift_codegen::ir::{Function, Inst, InstBuilder, JumpTableData, Opcode};
    use cranelift_codegen::settings;
    use cranelift_codegen::verify_function;
    use ssa::SSABuilder;
    use Variable;

    #[test]
    fn simple_block() {
        let mut func = Function::new();
        let mut ssa = SSABuilder::new();
        let ebb0 = func.dfg.make_ebb();
        // Here is the pseudo-program we want to translate:
        // x = 1;
        // y = 2;
        // z = x + y;
        // z = x + z;

        let block = ssa.declare_ebb_header_block(ebb0);
        let x_var = Variable::new(0);
        let x_ssa = {
            let mut cur = FuncCursor::new(&mut func);
            cur.insert_ebb(ebb0);
            cur.ins().iconst(I32, 1)
        };
        ssa.def_var(x_var, x_ssa, block);
        let y_var = Variable::new(1);
        let y_ssa = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(ebb0);
            cur.ins().iconst(I32, 2)
        };
        ssa.def_var(y_var, y_ssa, block);

        assert_eq!(ssa.use_var(&mut func, x_var, I32, block).0, x_ssa);
        assert_eq!(ssa.use_var(&mut func, y_var, I32, block).0, y_ssa);
        let z_var = Variable::new(2);
        let x_use1 = ssa.use_var(&mut func, x_var, I32, block).0;
        let y_use1 = ssa.use_var(&mut func, y_var, I32, block).0;
        let z1_ssa = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(ebb0);
            cur.ins().iadd(x_use1, y_use1)
        };
        ssa.def_var(z_var, z1_ssa, block);
        assert_eq!(ssa.use_var(&mut func, z_var, I32, block).0, z1_ssa);
        let x_use2 = ssa.use_var(&mut func, x_var, I32, block).0;
        let z_use1 = ssa.use_var(&mut func, z_var, I32, block).0;
        let z2_ssa = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(ebb0);
            cur.ins().iadd(x_use2, z_use1)
        };
        ssa.def_var(z_var, z2_ssa, block);
        assert_eq!(ssa.use_var(&mut func, z_var, I32, block).0, z2_ssa);
    }

    #[test]
    fn sequence_of_blocks() {
        let mut func = Function::new();
        let mut ssa = SSABuilder::new();
        let ebb0 = func.dfg.make_ebb();
        let ebb1 = func.dfg.make_ebb();
        // Here is the pseudo-program we want to translate:
        // ebb0:
        //    x = 1;
        //    y = 2;
        //    z = x + y;
        //    brnz y, ebb1;
        //    z = x + z;
        // ebb1:
        //    y = x + y;

        let block0 = ssa.declare_ebb_header_block(ebb0);
        let x_var = Variable::new(0);
        let x_ssa = {
            let mut cur = FuncCursor::new(&mut func);
            cur.insert_ebb(ebb0);
            cur.insert_ebb(ebb1);
            cur.goto_bottom(ebb0);
            cur.ins().iconst(I32, 1)
        };
        ssa.def_var(x_var, x_ssa, block0);
        let y_var = Variable::new(1);
        let y_ssa = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(ebb0);
            cur.ins().iconst(I32, 2)
        };
        ssa.def_var(y_var, y_ssa, block0);
        assert_eq!(ssa.use_var(&mut func, x_var, I32, block0).0, x_ssa);
        assert_eq!(ssa.use_var(&mut func, y_var, I32, block0).0, y_ssa);
        let z_var = Variable::new(2);
        let x_use1 = ssa.use_var(&mut func, x_var, I32, block0).0;
        let y_use1 = ssa.use_var(&mut func, y_var, I32, block0).0;
        let z1_ssa = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(ebb0);
            cur.ins().iadd(x_use1, y_use1)
        };
        ssa.def_var(z_var, z1_ssa, block0);
        assert_eq!(ssa.use_var(&mut func, z_var, I32, block0).0, z1_ssa);
        let y_use2 = ssa.use_var(&mut func, y_var, I32, block0).0;
        let jump_inst: Inst = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(ebb0);
            cur.ins().brnz(y_use2, ebb1, &[])
        };
        let block1 = ssa.declare_ebb_body_block(block0);
        let x_use2 = ssa.use_var(&mut func, x_var, I32, block1).0;
        assert_eq!(x_use2, x_ssa);
        let z_use1 = ssa.use_var(&mut func, z_var, I32, block1).0;
        assert_eq!(z_use1, z1_ssa);
        let z2_ssa = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(ebb0);
            cur.ins().iadd(x_use2, z_use1)
        };
        ssa.def_var(z_var, z2_ssa, block1);
        assert_eq!(ssa.use_var(&mut func, z_var, I32, block1).0, z2_ssa);
        ssa.seal_ebb_header_block(ebb0, &mut func);
        let block2 = ssa.declare_ebb_header_block(ebb1);
        ssa.declare_ebb_predecessor(ebb1, block0, jump_inst);
        ssa.seal_ebb_header_block(ebb1, &mut func);
        let x_use3 = ssa.use_var(&mut func, x_var, I32, block2).0;
        assert_eq!(x_ssa, x_use3);
        let y_use3 = ssa.use_var(&mut func, y_var, I32, block2).0;
        assert_eq!(y_ssa, y_use3);
        let y2_ssa = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(ebb0);
            cur.ins().iadd(x_use3, y_use3)
        };
        ssa.def_var(y_var, y2_ssa, block2);
        match func.dfg.analyze_branch(jump_inst) {
            BranchInfo::SingleDest(dest, jump_args) => {
                assert_eq!(dest, ebb1);
                assert_eq!(jump_args.len(), 0);
            }
            _ => assert!(false),
        };
    }

    #[test]
    fn program_with_loop() {
        let mut func = Function::new();
        let mut ssa = SSABuilder::new();
        let ebb0 = func.dfg.make_ebb();
        let ebb1 = func.dfg.make_ebb();
        let ebb2 = func.dfg.make_ebb();
        // Here is the pseudo-program we want to translate:
        // ebb0:
        //    x = 1;
        //    y = 2;
        //    z = x + y;
        //    jump ebb1
        // ebb1:
        //    z = z + y;
        //    brnz y, ebb1;
        //    z = z - x;
        //    return y
        // ebb2:
        //    y = y - x
        //    jump ebb1

        let block0 = ssa.declare_ebb_header_block(ebb0);
        ssa.seal_ebb_header_block(ebb0, &mut func);
        let x_var = Variable::new(0);
        let x1 = {
            let mut cur = FuncCursor::new(&mut func);
            cur.insert_ebb(ebb0);
            cur.insert_ebb(ebb1);
            cur.insert_ebb(ebb2);
            cur.goto_bottom(ebb0);
            cur.ins().iconst(I32, 1)
        };
        ssa.def_var(x_var, x1, block0);
        assert_eq!(ssa.use_var(&mut func, x_var, I32, block0).0, x1);
        let y_var = Variable::new(1);
        let y1 = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(ebb0);
            cur.ins().iconst(I32, 2)
        };
        ssa.def_var(y_var, y1, block0);
        assert_eq!(ssa.use_var(&mut func, y_var, I32, block0).0, y1);
        let z_var = Variable::new(2);
        let x2 = ssa.use_var(&mut func, x_var, I32, block0).0;
        assert_eq!(x2, x1);
        let y2 = ssa.use_var(&mut func, y_var, I32, block0).0;
        assert_eq!(y2, y1);
        let z1 = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(ebb0);
            cur.ins().iadd(x2, y2)
        };
        ssa.def_var(z_var, z1, block0);
        let jump_ebb0_ebb1 = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(ebb0);
            cur.ins().jump(ebb1, &[])
        };
        let block1 = ssa.declare_ebb_header_block(ebb1);
        ssa.declare_ebb_predecessor(ebb1, block0, jump_ebb0_ebb1);
        let z2 = ssa.use_var(&mut func, z_var, I32, block1).0;
        let y3 = ssa.use_var(&mut func, y_var, I32, block1).0;
        let z3 = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(ebb1);
            cur.ins().iadd(z2, y3)
        };
        ssa.def_var(z_var, z3, block1);
        let y4 = ssa.use_var(&mut func, y_var, I32, block1).0;
        assert_eq!(y4, y3);
        let jump_ebb1_ebb2 = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(ebb1);
            cur.ins().brnz(y4, ebb2, &[])
        };
        let block2 = ssa.declare_ebb_body_block(block1);
        let z4 = ssa.use_var(&mut func, z_var, I32, block2).0;
        assert_eq!(z4, z3);
        let x3 = ssa.use_var(&mut func, x_var, I32, block2).0;
        let z5 = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(ebb1);
            cur.ins().isub(z4, x3)
        };
        ssa.def_var(z_var, z5, block2);
        let y5 = ssa.use_var(&mut func, y_var, I32, block2).0;
        assert_eq!(y5, y3);
        {
            let mut cur = FuncCursor::new(&mut func).at_bottom(ebb1);
            cur.ins().return_(&[y5])
        };

        let block3 = ssa.declare_ebb_header_block(ebb2);
        ssa.declare_ebb_predecessor(ebb2, block1, jump_ebb1_ebb2);
        ssa.seal_ebb_header_block(ebb2, &mut func);
        let y6 = ssa.use_var(&mut func, y_var, I32, block3).0;
        assert_eq!(y6, y3);
        let x4 = ssa.use_var(&mut func, x_var, I32, block3).0;
        assert_eq!(x4, x3);
        let y7 = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(ebb2);
            cur.ins().isub(y6, x4)
        };
        ssa.def_var(y_var, y7, block3);
        let jump_ebb2_ebb1 = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(ebb2);
            cur.ins().jump(ebb1, &[])
        };

        ssa.declare_ebb_predecessor(ebb1, block3, jump_ebb2_ebb1);
        ssa.seal_ebb_header_block(ebb1, &mut func);
        assert_eq!(func.dfg.ebb_params(ebb1)[0], z2);
        assert_eq!(func.dfg.ebb_params(ebb1)[1], y3);
        assert_eq!(func.dfg.resolve_aliases(x3), x1);
    }

    #[test]
    fn br_table_with_args() {
        // This tests the on-demand splitting of critical edges for br_table with jump arguments
        //
        // Here is the pseudo-program we want to translate:
        //
        // function %f {
        // jt = jump_table [ebb2, ebb1]
        // ebb0:
        //    x = 1;
        //    br_table x, ebb2, jt
        // ebb1:
        //    x = 2
        //    jump ebb2
        // ebb2:
        //    x = x + 1
        //    return
        // }

        let mut func = Function::new();
        let mut ssa = SSABuilder::new();
        let mut jump_table = JumpTableData::new();
        let ebb0 = func.dfg.make_ebb();
        let ebb1 = func.dfg.make_ebb();
        let ebb2 = func.dfg.make_ebb();

        // ebb0:
        //    x = 1;
        let block0 = ssa.declare_ebb_header_block(ebb0);
        ssa.seal_ebb_header_block(ebb0, &mut func);
        let x_var = Variable::new(0);
        let x1 = {
            let mut cur = FuncCursor::new(&mut func);
            cur.insert_ebb(ebb0);
            cur.insert_ebb(ebb1);
            cur.insert_ebb(ebb2);
            cur.goto_bottom(ebb0);
            cur.ins().iconst(I32, 1)
        };
        ssa.def_var(x_var, x1, block0);

        // jt = jump_table [ebb2, ebb1]
        jump_table.push_entry(ebb2);
        jump_table.push_entry(ebb1);
        let jt = func.create_jump_table(jump_table);

        // ebb0:
        //    ...
        //    br_table x, ebb2, jt
        ssa.use_var(&mut func, x_var, I32, block0).0;
        let br_table = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(ebb0);
            cur.ins().br_table(x1, ebb2, jt)
        };

        // ebb1:
        //    x = 2
        //    jump ebb2
        let block1 = ssa.declare_ebb_header_block(ebb1);
        ssa.seal_ebb_header_block(ebb1, &mut func);
        let x2 = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(ebb1);
            cur.ins().iconst(I32, 2)
        };
        ssa.def_var(x_var, x2, block1);
        let jump_inst = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(ebb1);
            cur.ins().jump(ebb2, &[])
        };

        // ebb2:
        //    x = x + 1
        //    return
        let block3 = ssa.declare_ebb_header_block(ebb2);
        ssa.declare_ebb_predecessor(ebb2, block1, jump_inst);
        ssa.declare_ebb_predecessor(ebb2, block0, br_table);
        ssa.seal_ebb_header_block(ebb2, &mut func);
        let block4 = ssa.declare_ebb_body_block(block3);
        let x3 = ssa.use_var(&mut func, x_var, I32, block4).0;
        let x4 = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(ebb2);
            cur.ins().iadd_imm(x3, 1)
        };
        ssa.def_var(x_var, x4, block4);
        {
            let mut cur = FuncCursor::new(&mut func).at_bottom(ebb2);
            cur.ins().return_(&[])
        };

        let flags = settings::Flags::new(settings::builder());
        match verify_function(&func, &flags) {
            Ok(()) => {}
            Err(_errors) => {
                #[cfg(feature = "std")]
                panic!(_errors);
                #[cfg(not(feature = "std"))]
                panic!("function failed to verify");
            }
        }
    }

    #[test]
    fn undef_values_reordering() {
        let mut func = Function::new();
        let mut ssa = SSABuilder::new();
        let ebb0 = func.dfg.make_ebb();
        let ebb1 = func.dfg.make_ebb();
        // Here is the pseudo-program we want to translate:
        // ebb0:
        //    x = 0
        //    y = 1
        //    z = 2
        //    jump ebb1
        // ebb1:
        //    x = z + x
        //    y = y - x
        //    jump ebb1
        //
        let block0 = ssa.declare_ebb_header_block(ebb0);
        let x_var = Variable::new(0);
        let y_var = Variable::new(1);
        let z_var = Variable::new(2);
        ssa.seal_ebb_header_block(ebb0, &mut func);
        let x1 = {
            let mut cur = FuncCursor::new(&mut func);
            cur.insert_ebb(ebb0);
            cur.insert_ebb(ebb1);
            cur.goto_bottom(ebb0);
            cur.ins().iconst(I32, 0)
        };
        ssa.def_var(x_var, x1, block0);
        let y1 = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(ebb0);
            cur.ins().iconst(I32, 1)
        };
        ssa.def_var(y_var, y1, block0);
        let z1 = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(ebb0);
            cur.ins().iconst(I32, 2)
        };
        ssa.def_var(z_var, z1, block0);
        let jump_inst = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(ebb0);
            cur.ins().jump(ebb1, &[])
        };
        let block1 = ssa.declare_ebb_header_block(ebb1);
        ssa.declare_ebb_predecessor(ebb1, block0, jump_inst);
        let z2 = ssa.use_var(&mut func, z_var, I32, block1).0;
        assert_eq!(func.dfg.ebb_params(ebb1)[0], z2);
        let x2 = ssa.use_var(&mut func, x_var, I32, block1).0;
        assert_eq!(func.dfg.ebb_params(ebb1)[1], x2);
        let x3 = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(ebb1);
            cur.ins().iadd(x2, z2)
        };
        ssa.def_var(x_var, x3, block1);
        let x4 = ssa.use_var(&mut func, x_var, I32, block1).0;
        let y3 = ssa.use_var(&mut func, y_var, I32, block1).0;
        assert_eq!(func.dfg.ebb_params(ebb1)[2], y3);
        let y4 = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(ebb1);
            cur.ins().isub(y3, x4)
        };
        ssa.def_var(y_var, y4, block1);
        let jump_inst = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(ebb1);
            cur.ins().jump(ebb1, &[])
        };
        ssa.declare_ebb_predecessor(ebb1, block1, jump_inst);
        ssa.seal_ebb_header_block(ebb1, &mut func);
        // At sealing the "z" argument disappear but the remaining "x" and "y" args have to be
        // in the right order.
        assert_eq!(func.dfg.ebb_params(ebb1)[1], y3);
        assert_eq!(func.dfg.ebb_params(ebb1)[0], x2);
    }

    #[test]
    fn undef() {
        // Use vars of various types which have not been defined.
        let mut func = Function::new();
        let mut ssa = SSABuilder::new();
        let ebb0 = func.dfg.make_ebb();
        let block = ssa.declare_ebb_header_block(ebb0);
        ssa.seal_ebb_header_block(ebb0, &mut func);
        let i32_var = Variable::new(0);
        let f32_var = Variable::new(1);
        let f64_var = Variable::new(2);
        let b1_var = Variable::new(3);
        let f32x4_var = Variable::new(4);
        ssa.use_var(&mut func, i32_var, I32, block);
        ssa.use_var(&mut func, f32_var, F32, block);
        ssa.use_var(&mut func, f64_var, F64, block);
        ssa.use_var(&mut func, b1_var, B1, block);
        ssa.use_var(&mut func, f32x4_var, F32X4, block);
        assert_eq!(func.dfg.num_ebb_params(ebb0), 0);
    }

    #[test]
    fn undef_in_entry() {
        // Use a var which has not been defined. The search should hit the
        // top of the entry block, and then fall back to inserting an iconst.
        let mut func = Function::new();
        let mut ssa = SSABuilder::new();
        let ebb0 = func.dfg.make_ebb();
        let block = ssa.declare_ebb_header_block(ebb0);
        ssa.seal_ebb_header_block(ebb0, &mut func);
        let x_var = Variable::new(0);
        assert_eq!(func.dfg.num_ebb_params(ebb0), 0);
        ssa.use_var(&mut func, x_var, I32, block);
        assert_eq!(func.dfg.num_ebb_params(ebb0), 0);
        assert_eq!(
            func.dfg[func.layout.first_inst(ebb0).unwrap()].opcode(),
            Opcode::Iconst
        );
    }

    #[test]
    fn undef_in_entry_sealed_after() {
        // Use a var which has not been defined, but the block is not sealed
        // until afterward. Before sealing, the SSA builder should insert an
        // ebb param; after sealing, it should be removed.
        let mut func = Function::new();
        let mut ssa = SSABuilder::new();
        let ebb0 = func.dfg.make_ebb();
        let block = ssa.declare_ebb_header_block(ebb0);
        let x_var = Variable::new(0);
        assert_eq!(func.dfg.num_ebb_params(ebb0), 0);
        ssa.use_var(&mut func, x_var, I32, block);
        assert_eq!(func.dfg.num_ebb_params(ebb0), 1);
        ssa.seal_ebb_header_block(ebb0, &mut func);
        assert_eq!(func.dfg.num_ebb_params(ebb0), 0);
        assert_eq!(
            func.dfg[func.layout.first_inst(ebb0).unwrap()].opcode(),
            Opcode::Iconst
        );
    }

    #[test]
    fn unreachable_use() {
        let mut func = Function::new();
        let mut ssa = SSABuilder::new();
        let ebb0 = func.dfg.make_ebb();
        let ebb1 = func.dfg.make_ebb();
        // Here is the pseudo-program we want to translate:
        // ebb0:
        //    return
        // ebb1:
        //    brz v1, ebb1
        //    jump ebb1
        let _block0 = ssa.declare_ebb_header_block(ebb0);
        ssa.seal_ebb_header_block(ebb0, &mut func);
        let block1 = ssa.declare_ebb_header_block(ebb1);
        let block2 = ssa.declare_ebb_body_block(block1);
        {
            let mut cur = FuncCursor::new(&mut func);
            cur.insert_ebb(ebb0);
            cur.insert_ebb(ebb1);
            cur.goto_bottom(ebb0);
            cur.ins().return_(&[]);
            let x_var = Variable::new(0);
            cur.goto_bottom(ebb1);
            let val = ssa.use_var(&mut cur.func, x_var, I32, block1).0;
            let brz = cur.ins().brz(val, ebb1, &[]);
            ssa.declare_ebb_predecessor(ebb1, block1, brz);
            let j = cur.ins().jump(ebb1, &[]);
            ssa.declare_ebb_predecessor(ebb1, block2, j);
        }
        ssa.seal_ebb_header_block(ebb1, &mut func);
        let flags = settings::Flags::new(settings::builder());
        match verify_function(&func, &flags) {
            Ok(()) => {}
            Err(_errors) => {
                #[cfg(feature = "std")]
                panic!(_errors);
                #[cfg(not(feature = "std"))]
                panic!("function failed to verify");
            }
        }
    }

    #[test]
    fn unreachable_use_with_multiple_preds() {
        let mut func = Function::new();
        let mut ssa = SSABuilder::new();
        let ebb0 = func.dfg.make_ebb();
        let ebb1 = func.dfg.make_ebb();
        let ebb2 = func.dfg.make_ebb();
        // Here is the pseudo-program we want to translate:
        // ebb0:
        //    return
        // ebb1:
        //    brz v1, ebb2
        //    jump ebb1
        // ebb2:
        //    jump ebb1
        let _block0 = ssa.declare_ebb_header_block(ebb0);
        ssa.seal_ebb_header_block(ebb0, &mut func);
        let block1 = ssa.declare_ebb_header_block(ebb1);
        let block2 = ssa.declare_ebb_header_block(ebb2);
        {
            let mut cur = FuncCursor::new(&mut func);
            let x_var = Variable::new(0);
            cur.insert_ebb(ebb0);
            cur.insert_ebb(ebb1);
            cur.insert_ebb(ebb2);
            cur.goto_bottom(ebb0);
            cur.ins().return_(&[]);
            cur.goto_bottom(ebb1);
            let v = ssa.use_var(&mut cur.func, x_var, I32, block1).0;
            let brz = cur.ins().brz(v, ebb2, &[]);
            let j0 = cur.ins().jump(ebb1, &[]);
            cur.goto_bottom(ebb2);
            let j1 = cur.ins().jump(ebb1, &[]);
            ssa.declare_ebb_predecessor(ebb1, block2, brz);
            ssa.declare_ebb_predecessor(ebb1, block1, j0);
            ssa.declare_ebb_predecessor(ebb2, block1, j1);
        }
        ssa.seal_ebb_header_block(ebb1, &mut func);
        ssa.seal_ebb_header_block(ebb2, &mut func);
        let flags = settings::Flags::new(settings::builder());
        match verify_function(&func, &flags) {
            Ok(()) => {}
            Err(_errors) => {
                #[cfg(feature = "std")]
                panic!(_errors);
                #[cfg(not(feature = "std"))]
                panic!("function failed to verify");
            }
        }
    }
}
