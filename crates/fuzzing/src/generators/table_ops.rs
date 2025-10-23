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

use std::collections::{BTreeMap, BTreeSet};

const NUM_PARAMS_RANGE: RangeInclusive<u32> = 0..=10;
const MAX_TYPES_RANGE: RangeInclusive<u32> = 0..=32;
const NUM_GLOBALS_RANGE: RangeInclusive<u32> = 0..=10;
const TABLE_SIZE_RANGE: RangeInclusive<u32> = 0..=100;
const MAX_REC_GROUPS_RANGE: RangeInclusive<u32> = 0..=10;
const MAX_OPS: usize = 100;

const STRUCT_BASE: u32 = 5;
const TYPED_FN_BASE: u32 = 4;

/// RecGroup ID struct definition.
#[derive(
    Debug, Copy, Clone, Eq, PartialOrd, PartialEq, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct RecGroupId(u32);

/// TypeID struct definition.
#[derive(Debug, Clone, Eq, PartialOrd, PartialEq, Ord, Hash, Default, Serialize, Deserialize)]
pub struct TypeId(u32);

/// StructType definition
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StructType {
    // Empty for now; fields will come in a future PR.
}

/// CompsiteType definition
#[derive(Debug, Serialize, Deserialize)]
pub enum CompositeType {
    /// Struct Type definition
    Struct(StructType),
}

/// SubType definition
#[derive(Debug, Serialize, Deserialize)]
pub struct SubType {
    pub(crate) rec_group: RecGroupId,
    pub(crate) composite_type: CompositeType,
}
/// Struct types definition.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Types {
    rec_groups: BTreeSet<RecGroupId>,
    type_defs: BTreeMap<TypeId, SubType>,
}

impl Types {
    /// Create a fresh `Types` allocator with no recursive groups defined yet.
    pub fn new() -> Self {
        Self {
            rec_groups: Default::default(),
            type_defs: Default::default(),
        }
    }

    /// Insert a rec-group id. Returns true if newly inserted, false if it already existed.
    pub fn insert_rec_group(&mut self, id: RecGroupId) -> bool {
        self.rec_groups.insert(id)
    }

    ///  Insert a rec-group id.
    pub fn insert_empty_struct(&mut self, id: TypeId, group: RecGroupId) {
        self.type_defs.insert(
            id,
            SubType {
                rec_group: group,
                composite_type: CompositeType::Struct(StructType::default()),
            },
        );
    }

    /// Removes any entries beyond the given limit.
    pub fn fixup(&mut self, limits: &TableOpsLimits) {
        while self.rec_groups.len() > limits.max_rec_groups as usize {
            self.rec_groups.pop_last();
        }
        while self.type_defs.len() > limits.max_types as usize {
            self.type_defs.pop_last();
        }

        debug_assert!(
            self.type_defs
                .values()
                .all(|ty| self.rec_groups.contains(&ty.rec_group))
        );
    }
}

/// Limits controlling the structure of a generated Wasm module.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TableOpsLimits {
    pub(crate) num_params: u32,
    pub(crate) num_globals: u32,
    pub(crate) table_size: u32,
    pub(crate) max_rec_groups: u32,
    pub(crate) max_types: u32,
}

impl TableOpsLimits {
    fn fixup(&mut self) {
        // NB: Exhaustively match so that we remember to fixup any other new
        // limits we add in the future.
        let Self {
            num_params,
            num_globals,
            table_size,
            max_rec_groups,
            max_types,
        } = self;

        let clamp = |limit: &mut u32, range: RangeInclusive<u32>| {
            *limit = (*limit).clamp(*range.start(), *range.end())
        };
        clamp(table_size, TABLE_SIZE_RANGE);
        clamp(num_params, NUM_PARAMS_RANGE);
        clamp(num_globals, NUM_GLOBALS_RANGE);
        clamp(max_rec_groups, MAX_REC_GROUPS_RANGE);
        clamp(max_types, MAX_TYPES_RANGE);
    }
}

/// A description of a Wasm module that makes a series of `externref` table
/// operations.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TableOps {
    pub(crate) limits: TableOpsLimits,
    pub(crate) ops: Vec<TableOp>,
    pub(crate) types: Types,
}

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
        self.fixup();

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

        types.ty().function(
            vec![ValType::Ref(RefType {
                nullable: false,
                heap_type: wasm_encoder::HeapType::ANY,
            })],
            vec![],
        );

        let mut rec_groups: BTreeMap<RecGroupId, Vec<TypeId>> = self
            .types
            .rec_groups
            .iter()
            .copied()
            .map(|id| (id, Vec::new()))
            .collect();

        for (id, ty) in self.types.type_defs.iter() {
            rec_groups.entry(ty.rec_group).or_default().push(id.clone());
        }

        let encode_ty_id = |ty_id: &TypeId| -> wasm_encoder::SubType {
            let def = &self.types.type_defs[ty_id];
            match &def.composite_type {
                CompositeType::Struct(StructType {}) => wasm_encoder::SubType {
                    is_final: true,
                    supertype_idx: None,
                    composite_type: wasm_encoder::CompositeType {
                        inner: wasm_encoder::CompositeInnerType::Struct(wasm_encoder::StructType {
                            fields: Box::new([]),
                        }),
                        shared: false,
                    },
                },
            }
        };

        let mut struct_count: u32 = 0;
        for type_ids in rec_groups.values() {
            let members: Vec<wasm_encoder::SubType> = type_ids.iter().map(encode_ty_id).collect();
            types.ty().rec(members);
            struct_count += type_ids.len() as u32;
        }

        let typed_ft_base: u32 = STRUCT_BASE + struct_count;
        for i in 0..struct_count {
            let concrete = STRUCT_BASE + i;
            types.ty().function(
                vec![ValType::Ref(RefType {
                    nullable: false,
                    heap_type: wasm_encoder::HeapType::Concrete(concrete),
                })],
                vec![],
            );
        }

        // Import the GC function.
        let mut imports = ImportSection::new();
        imports.import("", "gc", EntityType::Function(0));
        imports.import("", "take_refs", EntityType::Function(2));
        imports.import("", "make_refs", EntityType::Function(3));
        imports.import("", "take_struct", EntityType::Function(4));

        let mut typed_names: Vec<String> = Vec::new();

        for i in 0..struct_count {
            let concrete = STRUCT_BASE + i;
            let ty_idx = typed_ft_base + i; //
            let name = format!("take_struct_{concrete}");
            typed_names.push(name);
            imports.import(
                "",
                typed_names.last().unwrap().as_str(),
                EntityType::Function(ty_idx),
            );
        }

        // Define our table.
        let mut tables = TableSection::new();
        tables.table(TableType {
            element_type: RefType::EXTERNREF,
            minimum: u64::from(self.limits.table_size),
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
        let imported_fn_count: u32 = 4 + struct_count;
        exports.export("run", ExportKind::Func, imported_fn_count);

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

    /// Fixes this test case such that it becomes valid.
    ///
    /// This is necessary because a random mutation (e.g. removing an op in the
    /// middle of our sequence) might have made it so that subsequent ops won't
    /// have their expected operand types on the Wasm stack
    /// anymore. Furthermore, because we serialize and deserialize test cases,
    /// and libFuzzer will occasionally mutate those serialized bytes directly,
    /// rather than use one of our custom mutations, we have no guarantee that
    /// pre-mutation test cases are even valid! Therefore, we always call this
    /// method before translating this "AST"-style representation into a raw
    /// Wasm binary.
    fn fixup(&mut self) {
        self.limits.fixup();
        self.types.fixup(&self.limits);

        let mut new_ops = Vec::with_capacity(self.ops.len());
        let mut stack = 0;

        for mut op in self.ops.iter().copied() {
            if self.limits.max_types == 0 && matches!(op, TableOp::StructNew(..)) {
                continue;
            }
            if self.limits.num_params == 0
                && matches!(op, TableOp::LocalGet(..) | TableOp::LocalSet(..))
            {
                continue;
            }
            if self.limits.num_globals == 0
                && matches!(op, TableOp::GlobalGet(..) | TableOp::GlobalSet(..))
            {
                continue;
            }

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

        let max_rec_groups = m::range(MAX_REC_GROUPS_RANGE).generate(ctx)?;
        let max_types = m::range(MAX_TYPES_RANGE).generate(ctx)?;

        let mut ops = TableOps {
            limits: TableOpsLimits {
                num_params,
                num_globals,
                table_size,
                max_rec_groups,
                max_types,
            },
            ops: {
                let mut v = vec![TableOp::Null(), TableOp::Drop(), TableOp::Gc()];
                if num_params > 0 {
                    v.push(TableOp::LocalSet(0));
                    v.push(TableOp::LocalGet(0));
                }
                if num_globals > 0 {
                    v.push(TableOp::GlobalSet(0));
                    v.push(TableOp::GlobalGet(0));
                }
                if max_types > 0 {
                    v.push(TableOp::StructNew(0));
                }
                v
            },
            types: Types::new(),
        };

        for i in 0..ops.limits.max_rec_groups {
            ops.types.insert_rec_group(RecGroupId(i));
        }

        if ops.limits.max_rec_groups > 0 {
            for i in 0..ops.limits.max_types {
                let tid = TypeId(i);
                let gid = RecGroupId(m::range(0..=ops.limits.max_rec_groups - 1).generate(ctx)?);

                ops.types.insert_empty_struct(tid, gid);
            }
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
    TableGet(elem_index: |ops| ops.table_size + 1 => u32) : 0 => 1,
    TableSet(elem_index: |ops| ops.table_size + 1 => u32) : 1 => 0,

    GlobalGet(global_index: |ops| ops.num_globals => u32) : 0 => 1,
    GlobalSet(global_index: |ops| ops.num_globals => u32) : 1 => 0,

    LocalGet(local_index: |ops| ops.num_params => u32) : 0 => 1,
    LocalSet(local_index: |ops| ops.num_params => u32) : 1 => 0,

    StructNew(type_index: |ops| ops.max_types => u32) : 0 => 0,
    TakeStructCall(type_index: |ops| ops.max_types => u32) : 0 => 0,
    TakeTypedStructCall(type_index: |ops| ops.max_types => u32) : 0 => 0,

    Drop : 1 => 0,

    Null : 0 => 1,
}

impl TableOp {
    fn insert(self, func: &mut Function, scratch_local: u32) {
        let gc_func_idx = 0;
        let take_refs_func_idx = 1;
        let make_refs_func_idx = 2;
        let take_structref_idx = 3;

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
                func.instruction(&Instruction::I32Const(x.cast_signed()));
                func.instruction(&Instruction::TableGet(0));
            }
            Self::TableSet(x) => {
                func.instruction(&Instruction::LocalSet(scratch_local));
                func.instruction(&Instruction::I32Const(x.cast_signed()));
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
            Self::StructNew(x) => {
                func.instruction(&Instruction::StructNew(x + 5));
                func.instruction(&Instruction::Call(take_structref_idx));
            }
            Self::TakeStructCall(x) => {
                func.instruction(&Instruction::StructNew(x + 5));
                func.instruction(&Instruction::Call(take_structref_idx));
            }
            Self::TakeTypedStructCall(x) => {
                let s = STRUCT_BASE + x;
                let f = TYPED_FN_BASE + x;
                func.instruction(&Instruction::StructNew(s));
                func.instruction(&Instruction::Call(f));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    /// Creates empty TableOps
    fn empty_test_ops() -> TableOps {
        let mut t = TableOps {
            limits: TableOpsLimits {
                num_params: 5,
                num_globals: 5,
                table_size: 5,
                max_rec_groups: 5,
                max_types: 5,
            },
            ops: vec![],
            types: Types::new(),
        };
        for i in 0..t.limits.max_rec_groups {
            t.types.insert_rec_group(RecGroupId(i));
        }
        t
    }

    /// Creates TableOps with all default opcodes
    fn test_ops(num_params: u32, num_globals: u32, table_size: u32) -> TableOps {
        let mut t = TableOps {
            limits: TableOpsLimits {
                num_params,
                num_globals,
                table_size,
                max_rec_groups: 7,
                max_types: 10,
            },
            ops: vec![
                TableOp::Null(),
                TableOp::Drop(),
                TableOp::Gc(),
                TableOp::LocalSet(0),
                TableOp::LocalGet(0),
                TableOp::GlobalSet(0),
                TableOp::GlobalGet(0),
                TableOp::StructNew(0),
            ],
            types: Types::new(),
        };
        for i in 0..t.limits.max_rec_groups {
            t.types.insert_rec_group(RecGroupId(i));
        }

        if t.limits.max_rec_groups > 0 {
            let mut rng = StdRng::seed_from_u64(0xC0FFEE);
            for i in 0..t.limits.max_types {
                let gid = RecGroupId(rng.gen_range(0..t.limits.max_rec_groups));
                t.types.insert_empty_struct(TypeId(i), gid);
            }
        }
        t
    }

    #[test]
    fn mutate_table_ops_with_default_mutator() -> mutatis::Result<()> {
        let _ = env_logger::try_init();
        let mut res = test_ops(5, 5, 5);

        let mut session = mutatis::Session::new();

        for _ in 0..2048 {
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
    fn struct_new_removed_when_no_types() -> mutatis::Result<()> {
        let _ = env_logger::try_init();

        let mut ops = test_ops(0, 0, 0);
        ops.limits.max_types = 0;
        ops.ops = vec![TableOp::StructNew(42)];

        let _ = ops.fixup();

        assert!(
            ops.ops
                .iter()
                .all(|op| !matches!(op, TableOp::StructNew(..))),
            "StructNew should be removed when there are no types"
        );
        Ok(())
    }

    #[test]
    fn local_ops_removed_when_no_params() -> mutatis::Result<()> {
        let _ = env_logger::try_init();

        let mut ops = test_ops(0, 0, 0);
        ops.limits.num_params = 0;
        ops.ops = vec![TableOp::LocalGet(42), TableOp::LocalSet(99)];

        ops.fixup();

        assert!(
            ops.ops
                .iter()
                .all(|op| !matches!(op, TableOp::LocalGet(..) | TableOp::LocalSet(..))),
            "LocalGet/LocalSet should be removed when there are no params"
        );
        Ok(())
    }

    #[test]
    fn global_ops_removed_when_no_globals() -> mutatis::Result<()> {
        let _ = env_logger::try_init();

        let mut ops = test_ops(0, 0, 0);
        ops.limits.num_globals = 0;
        ops.ops = vec![TableOp::GlobalGet(42), TableOp::GlobalSet(99)];

        ops.fixup();

        assert!(
            ops.ops
                .iter()
                .all(|op| !matches!(op, TableOp::GlobalGet(..) | TableOp::GlobalSet(..))),
            "GlobalGet/GlobalSet should be removed when there are no globals"
        );
        Ok(())
    }

    #[test]
    fn every_op_generated() -> mutatis::Result<()> {
        let _ = env_logger::try_init();
        let mut unseen_ops: std::collections::HashSet<_> = OP_NAMES.iter().copied().collect();

        let mut res = empty_test_ops();
        let mut session = mutatis::Session::new();

        'outer: for _ in 0..=1024 {
            session.mutate(&mut res)?;
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

        let actual_wat = wasmprinter::print_bytes(&wasm).expect("Failed to convert to WAT");
        let actual_wat = actual_wat.trim();

        let expected_wat = r#"
(module
  (type (;0;) (func (result externref externref externref)))
  (type (;1;) (func (param externref externref)))
  (type (;2;) (func (param externref externref externref)))
  (type (;3;) (func (result externref externref externref)))
  (type (;4;) (func (param (ref any))))
  (rec
    (type (;5;) (struct))
  )
  (rec)
  (rec
    (type (;6;) (struct))
  )
  (rec
    (type (;7;) (struct))
    (type (;8;) (struct))
    (type (;9;) (struct))
  )
  (rec
    (type (;10;) (struct))
    (type (;11;) (struct))
  )
  (rec
    (type (;12;) (struct))
    (type (;13;) (struct))
  )
  (rec
    (type (;14;) (struct))
  )
  (type (;15;) (func (param (ref 5))))
  (type (;16;) (func (param (ref 6))))
  (type (;17;) (func (param (ref 7))))
  (type (;18;) (func (param (ref 8))))
  (type (;19;) (func (param (ref 9))))
  (type (;20;) (func (param (ref 10))))
  (type (;21;) (func (param (ref 11))))
  (type (;22;) (func (param (ref 12))))
  (type (;23;) (func (param (ref 13))))
  (type (;24;) (func (param (ref 14))))
  (import "" "gc" (func (;0;) (type 0)))
  (import "" "take_refs" (func (;1;) (type 2)))
  (import "" "make_refs" (func (;2;) (type 3)))
  (import "" "take_struct" (func (;3;) (type 4)))
  (import "" "take_struct_5" (func (;4;) (type 15)))
  (import "" "take_struct_6" (func (;5;) (type 16)))
  (import "" "take_struct_7" (func (;6;) (type 17)))
  (import "" "take_struct_8" (func (;7;) (type 18)))
  (import "" "take_struct_9" (func (;8;) (type 19)))
  (import "" "take_struct_10" (func (;9;) (type 20)))
  (import "" "take_struct_11" (func (;10;) (type 21)))
  (import "" "take_struct_12" (func (;11;) (type 22)))
  (import "" "take_struct_13" (func (;12;) (type 23)))
  (import "" "take_struct_14" (func (;13;) (type 24)))
  (table (;0;) 5 externref)
  (global (;0;) (mut externref) ref.null extern)
  (global (;1;) (mut externref) ref.null extern)
  (export "run" (func 14))
  (func (;14;) (type 1) (param externref externref)
    (local externref)
    loop ;; label = @1
      ref.null extern
      drop
      call 0
      local.set 0
      local.get 0
      global.set 0
      global.get 0
      struct.new 5
      call 3
      drop
      drop
      drop
      br 0 (;@1;)
    end
  )
)
        "#;
        let expected_wat = expected_wat.trim();

        eprintln!("=== actual ===\n{actual_wat}");
        eprintln!("=== expected ===\n{expected_wat}");
        assert_eq!(
            actual_wat, expected_wat,
            "actual WAT does not match expected"
        );

        Ok(())
    }

    #[test]
    fn emits_empty_rec_groups_and_validates() -> mutatis::Result<()> {
        let _ = env_logger::try_init();

        let mut ops = test_ops(5, 5, 5);

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

        assert_eq!(recs, 7, "expected 2 (rec) blocks, got {recs}");
        assert_eq!(structs, 10, "expected no struct types, got {structs}");

        Ok(())
    }
}
