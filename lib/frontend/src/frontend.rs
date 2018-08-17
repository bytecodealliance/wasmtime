//! A frontend for building Cranelift IR from other languages.
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::entity::{EntityMap, EntitySet};
use cranelift_codegen::ir;
use cranelift_codegen::ir::function::DisplayFunction;
use cranelift_codegen::ir::{
    DataFlowGraph, Ebb, ExtFuncData, FuncRef, Function, GlobalValue, GlobalValueData, Heap,
    HeapData, Inst, InstBuilderBase, InstructionData, JumpTable, JumpTableData, SigRef, Signature,
    StackSlot, StackSlotData, Type, Value,
};
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::packed_option::PackedOption;
use ssa::{Block, SSABuilder, SideEffects};
use variable::Variable;

/// Structure used for translating a series of functions into Cranelift IR.
///
/// In order to reduce memory reallocations when compiling multiple functions,
/// `FunctionBuilderContext` holds various data structures which are cleared between
/// functions, rather than dropped, preserving the underlying allocations.
///
/// The `Variable` parameter can be any index-like type that can be made to
/// implement `EntityRef`. For frontends that don't have an obvious type to
/// use here, `variable::Variable` can be used.
pub struct FunctionBuilderContext {
    ssa: SSABuilder,
    ebbs: EntityMap<Ebb, EbbData>,
    types: EntityMap<Variable, Type>,
}

/// Temporary object used to build a single Cranelift IR `Function`.
pub struct FunctionBuilder<'a> {
    /// The function currently being built.
    /// This field is public so the function can be re-borrowed.
    pub func: &'a mut Function,

    /// Source location to assign to all new instructions.
    srcloc: ir::SourceLoc,

    func_ctx: &'a mut FunctionBuilderContext,
    position: Position,
}

#[derive(Clone, Default)]
struct EbbData {
    filled: bool,
    pristine: bool,
    user_param_count: usize,
}

struct Position {
    ebb: PackedOption<Ebb>,
    basic_block: PackedOption<Block>,
}

impl Position {
    fn at(ebb: Ebb, basic_block: Block) -> Self {
        Self {
            ebb: PackedOption::from(ebb),
            basic_block: PackedOption::from(basic_block),
        }
    }

    fn default() -> Self {
        Self {
            ebb: PackedOption::default(),
            basic_block: PackedOption::default(),
        }
    }

    fn is_default(&self) -> bool {
        self.ebb.is_none() && self.basic_block.is_none()
    }
}

impl FunctionBuilderContext {
    /// Creates a FunctionBuilderContext structure. The structure is automatically cleared after
    /// each [`FunctionBuilder`](struct.FunctionBuilder.html) completes translating a function.
    pub fn new() -> Self {
        Self {
            ssa: SSABuilder::new(),
            ebbs: EntityMap::new(),
            types: EntityMap::new(),
        }
    }

    fn clear(&mut self) {
        self.ssa.clear();
        self.ebbs.clear();
        self.types.clear();
    }

    fn is_empty(&self) -> bool {
        self.ssa.is_empty() && self.ebbs.is_empty() && self.types.is_empty()
    }
}

/// Implementation of the [`InstBuilder`](../codegen/ir/builder/trait.InstBuilder.html) that has
/// one convenience method per Cranelift IR instruction.
pub struct FuncInstBuilder<'short, 'long: 'short> {
    builder: &'short mut FunctionBuilder<'long>,
    ebb: Ebb,
}

impl<'short, 'long> FuncInstBuilder<'short, 'long> {
    fn new<'s, 'l>(builder: &'s mut FunctionBuilder<'l>, ebb: Ebb) -> FuncInstBuilder<'s, 'l> {
        FuncInstBuilder { builder, ebb }
    }
}

impl<'short, 'long> InstBuilderBase<'short> for FuncInstBuilder<'short, 'long> {
    fn data_flow_graph(&self) -> &DataFlowGraph {
        &self.builder.func.dfg
    }

    fn data_flow_graph_mut(&mut self) -> &mut DataFlowGraph {
        &mut self.builder.func.dfg
    }

    // This implementation is richer than `InsertBuilder` because we use the data of the
    // instruction being inserted to add related info to the DFG and the SSA building system,
    // and perform debug sanity checks.
    fn build(self, data: InstructionData, ctrl_typevar: Type) -> (Inst, &'short mut DataFlowGraph) {
        // We only insert the Ebb in the layout when an instruction is added to it
        self.builder.ensure_inserted_ebb();

        let inst = self.builder.func.dfg.make_inst(data.clone());
        self.builder.func.dfg.make_inst_results(inst, ctrl_typevar);
        self.builder.func.layout.append_inst(inst, self.ebb);
        if !self.builder.srcloc.is_default() {
            self.builder.func.srclocs[inst] = self.builder.srcloc;
        }

        if data.opcode().is_branch() {
            match data.branch_destination() {
                Some(dest_ebb) => {
                    // If the user has supplied jump arguments we must adapt the arguments of
                    // the destination ebb
                    self.builder.declare_successor(dest_ebb, inst);
                }
                None => {
                    // branch_destination() doesn't detect jump_tables
                    // If jump table we declare all entries successor
                    if let InstructionData::BranchTable { table, .. } = data {
                        // Unlike all other jumps/branches, jump tables are
                        // capable of having the same successor appear
                        // multiple times, so we must deduplicate.
                        let mut unique = EntitySet::<Ebb>::new();
                        for dest_ebb in self
                            .builder
                            .func
                            .jump_tables
                            .get(table)
                            .expect("you are referencing an undeclared jump table")
                            .entries()
                            .map(|(_, ebb)| ebb)
                            .filter(|dest_ebb| unique.insert(*dest_ebb))
                        {
                            self.builder.func_ctx.ssa.declare_ebb_predecessor(
                                dest_ebb,
                                self.builder.position.basic_block.unwrap(),
                                inst,
                            )
                        }
                    }
                }
            }
        }
        if data.opcode().is_terminator() {
            self.builder.fill_current_block()
        } else if data.opcode().is_branch() {
            self.builder.move_to_next_basic_block()
        }
        (inst, &mut self.builder.func.dfg)
    }
}

/// This module allows you to create a function in Cranelift IR in a straightforward way, hiding
/// all the complexity of its internal representation.
///
/// The module is parametrized by one type which is the representation of variables in your
/// origin language. It offers a way to conveniently append instruction to your program flow.
/// You are responsible to split your instruction flow into extended blocks (declared with
/// `create_ebb`) whose properties are:
///
/// - branch and jump instructions can only point at the top of extended blocks;
/// - the last instruction of each block is a terminator instruction which has no natural successor,
///   and those instructions can only appear at the end of extended blocks.
///
/// The parameters of Cranelift IR instructions are Cranelift IR values, which can only be created
/// as results of other Cranelift IR instructions. To be able to create variables redefined multiple
/// times in your program, use the `def_var` and `use_var` command, that will maintain the
/// correspondence between your variables and Cranelift IR SSA values.
///
/// The first block for which you call `switch_to_block` will be assumed to be the beginning of
/// the function.
///
/// At creation, a `FunctionBuilder` instance borrows an already allocated `Function` which it
/// modifies with the information stored in the mutable borrowed
/// [`FunctionBuilderContext`](struct.FunctionBuilderContext.html). The function passed in
/// argument should be newly created with
/// [`Function::with_name_signature()`](../function/struct.Function.html), whereas the
/// `FunctionBuilderContext` can be kept as is between two function translations.
///
/// # Errors
///
/// The functions below will panic in debug mode whenever you try to modify the Cranelift IR
/// function in a way that violate the coherence of the code. For instance: switching to a new
/// `Ebb` when you haven't filled the current one with a terminator instruction, inserting a
/// return instruction with arguments that don't match the function's signature.
impl<'a> FunctionBuilder<'a> {
    /// Creates a new FunctionBuilder structure that will operate on a `Function` using a
    /// `FunctionBuilderContext`.
    pub fn new(
        func: &'a mut Function,
        func_ctx: &'a mut FunctionBuilderContext,
    ) -> FunctionBuilder<'a> {
        debug_assert!(func_ctx.is_empty());
        FunctionBuilder {
            func,
            srcloc: Default::default(),
            func_ctx,
            position: Position::default(),
        }
    }

    /// Set the source location that should be assigned to all new instructions.
    pub fn set_srcloc(&mut self, srcloc: ir::SourceLoc) {
        self.srcloc = srcloc;
    }

    /// Creates a new `Ebb` and returns its reference.
    pub fn create_ebb(&mut self) -> Ebb {
        let ebb = self.func.dfg.make_ebb();
        self.func_ctx.ssa.declare_ebb_header_block(ebb);
        self.func_ctx.ebbs[ebb] = EbbData {
            filled: false,
            pristine: true,
            user_param_count: 0,
        };
        ebb
    }

    /// After the call to this function, new instructions will be inserted into the designated
    /// block, in the order they are declared. You must declare the types of the Ebb arguments
    /// you will use here.
    ///
    /// When inserting the terminator instruction (which doesn't have a fallthrough to its immediate
    /// successor), the block will be declared filled and it will not be possible to append
    /// instructions to it.
    pub fn switch_to_block(&mut self, ebb: Ebb) {
        // First we check that the previous block has been filled.
        debug_assert!(
            self.position.is_default()
                || self.is_unreachable()
                || self.is_pristine()
                || self.is_filled(),
            "you have to fill your block before switching"
        );
        // We cannot switch to a filled block
        debug_assert!(
            !self.func_ctx.ebbs[ebb].filled,
            "you cannot switch to a block which is already filled"
        );

        let basic_block = self.func_ctx.ssa.header_block(ebb);
        // Then we change the cursor position.
        self.position = Position::at(ebb, basic_block);
    }

    /// Declares that all the predecessors of this block are known.
    ///
    /// Function to call with `ebb` as soon as the last branch instruction to `ebb` has been
    /// created. Forgetting to call this method on every block will cause inconsistencies in the
    /// produced functions.
    pub fn seal_block(&mut self, ebb: Ebb) {
        let side_effects = self.func_ctx.ssa.seal_ebb_header_block(ebb, self.func);
        self.handle_ssa_side_effects(side_effects);
    }

    /// Effectively calls seal_block on all blocks in the function.
    ///
    /// It's more efficient to seal `Ebb`s as soon as possible, during
    /// translation, but for frontends where this is impractical to do, this
    /// function can be used at the end of translating all blocks to ensure
    /// that everything is sealed.
    pub fn seal_all_blocks(&mut self) {
        let side_effects = self.func_ctx.ssa.seal_all_ebb_header_blocks(self.func);
        self.handle_ssa_side_effects(side_effects);
    }

    /// In order to use a variable in a `use_var`, you need to declare its type with this method.
    pub fn declare_var(&mut self, var: Variable, ty: Type) {
        self.func_ctx.types[var] = ty;
    }

    /// Returns the Cranelift IR value corresponding to the utilization at the current program
    /// position of a previously defined user variable.
    pub fn use_var(&mut self, var: Variable) -> Value {
        let (val, side_effects) = {
            let ty = *self.func_ctx.types.get(var).unwrap_or_else(|| {
                panic!(
                    "variable {:?} is used but its type has not been declared",
                    var
                )
            });
            self.func_ctx
                .ssa
                .use_var(self.func, var, ty, self.position.basic_block.unwrap())
        };
        self.handle_ssa_side_effects(side_effects);
        val
    }

    /// Register a new definition of a user variable. The type of the value must be
    /// the same as the type registered for the variable.
    pub fn def_var(&mut self, var: Variable, val: Value) {
        debug_assert_eq!(
            *self.func_ctx.types.get(var).unwrap_or_else(|| panic!(
                "variable {:?} is used but its type has not been declared",
                var
            )),
            self.func.dfg.value_type(val),
            "declared type of variable {:?} doesn't match type of value {}",
            var,
            val
        );

        self.func_ctx
            .ssa
            .def_var(var, val, self.position.basic_block.unwrap());
    }

    /// Creates a jump table in the function, to be used by `br_table` instructions.
    pub fn create_jump_table(&mut self, data: JumpTableData) -> JumpTable {
        self.func.create_jump_table(data)
    }

    /// Inserts an entry in a previously declared jump table.
    pub fn insert_jump_table_entry(&mut self, jt: JumpTable, index: usize, ebb: Ebb) {
        self.func.insert_jump_table_entry(jt, index, ebb)
    }

    /// Creates a stack slot in the function, to be used by `stack_load`, `stack_store` and
    /// `stack_addr` instructions.
    pub fn create_stack_slot(&mut self, data: StackSlotData) -> StackSlot {
        self.func.create_stack_slot(data)
    }

    /// Adds a signature which can later be used to declare an external function import.
    pub fn import_signature(&mut self, signature: Signature) -> SigRef {
        self.func.import_signature(signature)
    }

    /// Declare an external function import.
    pub fn import_function(&mut self, data: ExtFuncData) -> FuncRef {
        self.func.import_function(data)
    }

    /// Declares a global value accessible to the function.
    pub fn create_global_value(&mut self, data: GlobalValueData) -> GlobalValue {
        self.func.create_global_value(data)
    }

    /// Declares a heap accessible to the function.
    pub fn create_heap(&mut self, data: HeapData) -> Heap {
        self.func.create_heap(data)
    }

    /// Returns an object with the [`InstBuilder`](../codegen/ir/builder/trait.InstBuilder.html)
    /// trait that allows to conveniently append an instruction to the current `Ebb` being built.
    pub fn ins<'short>(&'short mut self) -> FuncInstBuilder<'short, 'a> {
        let ebb = self.position.ebb.unwrap();
        FuncInstBuilder::new(self, ebb)
    }

    /// Make sure that the current EBB is inserted in the layout.
    pub fn ensure_inserted_ebb(&mut self) {
        let ebb = self.position.ebb.unwrap();
        if self.func_ctx.ebbs[ebb].pristine {
            if !self.func.layout.is_ebb_inserted(ebb) {
                self.func.layout.append_ebb(ebb);
            }
            self.func_ctx.ebbs[ebb].pristine = false;
        } else {
            debug_assert!(
                !self.func_ctx.ebbs[ebb].filled,
                "you cannot add an instruction to a block already filled"
            );
        }
    }

    /// Returns a `FuncCursor` pointed at the current position ready for inserting instructions.
    ///
    /// This can be used to insert SSA code that doesn't need to access locals and that doesn't
    /// need to know about `FunctionBuilder` at all.
    pub fn cursor(&mut self) -> FuncCursor {
        self.ensure_inserted_ebb();
        FuncCursor::new(self.func)
            .with_srcloc(self.srcloc)
            .at_bottom(self.position.ebb.unwrap())
    }

    /// Append parameters to the given `Ebb` corresponding to the function
    /// parameters. This can be used to set up the ebb parameters for the
    /// entry block.
    pub fn append_ebb_params_for_function_params(&mut self, ebb: Ebb) {
        // These parameters count as "user" parameters here because they aren't
        // inserted by the SSABuilder.
        let user_param_count = &mut self.func_ctx.ebbs[ebb].user_param_count;
        for argtyp in &self.func.signature.params {
            *user_param_count += 1;
            self.func.dfg.append_ebb_param(ebb, argtyp.value_type);
        }
    }

    /// Append parameters to the given `Ebb` corresponding to the function
    /// return values. This can be used to set up the ebb parameters for a
    /// function exit block.
    pub fn append_ebb_params_for_function_returns(&mut self, ebb: Ebb) {
        // These parameters count as "user" parameters here because they aren't
        // inserted by the SSABuilder.
        let user_param_count = &mut self.func_ctx.ebbs[ebb].user_param_count;
        for argtyp in &self.func.signature.returns {
            *user_param_count += 1;
            self.func.dfg.append_ebb_param(ebb, argtyp.value_type);
        }
    }

    /// Declare that translation of the current function is complete. This
    /// resets the state of the `FunctionBuilder` in preparation to be used
    /// for another function.
    pub fn finalize(&mut self) {
        // Check that all the `Ebb`s are filled and sealed.
        debug_assert!(
            self.func_ctx
                .ebbs
                .iter()
                .all(|(ebb, ebb_data)| ebb_data.pristine || self.func_ctx.ssa.is_sealed(ebb)),
            "all blocks should be sealed before dropping a FunctionBuilder"
        );
        debug_assert!(
            self.func_ctx
                .ebbs
                .values()
                .all(|ebb_data| ebb_data.pristine || ebb_data.filled),
            "all blocks should be filled before dropping a FunctionBuilder"
        );

        // Clear the state (but preserve the allocated buffers) in preparation
        // for translation another function.
        self.func_ctx.clear();

        // Reset srcloc and position to initial states.
        self.srcloc = Default::default();
        self.position = Position::default();
    }
}

/// All the functions documented in the previous block are write-only and help you build a valid
/// Cranelift IR functions via multiple debug asserts. However, you might need to improve the
/// performance of your translation perform more complex transformations to your Cranelift IR
/// function. The functions below help you inspect the function you're creating and modify it
/// in ways that can be unsafe if used incorrectly.
impl<'a> FunctionBuilder<'a> {
    /// Retrieves all the parameters for an `Ebb` currently inferred from the jump instructions
    /// inserted that target it and the SSA construction.
    pub fn ebb_params(&self, ebb: Ebb) -> &[Value] {
        self.func.dfg.ebb_params(ebb)
    }

    /// Retrieves the signature with reference `sigref` previously added with `import_signature`.
    pub fn signature(&self, sigref: SigRef) -> Option<&Signature> {
        self.func.dfg.signatures.get(sigref)
    }

    /// Creates a parameter for a specific `Ebb` by appending it to the list of already existing
    /// parameters.
    ///
    /// **Note:** this function has to be called at the creation of the `Ebb` before adding
    /// instructions to it, otherwise this could interfere with SSA construction.
    pub fn append_ebb_param(&mut self, ebb: Ebb, ty: Type) -> Value {
        debug_assert!(self.func_ctx.ebbs[ebb].pristine);
        debug_assert_eq!(
            self.func_ctx.ebbs[ebb].user_param_count,
            self.func.dfg.num_ebb_params(ebb)
        );
        self.func_ctx.ebbs[ebb].user_param_count += 1;
        self.func.dfg.append_ebb_param(ebb, ty)
    }

    /// Returns the result values of an instruction.
    pub fn inst_results(&self, inst: Inst) -> &[Value] {
        self.func.dfg.inst_results(inst)
    }

    /// Changes the destination of a jump instruction after creation.
    ///
    /// **Note:** You are responsible for maintaining the coherence with the arguments of
    /// other jump instructions.
    pub fn change_jump_destination(&mut self, inst: Inst, new_dest: Ebb) {
        let old_dest = self.func.dfg[inst]
            .branch_destination_mut()
            .expect("you want to change the jump destination of a non-jump instruction");
        let pred = self.func_ctx.ssa.remove_ebb_predecessor(*old_dest, inst);
        *old_dest = new_dest;
        self.func_ctx
            .ssa
            .declare_ebb_predecessor(new_dest, pred, inst);
    }

    /// Returns `true` if and only if the current `Ebb` is sealed and has no predecessors declared.
    ///
    /// The entry block of a function is never unreachable.
    pub fn is_unreachable(&self) -> bool {
        let is_entry = match self.func.layout.entry_block() {
            None => false,
            Some(entry) => self.position.ebb.unwrap() == entry,
        };
        !is_entry && self.func_ctx.ssa.is_sealed(self.position.ebb.unwrap())
            && !self
                .func_ctx
                .ssa
                .has_any_predecessors(self.position.ebb.unwrap())
    }

    /// Returns `true` if and only if no instructions have been added since the last call to
    /// `switch_to_block`.
    pub fn is_pristine(&self) -> bool {
        self.func_ctx.ebbs[self.position.ebb.unwrap()].pristine
    }

    /// Returns `true` if and only if a terminator instruction has been inserted since the
    /// last call to `switch_to_block`.
    pub fn is_filled(&self) -> bool {
        self.func_ctx.ebbs[self.position.ebb.unwrap()].filled
    }

    /// Returns a displayable object for the function as it is.
    ///
    /// Useful for debug purposes. Use it with `None` for standard printing.
    // Clippy thinks the lifetime that follows is needless, but rustc needs it
    #[cfg_attr(feature = "cargo-clippy", allow(needless_lifetimes))]
    pub fn display<'b, I: Into<Option<&'b TargetIsa>>>(&'b self, isa: I) -> DisplayFunction {
        self.func.display(isa)
    }
}

// Helper functions
impl<'a> FunctionBuilder<'a> {
    fn move_to_next_basic_block(&mut self) {
        self.position.basic_block = PackedOption::from(
            self.func_ctx
                .ssa
                .declare_ebb_body_block(self.position.basic_block.unwrap()),
        );
    }

    fn fill_current_block(&mut self) {
        self.func_ctx.ebbs[self.position.ebb.unwrap()].filled = true;
    }

    fn declare_successor(&mut self, dest_ebb: Ebb, jump_inst: Inst) {
        self.func_ctx.ssa.declare_ebb_predecessor(
            dest_ebb,
            self.position.basic_block.unwrap(),
            jump_inst,
        );
    }

    fn handle_ssa_side_effects(&mut self, side_effects: SideEffects) {
        for split_ebb in side_effects.split_ebbs_created {
            self.func_ctx.ebbs[split_ebb].filled = true
        }
        for modified_ebb in side_effects.instructions_added_to_ebbs {
            self.func_ctx.ebbs[modified_ebb].pristine = false
        }
    }
}

#[cfg(test)]
mod tests {

    use cranelift_codegen::entity::EntityRef;
    use cranelift_codegen::ir::types::*;
    use cranelift_codegen::ir::{AbiParam, ExternalName, Function, InstBuilder, Signature};
    use cranelift_codegen::settings;
    use cranelift_codegen::settings::CallConv;
    use cranelift_codegen::verifier::verify_function;
    use frontend::{FunctionBuilder, FunctionBuilderContext};
    use Variable;

    fn sample_function(lazy_seal: bool) {
        let mut sig = Signature::new(CallConv::SystemV);
        sig.returns.push(AbiParam::new(I32));
        sig.params.push(AbiParam::new(I32));

        let mut fn_ctx = FunctionBuilderContext::new();
        let mut func = Function::with_name_signature(ExternalName::testcase("sample"), sig);
        {
            let mut builder = FunctionBuilder::new(&mut func, &mut fn_ctx);

            let block0 = builder.create_ebb();
            let block1 = builder.create_ebb();
            let block2 = builder.create_ebb();
            let x = Variable::new(0);
            let y = Variable::new(1);
            let z = Variable::new(2);
            builder.declare_var(x, I32);
            builder.declare_var(y, I32);
            builder.declare_var(z, I32);
            builder.append_ebb_params_for_function_params(block0);

            builder.switch_to_block(block0);
            if !lazy_seal {
                builder.seal_block(block0);
            }
            {
                let tmp = builder.ebb_params(block0)[0]; // the first function parameter
                builder.def_var(x, tmp);
            }
            {
                let tmp = builder.ins().iconst(I32, 2);
                builder.def_var(y, tmp);
            }
            {
                let arg1 = builder.use_var(x);
                let arg2 = builder.use_var(y);
                let tmp = builder.ins().iadd(arg1, arg2);
                builder.def_var(z, tmp);
            }
            builder.ins().jump(block1, &[]);

            builder.switch_to_block(block1);
            {
                let arg1 = builder.use_var(y);
                let arg2 = builder.use_var(z);
                let tmp = builder.ins().iadd(arg1, arg2);
                builder.def_var(z, tmp);
            }
            {
                let arg = builder.use_var(y);
                builder.ins().brnz(arg, block2, &[]);
            }
            {
                let arg1 = builder.use_var(z);
                let arg2 = builder.use_var(x);
                let tmp = builder.ins().isub(arg1, arg2);
                builder.def_var(z, tmp);
            }
            {
                let arg = builder.use_var(y);
                builder.ins().return_(&[arg]);
            }

            builder.switch_to_block(block2);
            if !lazy_seal {
                builder.seal_block(block2);
            }

            {
                let arg1 = builder.use_var(y);
                let arg2 = builder.use_var(x);
                let tmp = builder.ins().isub(arg1, arg2);
                builder.def_var(y, tmp);
            }
            builder.ins().jump(block1, &[]);
            if !lazy_seal {
                builder.seal_block(block1);
            }

            if lazy_seal {
                builder.seal_all_blocks();
            }

            builder.finalize();
        }

        let flags = settings::Flags::new(settings::builder());
        // println!("{}", func.display(None));
        if let Err(errors) = verify_function(&func, &flags) {
            panic!("{}\n{}", func.display(None), errors)
        }
    }

    #[test]
    fn sample() {
        sample_function(false)
    }

    #[test]
    fn sample_with_lazy_seal() {
        sample_function(true)
    }
}
