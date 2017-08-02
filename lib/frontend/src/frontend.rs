//! A frontend for building Cretonne IL from other languages.
use cretonne::ir::{Ebb, Type, Value, Function, Inst, JumpTable, StackSlot, JumpTableData,
                   StackSlotData, DataFlowGraph, InstructionData, ExtFuncData, FuncRef, SigRef,
                   Signature, InstBuilderBase};
use cretonne::ir::instructions::BranchInfo;
use cretonne::ir::function::DisplayFunction;
use cretonne::isa::TargetIsa;
use ssa::{SSABuilder, SideEffects, Block};
use cretonne::entity_map::{EntityMap, PrimaryEntityData};
use cretonne::entity_ref::EntityRef;
use std::hash::Hash;

/// Permanent structure used for translating into Cretonne IL.
pub struct ILBuilder<Variable>
    where Variable: EntityRef + Hash + Default
{
    ssa: SSABuilder<Variable>,
    ebbs: EntityMap<Ebb, EbbData>,
    types: EntityMap<Variable, Type>,
    function_args_values: Vec<Value>,
}


/// Temporary object used to build a Cretonne IL `Function`.
pub struct FunctionBuilder<'a, Variable: 'a>
    where Variable: EntityRef + Hash + Default
{
    func: &'a mut Function,
    builder: &'a mut ILBuilder<Variable>,
    position: Position,
    pristine: bool,
}

#[derive(Clone, Default)]
struct EbbData {
    filled: bool,
    pristine: bool,
    user_arg_count: usize,
}

impl PrimaryEntityData for EbbData {}

struct Position {
    ebb: Ebb,
    basic_block: Block,
}

impl<Variable> ILBuilder<Variable>
    where Variable: EntityRef + Hash + Default
{
    /// Creates a ILBuilder structure. The structure is automatically cleared each time it is
    /// passed to a [`FunctionBuilder`](struct.FunctionBuilder.html) for creation.
    pub fn new() -> ILBuilder<Variable> {
        ILBuilder {
            ssa: SSABuilder::new(),
            ebbs: EntityMap::new(),
            types: EntityMap::new(),
            function_args_values: Vec::new(),
        }
    }

    fn clear(&mut self) {
        self.ssa.clear();
        self.ebbs.clear();
        self.types.clear();
        self.function_args_values.clear();
    }
}

/// Implementation of the [`InstBuilder`](../cretonne/ir/builder/trait.InstBuilder.html) that has
/// one convenience method per Cretonne IL instruction.
pub struct FuncInstBuilder<'short, 'long: 'short, Variable: 'long>
    where Variable: EntityRef + Hash + Default
{
    builder: &'short mut FunctionBuilder<'long, Variable>,
    ebb: Ebb,
}

impl<'short, 'long, Variable> FuncInstBuilder<'short, 'long, Variable>
    where Variable: EntityRef + Hash + Default
{
    fn new<'s, 'l>(builder: &'s mut FunctionBuilder<'l, Variable>,
                   ebb: Ebb)
                   -> FuncInstBuilder<'s, 'l, Variable> {
        FuncInstBuilder { builder, ebb }
    }
}

impl<'short, 'long, Variable> InstBuilderBase<'short> for FuncInstBuilder<'short, 'long, Variable>
    where Variable: EntityRef + Hash + Default
{
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
        if data.opcode().is_return() {
            self.builder
                .check_return_args(data.arguments(&self.builder.func.dfg.value_lists))
        }
        // We only insert the Ebb in the layout when an instruction is added to it
        if self.builder.builder.ebbs[self.builder.position.ebb].pristine {
            if !self.builder
                    .func
                    .layout
                    .is_ebb_inserted(self.builder.position.ebb) {
                self.builder
                    .func
                    .layout
                    .append_ebb(self.builder.position.ebb);
            }
            self.builder.builder.ebbs[self.builder.position.ebb].pristine = false;
        } else {
            debug_assert!(!self.builder.builder.ebbs[self.builder.position.ebb].filled,
                          "you cannot add an instruction to a block already filled");
        }
        let inst = self.builder.func.dfg.make_inst(data.clone());
        self.builder.func.dfg.make_inst_results(inst, ctrl_typevar);
        self.builder.func.layout.append_inst(inst, self.ebb);
        if data.opcode().is_branch() {
            match data.branch_destination() {
                Some(dest_ebb) => {
                    // If the user has supplied jump arguments we must adapt the arguments of
                    // the destination ebb
                    // TODO: find a way not to allocate a vector
                    let args_types: Vec<Type> =
                        match data.analyze_branch(&self.builder.func.dfg.value_lists) {
                            BranchInfo::SingleDest(_, args) => {
                                args.iter()
                                    .map(|arg| self.builder.func.dfg.value_type(arg.clone()))
                                    .collect()
                            }
                            _ => panic!("should not happen"),
                        };
                    self.builder
                        .ebb_args_adjustement(dest_ebb, args_types.as_slice());
                    self.builder.declare_successor(dest_ebb, inst);
                }
                None => {
                    // branch_destination() doesn't detect jump_tables
                    match data {
                        // If jump table we declare all entries successor
                        // TODO: not collect with vector?
                        InstructionData::BranchTable { table, .. } => {
                            for dest_ebb in self.builder
                                    .func
                                    .jump_tables
                                    .get(table)
                                    .expect("you are referencing an undeclared jump table")
                                    .entries()
                                    .map(|(_, ebb)| ebb)
                                    .collect::<Vec<Ebb>>() {
                                self.builder.declare_successor(dest_ebb, inst)
                            }
                        }
                        // If not we do nothing
                        _ => {}
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

/// This module allows you to create a function in Cretonne IL in a straightforward way, hiding
/// all the complexity of its internal representation.
///
/// The module is parametrized by one type which is the representation of variables in your
/// origin language. It offers a way to conveniently append instruction to your program flow.
/// You are responsible to split your instruction flow into extended blocks (declared with
/// `create_ebb`) whose properties are:
///
/// - branch and jump instructions can only point at the top of extended blocks;
/// - the last instruction of each block is a terminator instruction which has no natural sucessor,
///   and those instructions can only appear at the end of extended blocks.
///
/// The parameters of Cretonne IL instructions are Cretonne IL values, which can only be created
/// as results of other Cretonne IL instructions. To be able to create variables redefined multiple
/// times in your program, use the `def_var` and `use_var` command, that will maintain the
/// correspondance between your variables and Cretonne IL SSA values.
///
/// The first block for which you call `switch_to_block` will be assumed to be the beginning of
/// the function.
///
/// At creation, a `FunctionBuilder` instance borrows an already allocated `Function` which it
/// modifies with the information stored in the mutable borrowed
/// [`ILBuilder`](struct.ILBuilder.html). The function passed in argument should be newly created
///  with [`Function::with_name_signature()`](../function/struct.Function.html), whereas the
/// `ILBuilder` can be kept as is between two function translations.
///
/// # Errors
///
/// The functions below will panic in debug mode whenever you try to modify the Cretonne IL
/// function in a way that violate the coherence of the code. For instance: switching to a new
/// `Ebb` when you haven't filled the current one with a terminator instruction, inserting a
/// return instruction with arguments that don't match the function's signature.
impl<'a, Variable> FunctionBuilder<'a, Variable>
    where Variable: EntityRef + Hash + Default
{
    /// Creates a new FunctionBuilder structure that will operate on a `Function` using a
    /// `IlBuilder`.
    pub fn new(func: &'a mut Function,
               builder: &'a mut ILBuilder<Variable>)
               -> FunctionBuilder<'a, Variable> {
        builder.clear();
        FunctionBuilder {
            func: func,
            builder: builder,
            position: Position {
                ebb: Ebb::new(0),
                basic_block: Block::new(0),
            },
            pristine: true,
        }
    }

    /// Creates a new `Ebb` for the function and returns its reference.
    pub fn create_ebb(&mut self) -> Ebb {
        let ebb = self.func.dfg.make_ebb();
        self.builder.ssa.declare_ebb_header_block(ebb);
        *self.builder.ebbs.ensure(ebb) = EbbData {
            filled: false,
            pristine: true,
            user_arg_count: 0,
        };
        ebb
    }

    /// After the call to this function, new instructions will be inserted into the designated
    /// block, in the order they are declared. You must declare the types of the Ebb arguments
    /// you will use here.
    ///
    /// When inserting the terminator instruction (which doesn't have a falltrough to its immediate
    /// successor), the block will be declared filled and it will not be possible to append
    /// instructions to it.
    pub fn switch_to_block(&mut self, ebb: Ebb, jump_args: &[Type]) -> &[Value] {
        if self.pristine {
            self.fill_function_args_values(ebb);
        }
        if !self.builder.ebbs[self.position.ebb].pristine {
            // First we check that the previous block has been filled.
            debug_assert!(self.is_unreachable() || self.builder.ebbs[self.position.ebb].filled,
                          "you have to fill your block before switching");
        }
        // We cannot switch to a filled block
        debug_assert!(!self.builder.ebbs[ebb].filled,
                      "you cannot switch to a block which is already filled");

        let basic_block = self.builder.ssa.header_block(ebb);
        // Then we change the cursor position.
        self.position = Position {
            ebb: ebb,
            basic_block: basic_block,
        };
        self.ebb_args_adjustement(ebb, jump_args);
        self.func.dfg.ebb_args(ebb)
    }

    /// Declares that all the predecessors of this block are known.
    ///
    /// Function to call with `ebb` as soon as the last branch instruction to `ebb` has been
    /// created. Forgetting to call this method on every block will cause inconsistences in the
    /// produced functions.
    pub fn seal_block(&mut self, ebb: Ebb) {
        let side_effects = self.builder
            .ssa
            .seal_ebb_header_block(ebb,
                                   &mut self.func.dfg,
                                   &mut self.func.layout,
                                   &mut self.func.jump_tables);
        self.handle_ssa_side_effects(side_effects);
    }

    /// In order to use a variable in a `use_var`, you need to declare its type with this method.
    pub fn declare_var(&mut self, var: Variable, ty: Type) {
        *self.builder.types.ensure(var) = ty;
    }

    /// Returns the Cretonne IL value corresponding to the utilization at the current program
    /// position of a previously defined user variable.
    pub fn use_var(&mut self, var: Variable) -> Value {
        let ty = *self.builder
                      .types
                      .get(var)
                      .expect("this variable is used but its type has not been declared");
        let (val, side_effects) = self.builder
            .ssa
            .use_var(&mut self.func.dfg,
                     &mut self.func.layout,
                     &mut self.func.jump_tables,
                     var,
                     ty,
                     self.position.basic_block);
        self.handle_ssa_side_effects(side_effects);
        val
    }

    /// Register a new definition of a user variable. Panics if the type of the value is not the
    /// same as the type registered for the variable.
    pub fn def_var(&mut self, var: Variable, val: Value) {
        debug_assert!(self.func.dfg.value_type(val) == self.builder.types[var],
                      "the type of the value is not the type registered for the variable");
        self.builder
            .ssa
            .def_var(var, val, self.position.basic_block);
    }

    /// Returns the value corresponding to the `i`-th argument of the function as defined by
    /// the function signature. Panics if `i` is out of bounds or if called before the first call
    /// to `switch_to_block`.
    pub fn arg_value(&self, i: usize) -> Value {
        debug_assert!(!self.pristine, "you have to call switch_to_block first.");
        self.builder.function_args_values[i]
    }

    /// Creates a jump table in the function, to be used by `br_table` instructions.
    pub fn create_jump_table(&mut self) -> JumpTable {
        self.func.jump_tables.push(JumpTableData::new())
    }

    /// Inserts an entry in a previously declared jump table.
    pub fn insert_jump_table_entry(&mut self, jt: JumpTable, index: usize, ebb: Ebb) {
        self.func.jump_tables[jt].set_entry(index, ebb);
    }

    /// Creates a stack slot in the function, to be used by `stack_load`, `stack_store` and
    /// `stack_addr` instructions.
    pub fn create_stack_slot(&mut self, data: StackSlotData) -> StackSlot {
        self.func.stack_slots.push(data)
    }

    /// Adds a signature which can later be used to declare an external function import.
    pub fn import_signature(&mut self, signature: Signature) -> SigRef {
        self.func.dfg.signatures.push(signature)
    }

    /// Declare an external function import.
    pub fn import_function(&mut self, data: ExtFuncData) -> FuncRef {
        self.func.dfg.ext_funcs.push(data)
    }

    /// Returns an object with the [`InstBuilder`](../cretonne/ir/builder/trait.InstBuilder.html)
    /// trait that allows to conveniently append an instruction to the current `Ebb` being built.
    pub fn ins<'short>(&'short mut self) -> FuncInstBuilder<'short, 'a, Variable> {
        let ebb = self.position.ebb;
        FuncInstBuilder::new(self, ebb)
    }
}

/// All the functions documented in the previous block are write-only and help you build a valid
/// Cretonne IL functions via multiple debug asserts. However, you might need to improve the
/// performance of your translation perform more complex transformations to your Cretonne IL
/// function. The functions below help you inspect the function you're creating and modify it
/// in ways that can be unsafe if used incorrectly.
impl<'a, Variable> FunctionBuilder<'a, Variable>
    where Variable: EntityRef + Hash + Default
{
    /// Retrieves all the arguments for an `Ebb` currently infered from the jump instructions
    /// inserted that target it and the SSA construction.
    pub fn ebb_args(&self, ebb: Ebb) -> &[Value] {
        self.func.dfg.ebb_args(ebb)
    }

    /// Retrieves the signature with reference `sigref` previously added with `import_signature`.
    pub fn signature(&self, sigref: SigRef) -> Option<&Signature> {
        self.func.dfg.signatures.get(sigref)
    }

    /// Creates a argument for a specific `Ebb` by appending it to the list of already existing
    /// arguments.
    ///
    /// **Note:** this function has to be called at the creation of the `Ebb` before adding
    /// instructions to it, otherwise this could interfere with SSA construction.
    pub fn append_ebb_arg(&mut self, ebb: Ebb, ty: Type) -> Value {
        debug_assert!(self.builder.ebbs[ebb].pristine);
        self.func.dfg.append_ebb_arg(ebb, ty)
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
        let old_dest =
            self.func.dfg[inst]
                .branch_destination_mut()
                .expect("you want to change the jump destination of a non-jump instruction");
        let pred = self.builder.ssa.remove_ebb_predecessor(*old_dest, inst);
        *old_dest = new_dest;
        self.builder
            .ssa
            .declare_ebb_predecessor(new_dest, pred, inst);
    }

    /// Returns `true` if and only if the current `Ebb` is sealed and has no predecessors declared.
    ///
    /// The entry block of a function is never unreachable.
    pub fn is_unreachable(&self) -> bool {
        let is_entry = match self.func.layout.entry_block() {
            None => false,
            Some(entry) => self.position.ebb == entry,
        };
        (!is_entry && self.builder.ssa.is_sealed(self.position.ebb) &&
         self.builder.ssa.predecessors(self.position.ebb).is_empty())
    }

    /// Returns `true` if and only if no instructions have been added since the last call to
    /// `switch_to_block`.
    pub fn is_pristine(&self) -> bool {
        self.builder.ebbs[self.position.ebb].pristine
    }

    /// Returns `true` if and only if a terminator instruction has been inserted since the
    /// last call to `switch_to_block`.
    pub fn is_filled(&self) -> bool {
        self.builder.ebbs[self.position.ebb].filled
    }

    /// Returns a displayable object for the function as it is.
    ///
    /// Useful for debug purposes. Use it with `None` for standard printing.
    pub fn display<'b, I: Into<Option<&'b TargetIsa>>>(&'b self, isa: I) -> DisplayFunction {
        self.func.display(isa)
    }
}

impl<'a, Variable> Drop for FunctionBuilder<'a, Variable>
    where Variable: EntityRef + Hash + Default
{
    /// When a `FunctionBuilder` goes out of scope, it means that the function is fully built.
    /// We then proceed to check if all the `Ebb`s are filled and sealed
    fn drop(&mut self) {
        debug_assert!(self.builder
                          .ebbs
                          .keys()
                          .all(|ebb| {
                                   self.builder.ebbs[ebb].pristine ||
                                   (self.builder.ssa.is_sealed(ebb) &&
                                    self.builder.ebbs[ebb].filled)
                               }),
                      "all blocks should be filled and sealed before dropping a FunctionBuilder")
    }
}

// Helper functions
impl<'a, Variable> FunctionBuilder<'a, Variable>
    where Variable: EntityRef + Hash + Default
{
    fn move_to_next_basic_block(&mut self) {
        self.position.basic_block = self.builder
            .ssa
            .declare_ebb_body_block(self.position.basic_block);
    }

    fn fill_current_block(&mut self) {
        self.builder.ebbs[self.position.ebb].filled = true;
    }

    fn declare_successor(&mut self, dest_ebb: Ebb, jump_inst: Inst) {
        self.builder
            .ssa
            .declare_ebb_predecessor(dest_ebb, self.position.basic_block, jump_inst);
    }

    fn check_return_args(&self, args: &[Value]) {
        debug_assert_eq!(args.len(),
                         self.func.signature.return_types.len(),
                         "the number of returned values doesn't match the function signature ");
        for (i, arg) in args.iter().enumerate() {
            let valty = self.func.dfg.value_type(*arg);
            debug_assert_eq!(valty,
                             self.func.signature.return_types[i].value_type,
                             "the types of the values returned don't match the \
                             function signature");
        }
    }

    fn fill_function_args_values(&mut self, ebb: Ebb) {
        debug_assert!(self.pristine);
        for argtyp in self.func.signature.argument_types.iter() {
            self.builder
                .function_args_values
                .push(self.func.dfg.append_ebb_arg(ebb, argtyp.value_type));
        }
        self.pristine = false;
    }


    fn ebb_args_adjustement(&mut self, dest_ebb: Ebb, jump_args: &[Type]) {
        let ty_to_append: Option<Vec<Type>> =
            if self.builder.ssa.predecessors(dest_ebb).len() == 0 ||
               self.builder.ebbs[dest_ebb].pristine {
                // This is the first jump instruction targeting this Ebb
                // so the jump arguments supplied here are this Ebb' arguments
                // However some of the arguments might already be there
                // in the Ebb so we have to check they're consistent
                let dest_ebb_args = self.func.dfg.ebb_args(dest_ebb);
                debug_assert!(dest_ebb_args
                                  .iter()
                                  .zip(jump_args.iter().take(dest_ebb_args.len()))
                                  .all(|(dest_arg, jump_arg)| {
                                           *jump_arg == self.func.dfg.value_type(*dest_arg)
                                       }),
                              "the jump argument supplied has not the \
                same type as the corresponding dest ebb argument");
                self.builder.ebbs[dest_ebb].user_arg_count = jump_args.len();
                Some(jump_args
                         .iter()
                         .skip(dest_ebb_args.len())
                         .cloned()
                         .collect())
            } else {
                let dest_ebb_args = self.func.dfg.ebb_args(dest_ebb);
                // The Ebb already has predecessors
                // We check that the arguments supplied match those supplied
                // previously.
                debug_assert!(jump_args.len() == self.builder.ebbs[dest_ebb].user_arg_count,
                              "the jump instruction doesn't have the same \
                      number of arguments as its destination Ebb \
                      ({} vs {}).",
                              jump_args.len(),
                              dest_ebb_args.len());
                debug_assert!(jump_args
                                  .iter()
                                  .zip(dest_ebb_args
                                      .iter()
                                      .take(self.builder.ebbs[dest_ebb].user_arg_count)
                                  )
                                  .all(|(jump_arg, dest_arg)| {
                                           *jump_arg == self.func.dfg.value_type(*dest_arg)
                                       }),
                              "the jump argument supplied has not the \
                    same type as the corresponding dest ebb argument");
                None
            };
        if let Some(ty_args) = ty_to_append {
            for ty in ty_args {
                self.func.dfg.append_ebb_arg(dest_ebb, ty);
            }
        }
    }

    fn handle_ssa_side_effects(&mut self, side_effects: SideEffects) {
        for split_ebb in side_effects.split_ebbs_created {
            self.builder.ebbs.ensure(split_ebb).filled = true
        }
        for modified_ebb in side_effects.instructions_added_to_ebbs {
            self.builder.ebbs[modified_ebb].pristine = false
        }
    }
}

#[cfg(test)]
mod tests {

    use cretonne::entity_ref::EntityRef;
    use cretonne::ir::{FunctionName, Function, CallConv, Signature, ArgumentType, InstBuilder};
    use cretonne::ir::types::*;
    use frontend::{ILBuilder, FunctionBuilder};
    use cretonne::verifier::verify_function;

    use std::u32;

    // An opaque reference to variable.
    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
    pub struct Variable(u32);
    impl EntityRef for Variable {
        fn new(index: usize) -> Self {
            assert!(index < (u32::MAX as usize));
            Variable(index as u32)
        }

        fn index(self) -> usize {
            self.0 as usize
        }
    }
    impl Default for Variable {
        fn default() -> Variable {
            Variable(u32::MAX)
        }
    }

    #[test]
    fn sample_function() {
        let mut sig = Signature::new(CallConv::Native);
        sig.return_types.push(ArgumentType::new(I32));
        sig.argument_types.push(ArgumentType::new(I32));

        let mut il_builder = ILBuilder::<Variable>::new();
        let mut func = Function::with_name_signature(FunctionName::new("sample_function"), sig);
        {
            let mut builder = FunctionBuilder::<Variable>::new(&mut func, &mut il_builder);

            let block0 = builder.create_ebb();
            let block1 = builder.create_ebb();
            let block2 = builder.create_ebb();
            let x = Variable(0);
            let y = Variable(1);
            let z = Variable(2);
            builder.declare_var(x, I32);
            builder.declare_var(y, I32);
            builder.declare_var(z, I32);

            builder.switch_to_block(block0, &[]);
            builder.seal_block(block0);
            {
                let tmp = builder.arg_value(0);
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

            builder.switch_to_block(block1, &[]);
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

            builder.switch_to_block(block2, &[]);
            builder.seal_block(block2);

            {
                let arg1 = builder.use_var(y);
                let arg2 = builder.use_var(x);
                let tmp = builder.ins().isub(arg1, arg2);
                builder.def_var(y, tmp);
            }
            builder.ins().jump(block1, &[]);
            builder.seal_block(block1);
        }

        let res = verify_function(&func, None);
        // println!("{}", func.display(None));
        match res {
            Ok(_) => {}
            Err(err) => panic!("{}{}", func.display(None), err),
        }
    }
}
