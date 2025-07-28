//! Generating series of `table.get` and `table.set` operations.
use mutatis::mutators as m;
use mutatis::{Candidates, Context, DefaultMutate, Generate, Mutate, Result as MutResult};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::ops::RangeInclusive;
use wasm_encoder::{
    CodeSection, ConstExpr, EntityType, ExportKind, ExportSection, Function, FunctionSection,
    GlobalSection, ImportSection, Instruction, Module, RefType, TableSection, TableType,
    TypeSection, ValType,
};

/// A description of a Wasm module that makes a series of `externref` table
/// operations.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TableOps {
    pub(crate) num_params: u32,
    pub(crate) num_globals: u32,
    pub(crate) table_size: i32,
    pub(crate) ops: Vec<TableOp>,
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
    pub fn abstract_stack_depth(&self, index: usize) -> usize {
        debug_assert!(index <= self.ops.len());
        let mut stack: usize = 0;
        for op in self.ops.iter().take(index) {
            let pop = op.operands_len();
            let push = op.results_len();
            stack = stack.saturating_sub(pop);
            stack += push;
        }
        stack
    }

    /// Fixes the stack after mutating the `idx`th op.
    ///
    /// The abstract stack depth starting at the `idx`th opcode must be `stack`.
    fn fixup(&mut self, idx: usize, mut stack: usize) {
        let mut new_ops = Vec::with_capacity(self.ops.len());
        new_ops.extend_from_slice(&self.ops[..idx]);

        // Iterate through all ops including and after `idx`, inserting a null
        // ref for any missing operands when they want to pop more operands than
        // exist on the stack.
        new_ops.extend(self.ops[idx..].iter().copied().flat_map(|op| {
            let mut temp = SmallVec::<[_; 4]>::new();

            while stack < op.operands_len() {
                temp.push(TableOp::Null());
                stack += 1;
            }

            temp.push(op);
            stack = stack - op.operands_len() + op.results_len();

            temp
        }));

        // Now make sure that the stack is empty at the end of the ops by
        // inserting drops as necessary.
        for _ in 0..stack {
            new_ops.push(TableOp::Drop());
        }

        self.ops = new_ops;
    }

    /// Attempts to remove the last opcode from the sequence.
    ///
    /// Returns `true` if an opcode was successfully removed, or `false` if the list was already empty.
    pub fn pop(&mut self) -> bool {
        self.ops.pop().is_some()
    }
}

/// A mutator for the table ops
#[derive(Debug)]
pub struct TableOpsMutator;

impl Mutate<TableOps> for TableOpsMutator {
    fn mutate(&mut self, c: &mut Candidates<'_>, ops: &mut TableOps) -> mutatis::Result<()> {
        // Insert
        if !c.shrink() {
            c.mutation(|ctx| {
                if let Some(idx) = ctx.rng().gen_index(ops.ops.len() + 1) {
                    let stack = ops.abstract_stack_depth(idx);
                    let (op, _new_stack_size) = TableOp::generate(ctx, &ops, stack)?;
                    ops.ops.insert(idx, op);
                    ops.fixup(idx, stack);
                }
                Ok(())
            })?;
        }

        // Remove
        if !ops.ops.is_empty() {
            c.mutation(|ctx| {
                let idx = ctx
                    .rng()
                    .gen_index(ops.ops.len())
                    .expect("ops is not empty");
                let stack = ops.abstract_stack_depth(idx);
                ops.ops.remove(idx);
                ops.fixup(idx, stack);
                Ok(())
            })?;
        }

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

impl<'a> arbitrary::Arbitrary<'a> for TableOps {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let mut session = mutatis::Session::new().seed(u.arbitrary()?);
        session
            .generate()
            .map_err(|_| arbitrary::Error::IncorrectFormat)
    }
}

impl Generate<TableOps> for TableOpsMutator {
    fn generate(&mut self, ctx: &mut Context) -> MutResult<TableOps> {
        let num_params = m::range(NUM_PARAMS_RANGE).generate(ctx)?;
        let num_globals = m::range(NUM_GLOBALS_RANGE).generate(ctx)?;
        let table_size = m::range(TABLE_SIZE_RANGE).generate(ctx)?;

        let mut ops = TableOps {
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

        let mut stack: usize = 0;
        while ops.ops.len() < MAX_OPS {
            let (op, new_stack_len) = TableOp::generate(ctx, &ops, stack)?;
            ops.ops.push(op);
            stack = new_stack_len;
        }

        // Drop any leftover refs on the stack.
        for _ in 0..stack {
            ops.ops.push(TableOp::Drop());
        }

        Ok(ops)
    }
}

macro_rules! define_table_ops {
    (
        $(
            $op:ident $( ( $($limit:expr => $ty:ty),* ) )? : $params:expr => $results:expr ,
        )*
    ) => {
        #[derive(Copy, Clone, Debug, Serialize, Deserialize)]
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

        $(
            #[allow(non_snake_case, reason = "macro-generated code")]
            fn $op(
                _ctx: &mut mutatis::Context,
                _ops: &TableOps,
                stack: usize,
            ) -> mutatis::Result<(TableOp, usize)> {
                #[allow(unused_comparisons, reason = "macro-generated code")]
                {
                    debug_assert!(stack >= $params);
                }

                let op = TableOp::$op(
                    $($({
                        let limit_fn = $limit as fn(&TableOps) -> $ty;
                        let limit = (limit_fn)(_ops);
                        debug_assert!(limit > 0);
                        m::range(0..=limit - 1).generate(_ctx)?
                    })*)?
                );
                let new_stack = stack - $params + $results;
                Ok((op, new_stack))
            }
        )*

        impl TableOp {
            fn generate(
                ctx: &mut mutatis::Context,
                ops: &TableOps,
                stack: usize,
            ) -> mutatis::Result<(TableOp, usize)> {
                let mut valid_choices: Vec<
                    fn (&mut mutatis::Context, &TableOps, usize) -> mutatis::Result<(TableOp, usize)>
                > = vec![];

                $(
                    #[allow(unused_comparisons, reason = "macro-generated code")]
                    if stack >= $params $($(
                        && {
                            let limit_fn: fn(&TableOps) -> $ty = $limit;
                            let limit = (limit_fn)(ops);
                            limit > 0
                        }
                    )*)? {
                        valid_choices.push($op);
                    }
                )*

                let f = *ctx.rng()
                    .choose(&valid_choices)
                    .expect("should always have a valid op choice");

                (f)(ctx, ops, stack)
            }
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

    /// Creates empty TableOps
    fn empty_test_ops(num_params: u32, num_globals: u32, table_size: i32) -> TableOps {
        TableOps {
            num_params,
            num_globals,
            table_size,
            ops: vec![],
        }
    }

    /// Creates TableOps with all default opcodes
    fn test_ops(num_params: u32, num_globals: u32, table_size: i32) -> TableOps {
        TableOps {
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
        }
    }

    #[test]
    fn mutate_table_ops_with_default_mutator() -> mutatis::Result<()> {
        let _ = env_logger::try_init();
        use mutatis::Session;
        use wasmparser::Validator;
        let mut res = test_ops(5, 5, 5);
        let mut session = Session::new();

        for _ in 0..1024 {
            session.mutate(&mut res)?;
            let wasm = res.to_wasm_binary();
            let mut validator = Validator::new();
            let wat = wasmprinter::print_bytes(&wasm).expect("[-] Failed .print_bytes(&wasm).");
            let result = validator.validate_all(&wasm);
            log::debug!("{wat}");
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

        let mut res = empty_test_ops(5, 5, 5);
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

    #[test]
    fn test_wat_string() -> mutatis::Result<()> {
        let _ = env_logger::try_init();

        let mut table_ops = test_ops(2, 2, 5);
        table_ops.ops.extend([
            TableOp::Null(),
            TableOp::Drop(),
            TableOp::Gc(),
            TableOp::LocalSet(0),
            TableOp::LocalGet(0),
            TableOp::GlobalSet(0),
            TableOp::GlobalGet(0),
            TableOp::Null(),
            TableOp::Drop(),
            TableOp::Gc(),
            TableOp::LocalSet(0),
            TableOp::LocalGet(0),
            TableOp::GlobalSet(0),
            TableOp::GlobalGet(0),
            TableOp::Null(),
            TableOp::Drop(),
        ]);
        let wasm = table_ops.to_wasm_binary();
        let wat = wasmprinter::print_bytes(&wasm).expect("Failed to convert to WAT");
        let expected = r#"
        (module
        (type (;0;) (func (result externref externref externref)))
        (type (;1;) (func (param externref externref)))
        (type (;2;) (func (param externref externref externref)))
        (type (;3;) (func (result externref externref externref)))
        (import "" "gc" (func (;0;) (type 0)))
        (import "" "take_refs" (func (;1;) (type 2)))
        (import "" "make_refs" (func (;2;) (type 3)))
        (table (;0;) 5 externref)
        (global (;0;) (mut externref) ref.null extern)
        (global (;1;) (mut externref) ref.null extern)
        (export "run" (func 3))
        (func (;3;) (type 1) (param externref externref)
            (local externref)
            loop ;; label = @1
            ref.null extern
            drop
            call 0
            local.set 0
            local.get 0
            global.set 0
            global.get 0
            ref.null extern
            drop
            call 0
            local.set 0
            local.get 0
            global.set 0
            global.get 0
            ref.null extern
            drop
            call 0
            local.set 0
            local.get 0
            global.set 0
            global.get 0
            ref.null extern
            drop
            br 0 (;@1;)
            end
        )
        )
        "#;

        let generated = wat.split_whitespace().collect::<Vec<_>>().join(" ");
        let expected = expected.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(generated, expected, "WAT does not match expected");

        Ok(())
    }
}
