//! A SSA-building API that handles incomplete CFGs.
//!
//! The algorithm is based upon Braun M., Buchwald S., Hack S., Leißa R., Mallon C.,
//! Zwinkau A. (2013) Simple and Efficient Construction of Static Single Assignment Form.
//! In: Jhala R., De Bosschere K. (eds) Compiler Construction. CC 2013.
//! Lecture Notes in Computer Science, vol 7791. Springer, Berlin, Heidelberg
//!
//! https://link.springer.com/content/pdf/10.1007/978-3-642-37051-9_6.pdf

use crate::Variable;
use alloc::vec::Vec;
use core::mem;
use core::u32;
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::entity::{EntityRef, PrimaryMap, SecondaryMap};
use cranelift_codegen::ir::immediates::{Ieee32, Ieee64};
use cranelift_codegen::ir::instructions::BranchInfo;
use cranelift_codegen::ir::types::{F32, F64};
use cranelift_codegen::ir::{Block, Function, Inst, InstBuilder, InstructionData, Type, Value};
use cranelift_codegen::packed_option::PackedOption;
use cranelift_codegen::packed_option::ReservedValue;
use smallvec::SmallVec;

/// Structure containing the data relevant the construction of SSA for a given function.
///
/// The parameter struct `Variable` corresponds to the way variables are represented in the
/// non-SSA language you're translating from.
///
/// The SSA building relies on information about the variables used and defined, as well as
/// their position relative to basic blocks which are stricter tha basic blocks since
/// they don't allow branching in the middle of them.
///
/// This SSA building module allows you to def and use variables on the fly while you are
/// constructing the CFG, no need for a separate SSA pass after the CFG is completed.
///
/// A basic block is said _filled_ if all the instruction that it contains have been translated,
/// and it is said _sealed_ if all of its predecessors have been declared. Only filled predecessors
/// can be declared.
pub struct SSABuilder {
    // TODO: Consider a sparse representation rather than SecondaryMap-of-SecondaryMap.
    /// Records for every variable and for every relevant block, the last definition of
    /// the variable in the block.
    variables: SecondaryMap<Variable, SecondaryMap<SSABlock, PackedOption<Value>>>,

    /// Records the position of the basic blocks and the list of values used but not defined in the
    /// block.
    ssa_blocks: PrimaryMap<SSABlock, SSABlockData>,

    /// Records the basic blocks at the beginning of the `Block`s.
    block_headers: SecondaryMap<Block, PackedOption<SSABlock>>,

    /// Call stack for use in the `use_var`/`predecessors_lookup` state machine.
    calls: Vec<Call>,
    /// Result stack for use in the `use_var`/`predecessors_lookup` state machine.
    results: Vec<Value>,

    /// Side effects accumulated in the `use_var`/`predecessors_lookup` state machine.
    side_effects: SideEffects,
}

/// Side effects of a `use_var` or a `seal_block_header_block` method call.
pub struct SideEffects {
    /// When we want to append jump arguments to a `br_table` instruction, the critical edge is
    /// splitted and the newly created `Block`s are signaled here.
    pub split_blocks_created: Vec<Block>,
    /// When a variable is used but has never been defined before (this happens in the case of
    /// unreachable code), a placeholder `iconst` or `fconst` value is added to the right `Block`.
    /// This field signals if it is the case and return the `Block` to which the initialization has
    /// been added.
    pub instructions_added_to_blocks: Vec<Block>,
}

impl SideEffects {
    fn new() -> Self {
        Self {
            split_blocks_created: Vec::new(),
            instructions_added_to_blocks: Vec::new(),
        }
    }

    fn is_empty(&self) -> bool {
        self.split_blocks_created.is_empty() && self.instructions_added_to_blocks.is_empty()
    }
}

/// Describes the current position of a basic block in the control flow graph.
enum SSABlockData {
    /// A block at the top of a `Block`.
    BlockHeader(BlockHeaderSSABlockData),
    /// A block inside a `Block` with an unique other block as its predecessor.
    /// The block is implicitly sealed at creation.
    BlockBody { ssa_pred: SSABlock },
}

impl SSABlockData {
    fn add_predecessor(&mut self, ssa_pred: SSABlock, inst: Inst) {
        match *self {
            Self::BlockBody { .. } => panic!("you can't add a predecessor to a body block"),
            Self::BlockHeader(ref mut data) => {
                debug_assert!(!data.sealed, "sealed blocks cannot accept new predecessors");
                data.predecessors.push(PredBlock::new(ssa_pred, inst));
            }
        }
    }
    fn remove_predecessor(&mut self, inst: Inst) -> SSABlock {
        match *self {
            Self::BlockBody { .. } => panic!("should not happen"),
            Self::BlockHeader(ref mut data) => {
                // This a linear complexity operation but the number of predecessors is low
                // in all non-pathological cases
                let pred: usize = data
                    .predecessors
                    .iter()
                    .position(|&PredBlock { branch, .. }| branch == inst)
                    .expect("the predecessor you are trying to remove is not declared");
                data.predecessors.swap_remove(pred).ssa_block
            }
        }
    }
}

struct PredBlock {
    ssa_block: SSABlock,
    branch: Inst,
}

impl PredBlock {
    fn new(ssa_block: SSABlock, branch: Inst) -> Self {
        Self { ssa_block, branch }
    }
}

type PredBlockSmallVec = SmallVec<[PredBlock; 4]>;

struct BlockHeaderSSABlockData {
    // The predecessors of the Block header block, with the block and branch instruction.
    predecessors: PredBlockSmallVec,
    // A block header block is sealed if all of its predecessors have been declared.
    sealed: bool,
    // The block which this block is part of.
    block: Block,
    // List of current Block arguments for which an earlier def has not been found yet.
    undef_variables: Vec<(Variable, Value)>,
}

/// A opaque reference to a basic block.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct SSABlock(u32);
impl EntityRef for SSABlock {
    fn new(index: usize) -> Self {
        debug_assert!(index < (u32::MAX as usize));
        Self(index as u32)
    }

    fn index(self) -> usize {
        self.0 as usize
    }
}

impl ReservedValue for SSABlock {
    fn reserved_value() -> Self {
        Self(u32::MAX)
    }
}

impl SSABuilder {
    /// Allocate a new blank SSA builder struct. Use the API function to interact with the struct.
    pub fn new() -> Self {
        Self {
            variables: SecondaryMap::with_default(SecondaryMap::new()),
            ssa_blocks: PrimaryMap::new(),
            block_headers: SecondaryMap::new(),
            calls: Vec::new(),
            results: Vec::new(),
            side_effects: SideEffects::new(),
        }
    }

    /// Clears a `SSABuilder` from all its data, letting it in a pristine state without
    /// deallocating memory.
    pub fn clear(&mut self) {
        self.variables.clear();
        self.ssa_blocks.clear();
        self.block_headers.clear();
        debug_assert!(self.calls.is_empty());
        debug_assert!(self.results.is_empty());
        debug_assert!(self.side_effects.is_empty());
    }

    /// Tests whether an `SSABuilder` is in a cleared state.
    pub fn is_empty(&self) -> bool {
        self.variables.is_empty()
            && self.ssa_blocks.is_empty()
            && self.block_headers.is_empty()
            && self.calls.is_empty()
            && self.results.is_empty()
            && self.side_effects.is_empty()
    }
}

/// Small enum used for clarity in some functions.
#[derive(Debug)]
enum ZeroOneOrMore<T> {
    Zero,
    One(T),
    More,
}

/// Cases used internally by `use_var_nonlocal()` for avoiding the borrow checker.
#[derive(Debug)]
enum UseVarCases {
    Unsealed(Value),
    SealedOnePredecessor(SSABlock),
    SealedMultiplePredecessors(Value, Block),
}

/// States for the `use_var`/`predecessors_lookup` state machine.
enum Call {
    UseVar(SSABlock),
    FinishSealedOnePredecessor(SSABlock),
    FinishPredecessorsLookup(Value, Block),
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
    } else if ty.is_ref() {
        cur.ins().null(ty)
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
///   basic block with `declare_block_body_block` or `declare_block_header_block` depending on the
///   position of the basic block;
///
/// - while traversing a basic block and translating instruction, use `def_var` and `use_var`
///   to record definitions and uses of variables, these methods will give you the corresponding
///   SSA values;
///
/// - when all the instructions in a basic block have translated, the block is said _filled_ and
///   only then you can add it as a predecessor to other blocks with `declare_block_predecessor`;
///
/// - when you have constructed all the predecessor to a basic block at the beginning of a `Block`,
///   call `seal_block_header_block` on it with the `Function` that you are building.
///
/// This API will give you the correct SSA values to use as arguments of your instructions,
/// as well as modify the jump instruction and `Block` headers parameters to account for the SSA
/// Phi functions.
///
impl SSABuilder {
    /// Declares a new definition of a variable in a given basic block.
    /// The SSA value is passed as an argument because it should be created with
    /// `ir::DataFlowGraph::append_result`.
    pub fn def_var(&mut self, var: Variable, val: Value, ssa_block: SSABlock) {
        self.variables[var][ssa_block] = PackedOption::from(val);
    }

    /// Declares a use of a variable in a given basic block. Returns the SSA value corresponding
    /// to the current SSA definition of this variable and a list of newly created Blocks that
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
        ssa_block: SSABlock,
    ) -> (Value, SideEffects) {
        // First, try Local Value Numbering (Algorithm 1 in the paper).
        // If the variable already has a known Value in this block, use that.
        if let Some(var_defs) = self.variables.get(var) {
            if let Some(val) = var_defs[ssa_block].expand() {
                return (val, SideEffects::new());
            }
        }

        // Otherwise, use Global Value Numbering (Algorithm 2 in the paper).
        // This resolves the Value with respect to its predecessors.
        debug_assert!(self.calls.is_empty());
        debug_assert!(self.results.is_empty());
        debug_assert!(self.side_effects.is_empty());

        // Prepare the 'calls' and 'results' stacks for the state machine.
        self.use_var_nonlocal(func, var, ty, ssa_block);

        let value = self.run_state_machine(func, var, ty);
        let side_effects = mem::replace(&mut self.side_effects, SideEffects::new());

        (value, side_effects)
    }

    /// Resolve the minimal SSA Value of `var` in `block` by traversing predecessors.
    ///
    /// This function sets up state for `run_state_machine()` but does not execute it.
    fn use_var_nonlocal(
        &mut self,
        func: &mut Function,
        var: Variable,
        ty: Type,
        ssa_block: SSABlock,
    ) {
        // This function is split into two parts to appease the borrow checker.
        // Part 1: With a mutable borrow of self, update the DataFlowGraph if necessary.
        let case = match self.ssa_blocks[ssa_block] {
            SSABlockData::BlockHeader(ref mut data) => {
                // The block has multiple predecessors so we append an Block parameter that
                // will serve as a value.
                if data.sealed {
                    if data.predecessors.len() == 1 {
                        // Optimize the common case of one predecessor: no param needed.
                        UseVarCases::SealedOnePredecessor(data.predecessors[0].ssa_block)
                    } else {
                        // Break potential cycles by eagerly adding an operandless param.
                        let val = func.dfg.append_block_param(data.block, ty);
                        UseVarCases::SealedMultiplePredecessors(val, data.block)
                    }
                } else {
                    let val = func.dfg.append_block_param(data.block, ty);
                    data.undef_variables.push((var, val));
                    UseVarCases::Unsealed(val)
                }
            }
            SSABlockData::BlockBody { ssa_pred } => UseVarCases::SealedOnePredecessor(ssa_pred),
        };

        // Part 2: Prepare SSABuilder state for run_state_machine().
        match case {
            UseVarCases::SealedOnePredecessor(pred) => {
                // Get the Value directly from the single predecessor.
                self.calls.push(Call::FinishSealedOnePredecessor(ssa_block));
                self.calls.push(Call::UseVar(pred));
            }
            UseVarCases::Unsealed(val) => {
                // Define the operandless param added above to prevent lookup cycles.
                self.def_var(var, val, ssa_block);

                // Nothing more can be known at this point.
                self.results.push(val);
            }
            UseVarCases::SealedMultiplePredecessors(val, block) => {
                // Define the operandless param added above to prevent lookup cycles.
                self.def_var(var, val, ssa_block);

                // Look up a use_var for each precessor.
                self.begin_predecessors_lookup(val, block);
            }
        }
    }

    /// For blocks with a single predecessor, once we've determined the value,
    /// record a local def for it for future queries to find.
    fn finish_sealed_one_predecessor(&mut self, var: Variable, ssa_block: SSABlock) {
        let val = *self.results.last().unwrap();
        self.def_var(var, val, ssa_block);
    }

    /// Declares a new basic block belonging to the body of a certain `Block` and having `pred`
    /// as a predecessor. `pred` is the only predecessor of the block and the block is sealed
    /// at creation.
    ///
    /// To declare a `Block` header block, see `declare_block_header_block`.
    pub fn declare_block_body_block(&mut self, ssa_pred: SSABlock) -> SSABlock {
        self.ssa_blocks.push(SSABlockData::BlockBody { ssa_pred })
    }

    /// Declares a new basic block at the beginning of a `Block`. No predecessors are declared
    /// here and the block is not sealed.
    /// Predecessors have to be added with `declare_block_predecessor`.
    pub fn declare_block_header_block(&mut self, block: Block) -> SSABlock {
        let ssa_block = self
            .ssa_blocks
            .push(SSABlockData::BlockHeader(BlockHeaderSSABlockData {
                predecessors: PredBlockSmallVec::new(),
                sealed: false,
                block,
                undef_variables: Vec::new(),
            }));
        self.block_headers[block] = ssa_block.into();
        ssa_block
    }
    /// Gets the header block corresponding to an Block, panics if the Block or the header block
    /// isn't declared.
    pub fn header_block(&self, block: Block) -> SSABlock {
        self.block_headers
            .get(block)
            .expect("the block has not been declared")
            .expand()
            .expect("the header block has not been defined")
    }

    /// Declares a new predecessor for a `Block` header block and record the branch instruction
    /// of the predecessor that leads to it.
    ///
    /// Note that the predecessor is a `SSABlock` and not a `Block`. This `SSABlock` must be filled
    /// before added as predecessor. Note that you must provide no jump arguments to the branch
    /// instruction when you create it since `SSABuilder` will fill them for you.
    ///
    /// Callers are expected to avoid adding the same predecessor more than once in the case
    /// of a jump table.
    pub fn declare_block_predecessor(&mut self, block: Block, ssa_pred: SSABlock, inst: Inst) {
        debug_assert!(!self.is_sealed(block));
        let header_block = self.header_block(block);
        self.ssa_blocks[header_block].add_predecessor(ssa_pred, inst)
    }

    /// Remove a previously declared Block predecessor by giving a reference to the jump
    /// instruction. Returns the basic block containing the instruction.
    ///
    /// Note: use only when you know what you are doing, this might break the SSA building problem
    pub fn remove_block_predecessor(&mut self, block: Block, inst: Inst) -> SSABlock {
        debug_assert!(!self.is_sealed(block));
        let header_block = self.header_block(block);
        self.ssa_blocks[header_block].remove_predecessor(inst)
    }

    /// Completes the global value numbering for a `Block`, all of its predecessors having been
    /// already sealed.
    ///
    /// This method modifies the function's `Layout` by adding arguments to the `Block`s to
    /// take into account the Phi function placed by the SSA algorithm.
    ///
    /// Returns the list of newly created blocks for critical edge splitting.
    pub fn seal_block_header_block(&mut self, block: Block, func: &mut Function) -> SideEffects {
        self.seal_one_block_header_block(block, func);
        mem::replace(&mut self.side_effects, SideEffects::new())
    }

    /// Completes the global value numbering for all `Block`s in `func`.
    ///
    /// It's more efficient to seal `Block`s as soon as possible, during
    /// translation, but for frontends where this is impractical to do, this
    /// function can be used at the end of translating all blocks to ensure
    /// that everything is sealed.
    pub fn seal_all_block_header_blocks(&mut self, func: &mut Function) -> SideEffects {
        // Seal all `Block`s currently in the function. This can entail splitting
        // and creation of new blocks, however such new blocks are sealed on
        // the fly, so we don't need to account for them here.
        for block in self.block_headers.keys() {
            self.seal_one_block_header_block(block, func);
        }
        mem::replace(&mut self.side_effects, SideEffects::new())
    }

    /// Helper function for `seal_block_header_block` and
    /// `seal_all_block_header_blocks`.
    fn seal_one_block_header_block(&mut self, block: Block, func: &mut Function) {
        let ssa_block = self.header_block(block);

        let undef_vars = match self.ssa_blocks[ssa_block] {
            SSABlockData::BlockBody { .. } => panic!("this should not happen"),
            SSABlockData::BlockHeader(ref mut data) => {
                debug_assert!(
                    !data.sealed,
                    "Attempting to seal {} which is already sealed.",
                    block
                );
                debug_assert_eq!(block, data.block);
                // Extract the undef_variables data from the block so that we
                // can iterate over it without borrowing the whole builder.
                mem::replace(&mut data.undef_variables, Vec::new())
            }
        };

        // For each undef var we look up values in the predecessors and create an block parameter
        // only if necessary.
        for (var, val) in undef_vars {
            let ty = func.dfg.value_type(val);
            self.predecessors_lookup(func, val, var, ty, block);
        }
        self.mark_block_header_block_sealed(ssa_block);
    }

    /// Set the `sealed` flag for `block`.
    fn mark_block_header_block_sealed(&mut self, ssa_block: SSABlock) {
        // Then we mark the block as sealed.
        match self.ssa_blocks[ssa_block] {
            SSABlockData::BlockBody { .. } => panic!("this should not happen"),
            SSABlockData::BlockHeader(ref mut data) => {
                debug_assert!(!data.sealed);
                debug_assert!(data.undef_variables.is_empty());
                data.sealed = true;
                // We could call data.predecessors.shrink_to_fit() here, if
                // important, because no further predecessors will be added
                // to this block.
            }
        }
    }

    /// Given the local SSA Value of a Variable in an Block, perform a recursive lookup on
    /// predecessors to determine if it is redundant with another Value earlier in the CFG.
    ///
    /// If such a Value exists and is redundant, the local Value is replaced by the
    /// corresponding non-local Value. If the original Value was an Block parameter,
    /// the parameter may be removed if redundant. Parameters are placed eagerly by callers
    /// to avoid infinite loops when looking up a Value for an Block that is in a CFG loop.
    ///
    /// Doing this lookup for each Value in each Block preserves SSA form during construction.
    ///
    /// Returns the chosen Value.
    ///
    /// ## Arguments
    ///
    /// `sentinel` is a dummy Block parameter inserted by `use_var_nonlocal()`.
    /// Its purpose is to allow detection of CFG cycles while traversing predecessors.
    ///
    /// The `sentinel: Value` and the `ty: Type` are describing the `var: Variable`
    /// that is being looked up.
    fn predecessors_lookup(
        &mut self,
        func: &mut Function,
        sentinel: Value,
        var: Variable,
        ty: Type,
        block: Block,
    ) -> Value {
        debug_assert!(self.calls.is_empty());
        debug_assert!(self.results.is_empty());
        // self.side_effects may be non-empty here so that callers can
        // accumulate side effects over multiple calls.
        self.begin_predecessors_lookup(sentinel, block);
        self.run_state_machine(func, var, ty)
    }

    /// Set up state for `run_state_machine()` to initiate non-local use lookups
    /// in all predecessors of `dest_block`, and arrange for a call to
    /// `finish_predecessors_lookup` once they complete.
    fn begin_predecessors_lookup(&mut self, sentinel: Value, dest_block: Block) {
        self.calls
            .push(Call::FinishPredecessorsLookup(sentinel, dest_block));
        // Iterate over the predecessors.
        let mut calls = mem::replace(&mut self.calls, Vec::new());
        calls.extend(self.predecessors(dest_block).iter().rev().map(
            |&PredBlock {
                 ssa_block: pred, ..
             }| Call::UseVar(pred),
        ));
        self.calls = calls;
    }

    /// Examine the values from the predecessors and compute a result value, creating
    /// block parameters as needed.
    fn finish_predecessors_lookup(
        &mut self,
        func: &mut Function,
        sentinel: Value,
        var: Variable,
        dest_block: Block,
    ) {
        let mut pred_values: ZeroOneOrMore<Value> = ZeroOneOrMore::Zero;

        // Determine how many predecessors are yielding unique, non-temporary Values.
        let num_predecessors = self.predecessors(dest_block).len();
        for &pred_val in self.results.iter().rev().take(num_predecessors) {
            match pred_values {
                ZeroOneOrMore::Zero => {
                    if pred_val != sentinel {
                        pred_values = ZeroOneOrMore::One(pred_val);
                    }
                }
                ZeroOneOrMore::One(old_val) => {
                    if pred_val != sentinel && pred_val != old_val {
                        pred_values = ZeroOneOrMore::More;
                        break;
                    }
                }
                ZeroOneOrMore::More => {
                    break;
                }
            }
        }

        // Those predecessors' Values have been examined: pop all their results.
        self.results.truncate(self.results.len() - num_predecessors);

        let result_val = match pred_values {
            ZeroOneOrMore::Zero => {
                // The variable is used but never defined before. This is an irregularity in the
                // code, but rather than throwing an error we silently initialize the variable to
                // 0. This will have no effect since this situation happens in unreachable code.
                if !func.layout.is_block_inserted(dest_block) {
                    func.layout.append_block(dest_block);
                }
                self.side_effects
                    .instructions_added_to_blocks
                    .push(dest_block);
                let zero = emit_zero(
                    func.dfg.value_type(sentinel),
                    FuncCursor::new(func).at_first_insertion_point(dest_block),
                );
                func.dfg.remove_block_param(sentinel);
                func.dfg.change_to_alias(sentinel, zero);
                zero
            }
            ZeroOneOrMore::One(pred_val) => {
                // Here all the predecessors use a single value to represent our variable
                // so we don't need to have it as an block argument.
                // We need to replace all the occurrences of val with pred_val but since
                // we can't afford a re-writing pass right now we just declare an alias.
                // Resolve aliases eagerly so that we can check for cyclic aliasing,
                // which can occur in unreachable code.
                let mut resolved = func.dfg.resolve_aliases(pred_val);
                if sentinel == resolved {
                    // Cycle detected. Break it by creating a zero value.
                    resolved = emit_zero(
                        func.dfg.value_type(sentinel),
                        FuncCursor::new(func).at_first_insertion_point(dest_block),
                    );
                }
                func.dfg.remove_block_param(sentinel);
                func.dfg.change_to_alias(sentinel, resolved);
                resolved
            }
            ZeroOneOrMore::More => {
                // There is disagreement in the predecessors on which value to use so we have
                // to keep the block argument. To avoid borrowing `self` for the whole loop,
                // temporarily detach the predecessors list and replace it with an empty list.
                let mut preds =
                    mem::replace(self.predecessors_mut(dest_block), PredBlockSmallVec::new());
                for &mut PredBlock {
                    ssa_block: ref mut pred_ssa_block,
                    branch: ref mut last_inst,
                } in &mut preds
                {
                    // We already did a full `use_var` above, so we can do just the fast path.
                    let ssa_block_map = self.variables.get(var).unwrap();
                    let pred_val = ssa_block_map.get(*pred_ssa_block).unwrap().unwrap();
                    let jump_arg = self.append_jump_argument(
                        func,
                        *last_inst,
                        *pred_ssa_block,
                        dest_block,
                        pred_val,
                        var,
                    );
                    if let Some((middle_block, middle_ssa_block, middle_jump_inst)) = jump_arg {
                        *pred_ssa_block = middle_ssa_block;
                        *last_inst = middle_jump_inst;
                        self.side_effects.split_blocks_created.push(middle_block);
                    }
                }
                // Now that we're done, move the predecessors list back.
                debug_assert!(self.predecessors(dest_block).is_empty());
                *self.predecessors_mut(dest_block) = preds;

                sentinel
            }
        };

        self.results.push(result_val);
    }

    /// Appends a jump argument to a jump instruction, returns block created in case of
    /// critical edge splitting.
    fn append_jump_argument(
        &mut self,
        func: &mut Function,
        jump_inst: Inst,
        jump_inst_ssa_block: SSABlock,
        dest_block: Block,
        val: Value,
        var: Variable,
    ) -> Option<(Block, SSABlock, Inst)> {
        match func.dfg.analyze_branch(jump_inst) {
            BranchInfo::NotABranch => {
                panic!("you have declared a non-branch instruction as a predecessor to an block");
            }
            // For a single destination appending a jump argument to the instruction
            // is sufficient.
            BranchInfo::SingleDest(_, _) => {
                func.dfg.append_inst_arg(jump_inst, val);
                None
            }
            BranchInfo::Table(jt, default_block) => {
                // In the case of a jump table, the situation is tricky because br_table doesn't
                // support arguments.
                // We have to split the critical edge
                let middle_block = func.dfg.make_block();
                func.layout.append_block(middle_block);
                let middle_ssa_block = self.declare_block_header_block(middle_block);
                self.ssa_blocks[middle_ssa_block].add_predecessor(jump_inst_ssa_block, jump_inst);
                self.mark_block_header_block_sealed(middle_ssa_block);

                if let Some(default_block) = default_block {
                    if dest_block == default_block {
                        match func.dfg[jump_inst] {
                            InstructionData::BranchTable {
                                destination: ref mut dest,
                                ..
                            } => {
                                *dest = middle_block;
                            }
                            _ => panic!("should not happen"),
                        }
                    }
                }

                for old_dest in func.jump_tables[jt].as_mut_slice() {
                    if *old_dest == dest_block {
                        *old_dest = middle_block;
                    }
                }
                let mut cur = FuncCursor::new(func).at_bottom(middle_block);
                let middle_jump_inst = cur.ins().jump(dest_block, &[val]);
                self.def_var(var, val, middle_ssa_block);
                Some((middle_block, middle_ssa_block, middle_jump_inst))
            }
        }
    }

    /// Returns the list of `Block`s that have been declared as predecessors of the argument.
    fn predecessors(&self, block: Block) -> &[PredBlock] {
        let ssa_block = self.header_block(block);
        match self.ssa_blocks[ssa_block] {
            SSABlockData::BlockBody { .. } => panic!("should not happen"),
            SSABlockData::BlockHeader(ref data) => &data.predecessors,
        }
    }

    /// Returns whether the given Block has any predecessor or not.
    pub fn has_any_predecessors(&self, block: Block) -> bool {
        !self.predecessors(block).is_empty()
    }

    /// Same as predecessors, but for &mut.
    fn predecessors_mut(&mut self, block: Block) -> &mut PredBlockSmallVec {
        let ssa_block = self.header_block(block);
        match self.ssa_blocks[ssa_block] {
            SSABlockData::BlockBody { .. } => panic!("should not happen"),
            SSABlockData::BlockHeader(ref mut data) => &mut data.predecessors,
        }
    }

    /// Returns `true` if and only if `seal_block_header_block` has been called on the argument.
    pub fn is_sealed(&self, block: Block) -> bool {
        match self.ssa_blocks[self.header_block(block)] {
            SSABlockData::BlockBody { .. } => panic!("should not happen"),
            SSABlockData::BlockHeader(ref data) => data.sealed,
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
                Call::UseVar(ssa_block) => {
                    // First we lookup for the current definition of the variable in this block
                    if let Some(var_defs) = self.variables.get(var) {
                        if let Some(val) = var_defs[ssa_block].expand() {
                            self.results.push(val);
                            continue;
                        }
                    }
                    self.use_var_nonlocal(func, var, ty, ssa_block);
                }
                Call::FinishSealedOnePredecessor(ssa_block) => {
                    self.finish_sealed_one_predecessor(var, ssa_block);
                }
                Call::FinishPredecessorsLookup(sentinel, dest_block) => {
                    self.finish_predecessors_lookup(func, sentinel, var, dest_block);
                }
            }
        }
        debug_assert_eq!(self.results.len(), 1);
        self.results.pop().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::ssa::SSABuilder;
    use crate::Variable;
    use cranelift_codegen::cursor::{Cursor, FuncCursor};
    use cranelift_codegen::entity::EntityRef;
    use cranelift_codegen::ir::instructions::BranchInfo;
    use cranelift_codegen::ir::types::*;
    use cranelift_codegen::ir::{Function, Inst, InstBuilder, JumpTableData, Opcode};
    use cranelift_codegen::settings;
    use cranelift_codegen::verify_function;

    #[test]
    fn simple_block() {
        let mut func = Function::new();
        let mut ssa = SSABuilder::new();
        let block0 = func.dfg.make_block();
        // Here is the pseudo-program we want to translate:
        // x = 1;
        // y = 2;
        // z = x + y;
        // z = x + z;

        let ssa_block = ssa.declare_block_header_block(block0);
        let x_var = Variable::new(0);
        let x_ssa = {
            let mut cur = FuncCursor::new(&mut func);
            cur.insert_block(block0);
            cur.ins().iconst(I32, 1)
        };
        ssa.def_var(x_var, x_ssa, ssa_block);
        let y_var = Variable::new(1);
        let y_ssa = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(block0);
            cur.ins().iconst(I32, 2)
        };
        ssa.def_var(y_var, y_ssa, ssa_block);

        assert_eq!(ssa.use_var(&mut func, x_var, I32, ssa_block).0, x_ssa);
        assert_eq!(ssa.use_var(&mut func, y_var, I32, ssa_block).0, y_ssa);
        let z_var = Variable::new(2);
        let x_use1 = ssa.use_var(&mut func, x_var, I32, ssa_block).0;
        let y_use1 = ssa.use_var(&mut func, y_var, I32, ssa_block).0;
        let z1_ssa = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(block0);
            cur.ins().iadd(x_use1, y_use1)
        };
        ssa.def_var(z_var, z1_ssa, ssa_block);
        assert_eq!(ssa.use_var(&mut func, z_var, I32, ssa_block).0, z1_ssa);
        let x_use2 = ssa.use_var(&mut func, x_var, I32, ssa_block).0;
        let z_use1 = ssa.use_var(&mut func, z_var, I32, ssa_block).0;
        let z2_ssa = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(block0);
            cur.ins().iadd(x_use2, z_use1)
        };
        ssa.def_var(z_var, z2_ssa, ssa_block);
        assert_eq!(ssa.use_var(&mut func, z_var, I32, ssa_block).0, z2_ssa);
    }

    #[test]
    fn sequence_of_blocks() {
        let mut func = Function::new();
        let mut ssa = SSABuilder::new();
        let block0 = func.dfg.make_block();
        let block1 = func.dfg.make_block();
        // Here is the pseudo-program we want to translate:
        // block0:
        //    x = 1;
        //    y = 2;
        //    z = x + y;
        //    brnz y, block1;
        //    z = x + z;
        // block1:
        //    y = x + y;

        let ssa_block0 = ssa.declare_block_header_block(block0);
        let x_var = Variable::new(0);
        let x_ssa = {
            let mut cur = FuncCursor::new(&mut func);
            cur.insert_block(block0);
            cur.insert_block(block1);
            cur.goto_bottom(block0);
            cur.ins().iconst(I32, 1)
        };
        ssa.def_var(x_var, x_ssa, ssa_block0);
        let y_var = Variable::new(1);
        let y_ssa = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(block0);
            cur.ins().iconst(I32, 2)
        };
        ssa.def_var(y_var, y_ssa, ssa_block0);
        assert_eq!(ssa.use_var(&mut func, x_var, I32, ssa_block0).0, x_ssa);
        assert_eq!(ssa.use_var(&mut func, y_var, I32, ssa_block0).0, y_ssa);
        let z_var = Variable::new(2);
        let x_use1 = ssa.use_var(&mut func, x_var, I32, ssa_block0).0;
        let y_use1 = ssa.use_var(&mut func, y_var, I32, ssa_block0).0;
        let z1_ssa = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(block0);
            cur.ins().iadd(x_use1, y_use1)
        };
        ssa.def_var(z_var, z1_ssa, ssa_block0);
        assert_eq!(ssa.use_var(&mut func, z_var, I32, ssa_block0).0, z1_ssa);
        let y_use2 = ssa.use_var(&mut func, y_var, I32, ssa_block0).0;
        let jump_inst: Inst = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(block0);
            cur.ins().brnz(y_use2, block1, &[])
        };
        let ssa_block1 = ssa.declare_block_body_block(ssa_block0);
        let x_use2 = ssa.use_var(&mut func, x_var, I32, ssa_block1).0;
        assert_eq!(x_use2, x_ssa);
        let z_use1 = ssa.use_var(&mut func, z_var, I32, ssa_block1).0;
        assert_eq!(z_use1, z1_ssa);
        let z2_ssa = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(block0);
            cur.ins().iadd(x_use2, z_use1)
        };
        ssa.def_var(z_var, z2_ssa, ssa_block1);
        assert_eq!(ssa.use_var(&mut func, z_var, I32, ssa_block1).0, z2_ssa);
        ssa.seal_block_header_block(block0, &mut func);
        let ssa_block2 = ssa.declare_block_header_block(block1);
        ssa.declare_block_predecessor(block1, ssa_block0, jump_inst);
        ssa.seal_block_header_block(block1, &mut func);
        let x_use3 = ssa.use_var(&mut func, x_var, I32, ssa_block2).0;
        assert_eq!(x_ssa, x_use3);
        let y_use3 = ssa.use_var(&mut func, y_var, I32, ssa_block2).0;
        assert_eq!(y_ssa, y_use3);
        let y2_ssa = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(block0);
            cur.ins().iadd(x_use3, y_use3)
        };
        ssa.def_var(y_var, y2_ssa, ssa_block2);
        match func.dfg.analyze_branch(jump_inst) {
            BranchInfo::SingleDest(dest, jump_args) => {
                assert_eq!(dest, block1);
                assert_eq!(jump_args.len(), 0);
            }
            _ => assert!(false),
        };
    }

    #[test]
    fn program_with_loop() {
        let mut func = Function::new();
        let mut ssa = SSABuilder::new();
        let block0 = func.dfg.make_block();
        let block1 = func.dfg.make_block();
        let block2 = func.dfg.make_block();
        // Here is the pseudo-program we want to translate:
        // block0:
        //    x = 1;
        //    y = 2;
        //    z = x + y;
        //    jump block1
        // block1:
        //    z = z + y;
        //    brnz y, block1;
        //    z = z - x;
        //    return y
        // block2:
        //    y = y - x
        //    jump block1

        let ssa_block0 = ssa.declare_block_header_block(block0);
        ssa.seal_block_header_block(block0, &mut func);
        let x_var = Variable::new(0);
        let x1 = {
            let mut cur = FuncCursor::new(&mut func);
            cur.insert_block(block0);
            cur.insert_block(block1);
            cur.insert_block(block2);
            cur.goto_bottom(block0);
            cur.ins().iconst(I32, 1)
        };
        ssa.def_var(x_var, x1, ssa_block0);
        assert_eq!(ssa.use_var(&mut func, x_var, I32, ssa_block0).0, x1);
        let y_var = Variable::new(1);
        let y1 = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(block0);
            cur.ins().iconst(I32, 2)
        };
        ssa.def_var(y_var, y1, ssa_block0);
        assert_eq!(ssa.use_var(&mut func, y_var, I32, ssa_block0).0, y1);
        let z_var = Variable::new(2);
        let x2 = ssa.use_var(&mut func, x_var, I32, ssa_block0).0;
        assert_eq!(x2, x1);
        let y2 = ssa.use_var(&mut func, y_var, I32, ssa_block0).0;
        assert_eq!(y2, y1);
        let z1 = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(block0);
            cur.ins().iadd(x2, y2)
        };
        ssa.def_var(z_var, z1, ssa_block0);
        let jump_block0_block1 = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(block0);
            cur.ins().jump(block1, &[])
        };
        let ssa_block1 = ssa.declare_block_header_block(block1);
        ssa.declare_block_predecessor(block1, ssa_block0, jump_block0_block1);
        let z2 = ssa.use_var(&mut func, z_var, I32, ssa_block1).0;
        let y3 = ssa.use_var(&mut func, y_var, I32, ssa_block1).0;
        let z3 = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(block1);
            cur.ins().iadd(z2, y3)
        };
        ssa.def_var(z_var, z3, ssa_block1);
        let y4 = ssa.use_var(&mut func, y_var, I32, ssa_block1).0;
        assert_eq!(y4, y3);
        let jump_block1_block2 = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(block1);
            cur.ins().brnz(y4, block2, &[])
        };
        let ssa_block2 = ssa.declare_block_body_block(ssa_block1);
        let z4 = ssa.use_var(&mut func, z_var, I32, ssa_block2).0;
        assert_eq!(z4, z3);
        let x3 = ssa.use_var(&mut func, x_var, I32, ssa_block2).0;
        let z5 = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(block1);
            cur.ins().isub(z4, x3)
        };
        ssa.def_var(z_var, z5, ssa_block2);
        let y5 = ssa.use_var(&mut func, y_var, I32, ssa_block2).0;
        assert_eq!(y5, y3);
        {
            let mut cur = FuncCursor::new(&mut func).at_bottom(block1);
            cur.ins().return_(&[y5])
        };

        let ssa_block3 = ssa.declare_block_header_block(block2);
        ssa.declare_block_predecessor(block2, ssa_block1, jump_block1_block2);
        ssa.seal_block_header_block(block2, &mut func);
        let y6 = ssa.use_var(&mut func, y_var, I32, ssa_block3).0;
        assert_eq!(y6, y3);
        let x4 = ssa.use_var(&mut func, x_var, I32, ssa_block3).0;
        assert_eq!(x4, x3);
        let y7 = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(block2);
            cur.ins().isub(y6, x4)
        };
        ssa.def_var(y_var, y7, ssa_block3);
        let jump_block2_block1 = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(block2);
            cur.ins().jump(block1, &[])
        };

        ssa.declare_block_predecessor(block1, ssa_block3, jump_block2_block1);
        ssa.seal_block_header_block(block1, &mut func);
        assert_eq!(func.dfg.block_params(block1)[0], z2);
        assert_eq!(func.dfg.block_params(block1)[1], y3);
        assert_eq!(func.dfg.resolve_aliases(x3), x1);
    }

    #[test]
    fn br_table_with_args() {
        // This tests the on-demand splitting of critical edges for br_table with jump arguments
        //
        // Here is the pseudo-program we want to translate:
        //
        // function %f {
        // jt = jump_table [block2, block1]
        // block0:
        //    x = 1;
        //    br_table x, block2, jt
        // block1:
        //    x = 2
        //    jump block2
        // block2:
        //    x = x + 1
        //    return
        // }

        let mut func = Function::new();
        let mut ssa = SSABuilder::new();
        let mut jump_table = JumpTableData::new();
        let block0 = func.dfg.make_block();
        let block1 = func.dfg.make_block();
        let block2 = func.dfg.make_block();

        // block0:
        //    x = 1;
        let ssa_block0 = ssa.declare_block_header_block(block0);
        ssa.seal_block_header_block(block0, &mut func);
        let x_var = Variable::new(0);
        let x1 = {
            let mut cur = FuncCursor::new(&mut func);
            cur.insert_block(block0);
            cur.insert_block(block1);
            cur.insert_block(block2);
            cur.goto_bottom(block0);
            cur.ins().iconst(I32, 1)
        };
        ssa.def_var(x_var, x1, ssa_block0);

        // jt = jump_table [block2, block1]
        jump_table.push_entry(block2);
        jump_table.push_entry(block1);
        let jt = func.create_jump_table(jump_table);

        // block0:
        //    ...
        //    br_table x, block2, jt
        ssa.use_var(&mut func, x_var, I32, ssa_block0).0;
        let br_table = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(block0);
            cur.ins().br_table(x1, block2, jt)
        };

        // block1:
        //    x = 2
        //    jump block2
        let ssa_block1 = ssa.declare_block_header_block(block1);
        ssa.seal_block_header_block(block1, &mut func);
        let x2 = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(block1);
            cur.ins().iconst(I32, 2)
        };
        ssa.def_var(x_var, x2, ssa_block1);
        let jump_inst = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(block1);
            cur.ins().jump(block2, &[])
        };

        // block2:
        //    x = x + 1
        //    return
        let ssa_block3 = ssa.declare_block_header_block(block2);
        ssa.declare_block_predecessor(block2, ssa_block1, jump_inst);
        ssa.declare_block_predecessor(block2, ssa_block0, br_table);
        ssa.seal_block_header_block(block2, &mut func);
        let ssa_block4 = ssa.declare_block_body_block(ssa_block3);
        let x3 = ssa.use_var(&mut func, x_var, I32, ssa_block4).0;
        let x4 = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(block2);
            cur.ins().iadd_imm(x3, 1)
        };
        ssa.def_var(x_var, x4, ssa_block4);
        {
            let mut cur = FuncCursor::new(&mut func).at_bottom(block2);
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
        let block0 = func.dfg.make_block();
        let block1 = func.dfg.make_block();
        // Here is the pseudo-program we want to translate:
        // block0:
        //    x = 0
        //    y = 1
        //    z = 2
        //    jump block1
        // block1:
        //    x = z + x
        //    y = y - x
        //    jump block1
        //
        let ssa_block0 = ssa.declare_block_header_block(block0);
        let x_var = Variable::new(0);
        let y_var = Variable::new(1);
        let z_var = Variable::new(2);
        ssa.seal_block_header_block(block0, &mut func);
        let x1 = {
            let mut cur = FuncCursor::new(&mut func);
            cur.insert_block(block0);
            cur.insert_block(block1);
            cur.goto_bottom(block0);
            cur.ins().iconst(I32, 0)
        };
        ssa.def_var(x_var, x1, ssa_block0);
        let y1 = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(block0);
            cur.ins().iconst(I32, 1)
        };
        ssa.def_var(y_var, y1, ssa_block0);
        let z1 = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(block0);
            cur.ins().iconst(I32, 2)
        };
        ssa.def_var(z_var, z1, ssa_block0);
        let jump_inst = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(block0);
            cur.ins().jump(block1, &[])
        };
        let ssa_block1 = ssa.declare_block_header_block(block1);
        ssa.declare_block_predecessor(block1, ssa_block0, jump_inst);
        let z2 = ssa.use_var(&mut func, z_var, I32, ssa_block1).0;
        assert_eq!(func.dfg.block_params(block1)[0], z2);
        let x2 = ssa.use_var(&mut func, x_var, I32, ssa_block1).0;
        assert_eq!(func.dfg.block_params(block1)[1], x2);
        let x3 = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(block1);
            cur.ins().iadd(x2, z2)
        };
        ssa.def_var(x_var, x3, ssa_block1);
        let x4 = ssa.use_var(&mut func, x_var, I32, ssa_block1).0;
        let y3 = ssa.use_var(&mut func, y_var, I32, ssa_block1).0;
        assert_eq!(func.dfg.block_params(block1)[2], y3);
        let y4 = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(block1);
            cur.ins().isub(y3, x4)
        };
        ssa.def_var(y_var, y4, ssa_block1);
        let jump_inst = {
            let mut cur = FuncCursor::new(&mut func).at_bottom(block1);
            cur.ins().jump(block1, &[])
        };
        ssa.declare_block_predecessor(block1, ssa_block1, jump_inst);
        ssa.seal_block_header_block(block1, &mut func);
        // At sealing the "z" argument disappear but the remaining "x" and "y" args have to be
        // in the right order.
        assert_eq!(func.dfg.block_params(block1)[1], y3);
        assert_eq!(func.dfg.block_params(block1)[0], x2);
    }

    #[test]
    fn undef() {
        // Use vars of various types which have not been defined.
        let mut func = Function::new();
        let mut ssa = SSABuilder::new();
        let block0 = func.dfg.make_block();
        let ssa_block = ssa.declare_block_header_block(block0);
        ssa.seal_block_header_block(block0, &mut func);
        let i32_var = Variable::new(0);
        let f32_var = Variable::new(1);
        let f64_var = Variable::new(2);
        let b1_var = Variable::new(3);
        let f32x4_var = Variable::new(4);
        ssa.use_var(&mut func, i32_var, I32, ssa_block);
        ssa.use_var(&mut func, f32_var, F32, ssa_block);
        ssa.use_var(&mut func, f64_var, F64, ssa_block);
        ssa.use_var(&mut func, b1_var, B1, ssa_block);
        ssa.use_var(&mut func, f32x4_var, F32X4, ssa_block);
        assert_eq!(func.dfg.num_block_params(block0), 0);
    }

    #[test]
    fn undef_in_entry() {
        // Use a var which has not been defined. The search should hit the
        // top of the entry block, and then fall back to inserting an iconst.
        let mut func = Function::new();
        let mut ssa = SSABuilder::new();
        let block0 = func.dfg.make_block();
        let ssa_block = ssa.declare_block_header_block(block0);
        ssa.seal_block_header_block(block0, &mut func);
        let x_var = Variable::new(0);
        assert_eq!(func.dfg.num_block_params(block0), 0);
        ssa.use_var(&mut func, x_var, I32, ssa_block);
        assert_eq!(func.dfg.num_block_params(block0), 0);
        assert_eq!(
            func.dfg[func.layout.first_inst(block0).unwrap()].opcode(),
            Opcode::Iconst
        );
    }

    #[test]
    fn undef_in_entry_sealed_after() {
        // Use a var which has not been defined, but the block is not sealed
        // until afterward. Before sealing, the SSA builder should insert an
        // block param; after sealing, it should be removed.
        let mut func = Function::new();
        let mut ssa = SSABuilder::new();
        let block0 = func.dfg.make_block();
        let ssa_block = ssa.declare_block_header_block(block0);
        let x_var = Variable::new(0);
        assert_eq!(func.dfg.num_block_params(block0), 0);
        ssa.use_var(&mut func, x_var, I32, ssa_block);
        assert_eq!(func.dfg.num_block_params(block0), 1);
        ssa.seal_block_header_block(block0, &mut func);
        assert_eq!(func.dfg.num_block_params(block0), 0);
        assert_eq!(
            func.dfg[func.layout.first_inst(block0).unwrap()].opcode(),
            Opcode::Iconst
        );
    }

    #[test]
    fn unreachable_use() {
        let mut func = Function::new();
        let mut ssa = SSABuilder::new();
        let block0 = func.dfg.make_block();
        let block1 = func.dfg.make_block();
        // Here is the pseudo-program we want to translate:
        // block0:
        //    return
        // block1:
        //    brz v1, block1
        //    jump block1
        let _ssa_block0 = ssa.declare_block_header_block(block0);
        ssa.seal_block_header_block(block0, &mut func);
        let ssa_block1 = ssa.declare_block_header_block(block1);
        let ssa_block2 = ssa.declare_block_body_block(ssa_block1);
        {
            let mut cur = FuncCursor::new(&mut func);
            cur.insert_block(block0);
            cur.insert_block(block1);
            cur.goto_bottom(block0);
            cur.ins().return_(&[]);
            let x_var = Variable::new(0);
            cur.goto_bottom(block1);
            let val = ssa.use_var(&mut cur.func, x_var, I32, ssa_block1).0;
            let brz = cur.ins().brz(val, block1, &[]);
            ssa.declare_block_predecessor(block1, ssa_block1, brz);
            let j = cur.ins().jump(block1, &[]);
            ssa.declare_block_predecessor(block1, ssa_block2, j);
        }
        ssa.seal_block_header_block(block1, &mut func);
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
        let block0 = func.dfg.make_block();
        let block1 = func.dfg.make_block();
        let block2 = func.dfg.make_block();
        // Here is the pseudo-program we want to translate:
        // block0:
        //    return
        // block1:
        //    brz v1, block2
        //    jump block1
        // block2:
        //    jump block1
        let _ssa_block0 = ssa.declare_block_header_block(block0);
        ssa.seal_block_header_block(block0, &mut func);
        let ssa_block1 = ssa.declare_block_header_block(block1);
        let ssa_block2 = ssa.declare_block_header_block(block2);
        {
            let mut cur = FuncCursor::new(&mut func);
            let x_var = Variable::new(0);
            cur.insert_block(block0);
            cur.insert_block(block1);
            cur.insert_block(block2);
            cur.goto_bottom(block0);
            cur.ins().return_(&[]);
            cur.goto_bottom(block1);
            let v = ssa.use_var(&mut cur.func, x_var, I32, ssa_block1).0;
            let brz = cur.ins().brz(v, block2, &[]);
            let j0 = cur.ins().jump(block1, &[]);
            cur.goto_bottom(block2);
            let j1 = cur.ins().jump(block1, &[]);
            ssa.declare_block_predecessor(block1, ssa_block2, brz);
            ssa.declare_block_predecessor(block1, ssa_block1, j0);
            ssa.declare_block_predecessor(block2, ssa_block1, j1);
        }
        ssa.seal_block_header_block(block1, &mut func);
        ssa.seal_block_header_block(block2, &mut func);
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
