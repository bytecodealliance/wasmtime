//! Generating series of `table.get` and `table.set` operations.
use mutatis::mutators as m;
use mutatis::{Candidates, Context, DefaultMutate, Generate, Mutate, Result as MutResult};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::ops::RangeInclusive;
use wasm_encoder::{
    CodeSection, CompositeInnerType, CompositeType, ConstExpr, EntityType, ExportKind,
    ExportSection, Function, FunctionSection, GlobalSection, ImportSection, Instruction, Module,
    RefType, StructType, SubType, TableSection, TableType, TypeSection, ValType,
};

/// RecGroup ID struct definition.
#[derive(Debug, Clone, Eq, PartialOrd, PartialEq, Ord, Hash, Default, Serialize, Deserialize)]
pub struct RecGroupId(u32);

/// Struct types definition.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StructTypes {
    next_group: u32,
    rec_groups: std::collections::BTreeSet<RecGroupId>,
}

impl StructTypes {
    /// Create a fresh `StructTypes` allocator with no recursive groups defined yet.
    pub fn new() -> Self {
        Self {
            next_group: 0,
            rec_groups: Default::default(),
        }
    }

    /// Allocate a new empty recursive group `(rec ...)` and return its unique `RecGroupId`.
    pub fn alloc_empty_rec_group(&mut self) -> RecGroupId {
        let id = RecGroupId(self.next_group);
        self.next_group += 1;
        self.rec_groups.insert(id.clone());
        id
    }

    /// Iterate over all allocated recursive groups.
    pub fn groups(&self) -> impl Iterator<Item = &RecGroupId> {
        self.rec_groups.iter()
    }
}

/// Limits controlling the structure of a generated Wasm module.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TableOpsLimits {
    pub(crate) num_params: u32,
    pub(crate) num_globals: u32,
    pub(crate) table_size: i32,
    pub(crate) num_rec_groups: u32,
    pub(crate) empty_structs_per_group: u32,
}

impl TableOpsLimits {
    /// Calculates and returns the total number of struct types the configuration describes
    #[inline]
    pub fn total_struct_types(&self) -> u32 {
        self.num_rec_groups
            .saturating_mul(self.empty_structs_per_group)
    }
}

/// A description of a Wasm module that makes a series of `externref` table
/// operations.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TableOps {
    pub(crate) limits: TableOpsLimits,
    pub(crate) ops: Vec<TableOp>,
    pub(crate) struct_types: StructTypes,
}

const BASE_STRUCT_TYPE_INDEX: u32 = 4;

const NUM_PARAMS_RANGE: RangeInclusive<u32> = 0..=10;
const NUM_GLOBALS_RANGE: RangeInclusive<u32> = 0..=10;
const TABLE_SIZE_RANGE: RangeInclusive<i32> = 0..=100;
const NUM_REC_GROUPS_RANGE: RangeInclusive<u32> = 0..=10;
const EMPTY_STRUCTS_PER_GROUP_RANGE: RangeInclusive<u32> = 1..=11;
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
    pub fn to_wasm_binary(&mut self) -> Vec<u8> {
        // Clamp limits to generate opcodes within bounds
        self.limits.table_size = self
            .limits
            .table_size
            .clamp(*TABLE_SIZE_RANGE.start(), *TABLE_SIZE_RANGE.end());

        self.limits.num_params = self
            .limits
            .num_params
            .clamp(*NUM_PARAMS_RANGE.start(), *NUM_PARAMS_RANGE.end());

        self.limits.num_globals = self
            .limits
            .num_globals
            .clamp(*NUM_GLOBALS_RANGE.start(), *NUM_GLOBALS_RANGE.end());

        self.limits.num_rec_groups = self
            .limits
            .num_rec_groups
            .clamp(*NUM_REC_GROUPS_RANGE.start(), *NUM_REC_GROUPS_RANGE.end());

        self.limits.empty_structs_per_group = self.limits.empty_structs_per_group.clamp(
            *EMPTY_STRUCTS_PER_GROUP_RANGE.start(),
            *EMPTY_STRUCTS_PER_GROUP_RANGE.end(),
        );

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
        let mut params: Vec<ValType> = Vec::with_capacity(self.limits.num_params as usize);
        for _i in 0..self.limits.num_params {
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
            minimum: self.limits.table_size as u64,
            maximum: None,
            table64: false,
            shared: false,
        });

        // Define our globals.
        let mut globals = GlobalSection::new();
        for _ in 0..self.limits.num_globals {
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
            op.insert(&mut func, self.limits.num_params);
        }
        func.instruction(&Instruction::Br(0));
        func.instruction(&Instruction::End);
        func.instruction(&Instruction::End);

        for _gid in self.struct_types.groups() {
            let n = self.limits.empty_structs_per_group;
            let mut subtys = Vec::with_capacity(n as usize);
            for _ in 0..n {
                subtys.push(SubType {
                    is_final: true,
                    supertype_idx: None,
                    composite_type: CompositeType {
                        inner: CompositeInnerType::Struct(StructType {
                            fields: Box::new([]),
                        }),
                        shared: false,
                    },
                });
            }
            types.ty().rec(subtys);
        }

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
    ///
    fn fixup(&mut self) {
        let mut new_ops = Vec::with_capacity(self.ops.len());
        let mut stack = 0;

        for mut op in self.ops.iter().copied() {
            op.fixup(&self.limits);

            let mut temp = SmallVec::<[_; 4]>::new();

            while stack < op.operands_len() {
                temp.push(TableOp::Null());
                stack += 1;
            }

            temp.push(op);
            stack = stack - op.operands_len() + op.results_len();

            new_ops.extend(temp);
        }

        // Insert drops to balance the final stack state
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
        if !c.shrink() {
            c.mutation(|ctx| {
                if let Some(idx) = ctx.rng().gen_index(ops.ops.len() + 1) {
                    let stack = ops.abstract_stack_depth(idx);
                    let (op, _new_stack_size) = TableOp::generate(ctx, &ops, stack)?;
                    ops.ops.insert(idx, op);
                    ops.fixup();
                }
                Ok(())
            })?;
        }
        if !ops.ops.is_empty() {
            c.mutation(|ctx| {
                let idx = ctx
                    .rng()
                    .gen_index(ops.ops.len())
                    .expect("ops is not empty");
                ops.ops.remove(idx);
                ops.fixup();
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

        let num_rec_groups = m::range(NUM_REC_GROUPS_RANGE).generate(ctx)?;
        let empty_structs_per_group = m::range(EMPTY_STRUCTS_PER_GROUP_RANGE).generate(ctx)?;

        let mut ops = TableOps {
            limits: TableOpsLimits {
                num_params,
                num_globals,
                table_size,
                num_rec_groups,
                empty_structs_per_group,
            },
            ops: vec![
                TableOp::Null(),
                TableOp::Drop(),
                TableOp::Gc(),
                TableOp::LocalSet(0),
                TableOp::LocalGet(0),
                TableOp::GlobalSet(0),
                TableOp::GlobalGet(0),
            ],
            struct_types: StructTypes::new(),
        };

        for _ in 0..ops.limits.num_rec_groups {
            ops.struct_types.alloc_empty_rec_group();
        }

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
            $op:ident $( ( $($limit_var:ident : $limit:expr => $ty:ty),* ) )? : $params:expr => $results:expr ,
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
                _limits: &TableOpsLimits,
                stack: usize,
            ) -> mutatis::Result<(TableOp, usize)> {
                #[allow(unused_comparisons, reason = "macro-generated code")]
                {
                    debug_assert!(stack >= $params);
                }

                let op = TableOp::$op(
                    $($({
                        let limit_fn = $limit as fn(&TableOpsLimits) -> $ty;
                        let limit = (limit_fn)(_limits);
                        debug_assert!(limit > 0);
                        m::range(0..=limit - 1).generate(_ctx)?
                    })*)?
                );
                let new_stack = stack - $params + $results;
                Ok((op, new_stack))
            }
        )*

        impl TableOp {
            fn fixup(&mut self, limits: &TableOpsLimits) {
                match self {
                    $(
                        Self::$op( $( $( $limit_var ),* )? ) => {
                            $( $(
                                let limit_fn = $limit as fn(&TableOpsLimits) -> $ty;
                                let limit = (limit_fn)(limits);
                                debug_assert!(limit > 0);
                                *$limit_var = *$limit_var % limit;
                            )* )?
                        }
                    )*
                }
            }

            fn generate(
                ctx: &mut mutatis::Context,
                ops: &TableOps,
                stack: usize,
            ) -> mutatis::Result<(TableOp, usize)> {
                let mut valid_choices: Vec<
                    fn(&mut Context, &TableOpsLimits, usize) -> mutatis::Result<(TableOp, usize)>
                > = vec![];
                $(
                    #[allow(unused_comparisons, reason = "macro-generated code")]
                    if stack >= $params $($(
                        && {
                            let limit_fn = $limit as fn(&TableOpsLimits) -> $ty;
                            let limit = (limit_fn)(&ops.limits);
                            limit > 0
                        }
                    )*)? {
                        valid_choices.push($op);
                    }
                )*

                let f = *ctx.rng()
                    .choose(&valid_choices)
                    .expect("should always have a valid op choice");

                (f)(ctx, &ops.limits, stack)
            }
        }
    };
}

define_table_ops! {
    Gc : 0 => 3,

    MakeRefs : 0 => 3,
    TakeRefs : 3 => 0,

    // Add one to make sure that out of bounds table accesses are possible, but still rare.
    TableGet(elem_index: |ops| ops.table_size + 1 => i32) : 0 => 1,
    TableSet(elem_index: |ops| ops.table_size + 1 => i32) : 1 => 0,

    GlobalGet(global_index: |ops| ops.num_globals => u32) : 0 => 1,
    GlobalSet(global_index: |ops| ops.num_globals => u32) : 1 => 0,

    LocalGet(local_index: |ops| ops.num_params => u32) : 0 => 1,
    LocalSet(local_index: |ops| ops.num_params => u32) : 1 => 0,

    StructNew(k: |ops| ops.total_struct_types() => u32) : 0 => 0,

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
            Self::StructNew(k) => {
                let ty = BASE_STRUCT_TYPE_INDEX + k;
                func.instruction(&Instruction::StructNew(ty));
                func.instruction(&Instruction::Drop);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates empty TableOps
    fn empty_test_ops(num_params: u32, num_globals: u32, table_size: i32) -> TableOps {
        let mut t = TableOps {
            limits: TableOpsLimits {
                num_params,
                num_globals,
                table_size,
                num_rec_groups: 3,
                empty_structs_per_group: 4,
            },
            ops: vec![],
            struct_types: StructTypes::new(),
        };
        for _ in 0..t.limits.num_rec_groups {
            t.struct_types.alloc_empty_rec_group();
        }
        t
    }

    /// Creates TableOps with all default opcodes
    fn test_ops(num_params: u32, num_globals: u32, table_size: i32) -> TableOps {
        let mut t = TableOps {
            limits: TableOpsLimits {
                num_params,
                num_globals,
                table_size,
                num_rec_groups: 3,
                empty_structs_per_group: 4,
            },
            ops: vec![
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
                TableOp::StructNew(0),
            ],
            struct_types: StructTypes::new(),
        };
        for _ in 0..t.limits.num_rec_groups {
            t.struct_types.alloc_empty_rec_group();
        }
        t
    }

    #[test]
    fn mutate_table_ops_with_default_mutator() -> mutatis::Result<()> {
        let _ = env_logger::try_init();
        let mut res = test_ops(5, 5, 5);

        let mut session = mutatis::Session::new();

        for _ in 0..1024 {
            session.mutate(&mut res)?;
            let wasm = res.to_wasm_binary();

            let feats = wasmparser::WasmFeatures::default();
            feats.reference_types();
            feats.gc();
            let mut validator = wasmparser::Validator::new_with_features(feats);

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

        let wasm = table_ops.to_wasm_binary();
        let wat = wasmprinter::print_bytes(&wasm).expect("Failed to convert to WAT");
        let expected = r#"
            (module
            (type (;0;) (func (result externref externref externref)))
            (type (;1;) (func (param externref externref)))
            (type (;2;) (func (param externref externref externref)))
            (type (;3;) (func (result externref externref externref)))
            (rec
                (type (;4;) (struct))
                (type (;5;) (struct))
                (type (;6;) (struct))
                (type (;7;) (struct))
            )
            (rec
                (type (;8;) (struct))
                (type (;9;) (struct))
                (type (;10;) (struct))
                (type (;11;) (struct))
            )
            (rec
                (type (;12;) (struct))
                (type (;13;) (struct))
                (type (;14;) (struct))
                (type (;15;) (struct))
            )
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
                struct.new 4
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

    #[test]
    fn emits_empty_rec_groups_and_validates() -> mutatis::Result<()> {
        let _ = env_logger::try_init();

        let mut ops = TableOps {
            limits: TableOpsLimits {
                num_params: 2,
                num_globals: 1,
                table_size: 5,
                num_rec_groups: 2,
                empty_structs_per_group: 3,
            },
            ops: vec![TableOp::Null(), TableOp::Drop(), TableOp::StructNew(0)],
            struct_types: StructTypes::new(),
        };

        for _ in 0..ops.limits.num_rec_groups {
            ops.struct_types.alloc_empty_rec_group();
        }

        let wasm = ops.to_wasm_binary();

        let feats = wasmparser::WasmFeatures::default();
        feats.reference_types();
        feats.gc();
        let mut validator = wasmparser::Validator::new_with_features(feats);
        assert!(
            validator.validate_all(&wasm).is_ok(),
            "GC validation failed"
        );

        let wat = wasmprinter::print_bytes(&wasm).expect("to WAT");
        let recs = wat.matches("(rec").count();
        let structs = wat.matches("(struct)").count();

        assert_eq!(recs, 2, "expected 2 (rec) blocks, got {recs}");
        assert_eq!(
            structs,
            2 * 3,
            "expected 6 empty struct types, got {structs}"
        );

        Ok(())
    }
}
