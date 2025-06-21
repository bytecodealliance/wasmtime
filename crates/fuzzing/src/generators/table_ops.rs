//! Generating series of `table.get` and `table.set` operations.
use mutatis::mutators as m;
use mutatis::{Candidates, Context, DefaultMutate, Generate, Mutate, Result as MutResult};
use std::ops::RangeInclusive;
use wasm_encoder::{
    CodeSection, ConstExpr, EntityType, ExportKind, ExportSection, Function, FunctionSection,
    GlobalSection, ImportSection, Instruction, Module, RefType, TableSection, TableType,
    TypeSection, ValType,
};
/// A description of a Wasm module that makes a series of `externref` table
/// operations.
#[derive(Debug)]
pub struct TableOps {
    pub(crate) num_params: u32,
    pub(crate) num_globals: u32,
    pub(crate) table_size: i32,
    ops: Vec<TableOp>,
}

const NUM_PARAMS_RANGE: RangeInclusive<u32> = 0..=10;
const NUM_GLOBALS_RANGE: RangeInclusive<u32> = 0..=10;
const TABLE_SIZE_RANGE: RangeInclusive<i32> = 0..=100;
const MAX_OPS: usize = 100;

impl TableOps {
    /// Serialize this module into a Wasm binary.
    ///
    /// The module requires several function imports. See this function's
    /// implementation for their exact types.
    ///
    /// The single export of the module is a function "run" that takes
    /// `self.num_params` parameters of type `externref`.
    ///
    /// The "run" function does not terminate; you should run it with limited
    /// fuel. It also is not guaranteed to avoid traps: it may access
    /// out-of-bounds of the table.
    pub fn to_wasm_binary(&self) -> Vec<u8> {
        let mut module = Module::new();

        // Encode the types for all functions that we are using.
        let mut types = TypeSection::new();

        // 0: "gc"
        types.ty().function(
            vec![],
            // Return a bunch of stuff from `gc` so that we exercise GCing when
            // there is return pointer space allocated on the stack. This is
            // especially important because the x64 backend currently
            // dynamically adjusts the stack pointer for each call that uses
            // return pointers rather than statically allocating space in the
            // stack frame.
            vec![ValType::EXTERNREF, ValType::EXTERNREF, ValType::EXTERNREF],
        );

        // 1: "run"
        let mut params: Vec<ValType> = Vec::with_capacity(self.num_params as usize);
        for _i in 0..self.num_params {
            params.push(ValType::EXTERNREF);
        }
        let results = vec![];
        types.ty().function(params, results);

        // 2: `take_refs`
        types.ty().function(
            vec![ValType::EXTERNREF, ValType::EXTERNREF, ValType::EXTERNREF],
            vec![],
        );

        // 3: `make_refs`
        types.ty().function(
            vec![],
            vec![ValType::EXTERNREF, ValType::EXTERNREF, ValType::EXTERNREF],
        );

        // Import the GC function.
        let mut imports = ImportSection::new();
        imports.import("", "gc", EntityType::Function(0));
        imports.import("", "take_refs", EntityType::Function(2));
        imports.import("", "make_refs", EntityType::Function(3));

        // Define our table.
        let mut tables = TableSection::new();
        tables.table(TableType {
            element_type: RefType::EXTERNREF,
            minimum: self.table_size as u64,
            maximum: None,
            table64: false,
            shared: false,
        });

        // Define our globals.
        let mut globals = GlobalSection::new();
        for _ in 0..self.num_globals {
            globals.global(
                wasm_encoder::GlobalType {
                    val_type: wasm_encoder::ValType::EXTERNREF,
                    mutable: true,
                    shared: false,
                },
                &ConstExpr::ref_null(wasm_encoder::HeapType::EXTERN),
            );
        }

        // Define the "run" function export.
        let mut functions = FunctionSection::new();
        functions.function(1);

        let mut exports = ExportSection::new();
        exports.export("run", ExportKind::Func, 3);

        // Give ourselves one scratch local that we can use in various `TableOp`
        // implementations.
        let mut func = Function::new(vec![(1, ValType::EXTERNREF)]);

        func.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        for op in &self.ops {
            op.insert(&mut func, self.num_params);
        }
        func.instruction(&Instruction::Br(0));
        func.instruction(&Instruction::End);
        func.instruction(&Instruction::End);

        let mut code = CodeSection::new();
        code.function(&func);

        module
            .section(&types)
            .section(&imports)
            .section(&functions)
            .section(&tables)
            .section(&globals)
            .section(&exports)
            .section(&code);

        module.finish()
    }
    /// Computes the abstract stack depth after executing all operations
    pub fn abstract_stack_depth(&self) -> usize {
        let mut stack: usize = 0;
        for op in &self.ops {
            let pop = op.operands_len();
            let push = op.results_len();
            stack = stack.saturating_sub(pop);
            stack += push;
        }
        stack
    }
}

/// A mutator for the table ops
#[derive(Debug)]
pub struct TableOpsMutator;

impl Mutate<TableOps> for TableOpsMutator {
    fn mutate(&mut self, c: &mut Candidates<'_>, ops: &mut TableOps) -> mutatis::Result<()> {
        //  1. Append
        c.mutation(|ctx| {
            let mut stack = ops.abstract_stack_depth();
            add_table_op_mutatis(ops, ctx, &mut stack)
        })?;
        // 2. Remove
        c.mutation(|ctx| {
            if ops.ops.is_empty() {
                return Ok(());
            }
            if let Some(idx) = ctx.rng().gen_index(ops.ops.len()) {
                let removed = ops.ops.remove(idx);
                log::debug!("Remove called at idx: {} with opcode {:?}", idx, removed);
                let mut stack = 0isize;
                for op in &ops.ops[..idx] {
                    stack += op.results_len() as isize;
                    stack -= op.operands_len() as isize;
                }
                stack -= removed.results_len() as isize;
                stack += removed.operands_len() as isize;

                log::debug!(
                    "op = {:?}, pop = {}, push = {}, resulting stack = {}",
                    removed,
                    removed.operands_len(),
                    removed.results_len(),
                    stack
                );
                for _ in 0..removed.results_len() {
                    ops.ops.insert(idx, TableOp::Null());
                    log::debug!(
                        "Patched with ref.null at idx {} after removing {:?}",
                        idx,
                        removed
                    );
                    stack += 1;
                }
            }
            Ok(())
        })?;
        // 3. Replace
        c.mutation(|ctx| {
            if ops.ops.is_empty() {
                return Ok(());
            }

            let i = match ctx.rng().gen_index(ops.ops.len()) {
                Some(i) => i,
                None => return Ok(()),
            };

            let old_op = ops.ops.remove(i);
            let old_push = old_op.results_len();
            let old_pop = old_op.operands_len();

            // Stack at point of removal
            let mut temp_ops = TableOps {
                num_params: ops.num_params,
                num_globals: ops.num_globals,
                table_size: ops.table_size,
                ops: ops.ops[..i].to_vec(),
            };

            let mut stack = temp_ops.abstract_stack_depth();

            while stack
                < (old_pop as isize).try_into().expect(
                    "[-] Failed to convert old_pop to usize: value may be negative or too large",
                )
            {
                ops.ops.insert(i, TableOp::Null());
                stack += 1;
                log::debug!("Patching before replace at idx {i} due to underflow");
            }
            let mut new_ops = vec![];
            let mut total_push = 0;
            let mut total_pop = 0;
            let mut attempts = 0;
            while (total_push < old_push || total_pop < old_pop) && attempts < 10 {
                attempts += 1;
                add_table_op_mutatis(&mut temp_ops, ctx, &mut stack).ok();
                if let Some(new_op) = temp_ops.ops.pop() {
                    total_push += new_op.results_len();
                    total_pop += new_op.operands_len();
                    new_ops.push(new_op);
                }
            }
            if total_push == old_push && total_pop == old_pop {
                log::debug!(
                    "Replacing op at idx {i}: {:?} (pop {}, push {}) with {} op(s): [ {} ]",
                    old_op,
                    old_pop,
                    old_push,
                    new_ops.len(),
                    new_ops
                        .iter()
                        .map(|op| format!("{:?}", op))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                for (j, op) in new_ops.into_iter().enumerate() {
                    ops.ops.insert(i + j, op);
                }
            } else {
                ops.ops.insert(i, old_op);
                log::debug!(
                    "Replace at idx {i} failed: could not generate matching ops (pop {}, push {})",
                    old_pop,
                    old_push
                );
            }
            Ok(())
        })?;
        Ok(())
    }
}

impl DefaultMutate for TableOps {
    type DefaultMutate = TableOpsMutator;
}

impl Default for TableOpsMutator {
    fn default() -> Self {
        TableOpsMutator
    }
}

impl Generate<TableOps> for TableOpsMutator {
    fn generate(&mut self, ctx: &mut Context) -> MutResult<TableOps> {
        let num_params = m::range(NUM_PARAMS_RANGE).generate(ctx)?;
        let num_globals = m::range(NUM_GLOBALS_RANGE).generate(ctx)?;
        let table_size = m::range(TABLE_SIZE_RANGE).generate(ctx)?;
        let mut ops = Vec::new();
        let stack = 0u32;

        let mut temp_ops = TableOps {
            num_params,
            num_globals,
            table_size,
            ops: vec![
                TableOp::Null(),
                TableOp::Drop(),
                TableOp::Gc(),
                TableOp::LocalSet(0),
                TableOp::LocalGet(0),
                TableOp::GlobalSet(0),
                TableOp::GlobalGet(0),
            ],
        };
        while ops.len() < MAX_OPS {
            temp_ops.ops = ops.clone();
            let mut stack = temp_ops.abstract_stack_depth();
            let add_result = add_table_op_mutatis(&mut temp_ops, ctx, &mut stack);

            if let Ok(()) = add_result {
                if let Some(last) = temp_ops.ops.last() {
                    ops.push(*last);
                }
            } else {
                break;
            }
        }
        for _ in 0..stack {
            ops.push(TableOp::Drop());
        }
        Ok(TableOps {
            num_params,
            num_globals,
            table_size,
            ops,
        })
    }
}

macro_rules! define_table_ops {
    (
        $(
            $op:ident $( ( $($limit:expr => $ty:ty),* ) )? : $params:expr => $results:expr ,
        )*
    ) => {
        #[derive(Copy, Clone, Debug)]
        pub(crate) enum TableOp {
            $(
                $op ( $( $($ty),* )? ),
            )*
        }
        #[cfg(test)]
        const OP_NAMES: &'static[&'static str] = &[
            $(
                stringify!($op),
            )*
        ];

        impl TableOp {
            #[cfg(test)]
            fn name(&self) -> &'static str  {
                match self {
                    $(
                        Self::$op (..) => stringify!($op),
                    )*
                }
            }

            pub fn operands_len(&self) -> usize {
                match self {
                    $(
                        Self::$op (..) => $params,
                    )*
                }
            }

            pub fn results_len(&self) -> usize {
                match self {
                    $(
                        Self::$op (..) => $results,
                    )*
                }
            }
        }

        type TableOpConstructor = fn(
                    &mut TableOps,
                    &mut mutatis::Context,
                    &mut usize
                ) -> Option<TableOp>;

        $(
            #[allow(unused_variables, unused_comparisons, non_snake_case)]
            fn $op(
                ops: &mut TableOps,
                ctx: &mut mutatis::Context,
                stack: &mut usize
            ) -> Option<TableOp> {
                if $( $(($limit as fn(&TableOps) -> $ty)(ops) > 0 &&)* )? *stack >= $params {
                    let result = TableOp::$op(
                        $($(
                            m::range(0..=($limit as fn(&TableOps) -> $ty)(ops) - 1)
                                .generate(ctx)
                                .ok()?,
                        )*)?
                    );
                    *stack = *stack - $params + $results;
                    Some(result)
                } else {
                    None
                }
            }
        )*
        const TABLE_OP_GENERATORS: &[TableOpConstructor] = &[
            $(
                $op,
            )*
        ];
        fn add_table_op_mutatis(
            ops: &mut TableOps,
            ctx: &mut mutatis::Context,
            stack: &mut usize,
        ) -> mutatis::Result<()> {
            let mut valid_choices = vec![];
            for f in TABLE_OP_GENERATORS {
                let mut temp_stack = *stack;
                if let Some(op) = f(ops, ctx, &mut temp_stack) {
                    valid_choices.push((f, op, temp_stack));
                }
            }
            let (_, op, new_stack) = *ctx.rng()
                .choose(&valid_choices)
                .expect("[-] .choose(&valid_choices) failed.");

            *stack = new_stack;
            ops.ops.push(op);
            Ok(())
        }
    };
}

define_table_ops! {
    Gc : 0 => 3,

    MakeRefs : 0 => 3,
    TakeRefs : 3 => 0,

    // Add one to make sure that out of bounds table accesses are possible, but still rare.
    TableGet(|ops| ops.table_size + 1 => i32) : 0 => 1,
    TableSet(|ops| ops.table_size + 1 => i32) : 1 => 0,

    GlobalGet(|ops| ops.num_globals => u32) : 0 => 1,
    GlobalSet(|ops| ops.num_globals => u32) : 1 => 0,

    LocalGet(|ops| ops.num_params => u32) : 0 => 1,
    LocalSet(|ops| ops.num_params => u32) : 1 => 0,

    Drop : 1 => 0,

    Null : 0 => 1,
}

impl TableOp {
    fn insert(self, func: &mut Function, scratch_local: u32) {
        let gc_func_idx = 0;
        let take_refs_func_idx = 1;
        let make_refs_func_idx = 2;

        match self {
            Self::Gc() => {
                func.instruction(&Instruction::Call(gc_func_idx));
            }
            Self::MakeRefs() => {
                func.instruction(&Instruction::Call(make_refs_func_idx));
            }
            Self::TakeRefs() => {
                func.instruction(&Instruction::Call(take_refs_func_idx));
            }
            Self::TableGet(x) => {
                func.instruction(&Instruction::I32Const(x));
                func.instruction(&Instruction::TableGet(0));
            }
            Self::TableSet(x) => {
                func.instruction(&Instruction::LocalSet(scratch_local));
                func.instruction(&Instruction::I32Const(x));
                func.instruction(&Instruction::LocalGet(scratch_local));
                func.instruction(&Instruction::TableSet(0));
            }
            Self::GlobalGet(x) => {
                func.instruction(&Instruction::GlobalGet(x));
            }
            Self::GlobalSet(x) => {
                func.instruction(&Instruction::GlobalSet(x));
            }
            Self::LocalGet(x) => {
                func.instruction(&Instruction::LocalGet(x));
            }
            Self::LocalSet(x) => {
                func.instruction(&Instruction::LocalSet(x));
            }
            Self::Drop() => {
                func.instruction(&Instruction::Drop);
            }
            Self::Null() => {
                func.instruction(&Instruction::RefNull(wasm_encoder::HeapType::EXTERN));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    macro_rules! default_table_ops {
        ($num_params:expr, $num_globals:expr, $table_size:expr) => {
            TableOps {
                num_params: $num_params,
                num_globals: $num_globals,
                table_size: $table_size,
                ops: vec![
                    TableOp::Null(),
                    TableOp::Drop(),
                    TableOp::Gc(),
                    TableOp::LocalSet(0),
                    TableOp::LocalGet(0),
                    TableOp::GlobalSet(0),
                    TableOp::GlobalGet(0),
                ],
            }
        };
    }

    macro_rules! empty_table_ops {
        ($num_params:expr, $num_globals:expr, $table_size:expr) => {
            TableOps {
                num_params: $num_params,
                num_globals: $num_globals,
                table_size: $table_size,
                ops: vec![],
            }
        };
    }

    #[test]
    fn mutate_table_ops_with_default_mutator() -> mutatis::Result<()> {
        let _ = env_logger::try_init();
        use mutatis::Session;
        use wasmparser::Validator;
        let mut res = default_table_ops![5, 5, 5];
        let mut session = Session::new();

        for _ in 0..1024 {
            session.mutate(&mut res)?;
            let wasm = res.to_wasm_binary();
            let mut validator = Validator::new();
            let wat = wasmprinter::print_bytes(&wasm).expect("[-] Failed .print_bytes(&wasm).");
            let result = validator.validate_all(&wasm);
            assert!(
                result.is_ok(),
                "\n[-] Invalid wat: {}\n\t\t==== Failed Wat ====\n{}",
                result.err().expect("[-] Failed .err() in assert macro."),
                wat
            );
        }
        Ok(())
    }

    #[test]
    fn test_tableops_mutate_with() -> mutatis::Result<()> {
        let _ = env_logger::try_init();
        let mut res = default_table_ops![5, 5, 5];
        let mut generator = TableOpsMutator;
        let mut session = mutatis::Session::new();

        for _ in 0..=1024 {
            session.mutate_with(&mut generator, &mut res)?;
            let wasm = res.to_wasm_binary();
            let mut validator = wasmparser::Validator::new();
            let result = validator.validate_all(&wasm);
            let wat = wasmprinter::print_bytes(&wasm).unwrap();
            println!("{wat}");
            assert!(
                result.is_ok(),
                "\n[-] Invalid wat: {}\n\t\t==== Failed Wat ====\n{}",
                result.err().expect("[-] Failed .err() in assert macro."),
                wat
            );
        }
        Ok(())
    }

    #[test]
    fn every_op_generated() -> mutatis::Result<()> {
        let _ = env_logger::try_init();
        let mut unseen_ops: std::collections::HashSet<_> = OP_NAMES.iter().copied().collect();

        let mut res = empty_table_ops![5, 5, 5];
        let mut generator = TableOpsMutator;
        let mut session = mutatis::Session::new();

        'outer: for _ in 0..=1024 {
            session.mutate_with(&mut generator, &mut res)?;
            for op in &res.ops {
                unseen_ops.remove(op.name());
                if unseen_ops.is_empty() {
                    break 'outer;
                }
            }
        }
        assert!(unseen_ops.is_empty(), "Failed to generate {unseen_ops:?}");
        Ok(())
    }
    // TODO: Write a test for expected wat
    #[test]
    fn test_wat_string() -> mutatis::Result<()> {
        let _ = env_logger::try_init();
        Ok(())
    }
}
