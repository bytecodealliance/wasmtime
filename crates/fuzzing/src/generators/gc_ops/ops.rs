//! Operations for the `gc` operations.

use crate::generators::gc_ops::types::StackType;
use crate::generators::gc_ops::{
    limits::GcOpsLimits,
    types::{CompositeType, RecGroupId, StructType, TypeId, Types},
};
use mutatis::{Context, Generate, mutators as m};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use wasm_encoder::{
    CodeSection, ConstExpr, EntityType, ExportKind, ExportSection, Function, FunctionSection,
    GlobalSection, ImportSection, Instruction, Module, RefType, TableSection, TableType,
    TypeSection, ValType,
};

/// A description of a Wasm module that makes a series of `externref` table
/// operations.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GcOps {
    pub(crate) limits: GcOpsLimits,
    pub(crate) ops: Vec<GcOp>,
    pub(crate) types: Types,
}

impl GcOps {
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

        // 4: `take_struct`
        types.ty().function(
            vec![ValType::Ref(RefType {
                nullable: false,
                heap_type: wasm_encoder::HeapType::ANY,
            })],
            vec![],
        );

        let struct_type_base: u32 = types.len();

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
                        describes: None,
                        descriptor: None,
                    },
                },
            }
        };

        let mut struct_count = 0;

        for type_ids in rec_groups.values() {
            let members: Vec<wasm_encoder::SubType> = type_ids.iter().map(encode_ty_id).collect();
            types.ty().rec(members);
            struct_count += type_ids.len() as u32;
        }

        let typed_fn_type_base: u32 = struct_type_base + struct_count;

        for i in 0..struct_count {
            let concrete = struct_type_base + i;
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

        // For each of our concrete struct types, define a function
        // import that takes an argument of that concrete type.
        let typed_first_func_index: u32 = imports.len();

        for i in 0..struct_count {
            let ty_idx = typed_fn_type_base + i;
            let name = format!("take_struct_{}", struct_type_base + i);
            imports.import("", &name, EntityType::Function(ty_idx));
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
        let mut exports = ExportSection::new();

        let run_defined_idx = functions.len();
        functions.function(1);
        let run_func_index = imports.len() + run_defined_idx;
        exports.export("run", ExportKind::Func, run_func_index);

        // Give ourselves one scratch local that we can use in various `GcOp`
        // implementations.
        let local_decls: Vec<(u32, ValType)> = vec![(1, ValType::EXTERNREF)];

        let mut func = Function::new(local_decls);
        func.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        for op in &self.ops {
            op.insert(
                &mut func,
                self.limits.num_params,
                struct_type_base,
                typed_first_func_index,
            );
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
    pub fn fixup(&mut self) {
        self.limits.fixup();
        self.types.fixup(&self.limits);

        let mut new_ops = Vec::with_capacity(self.ops.len());
        let mut stack: Vec<StackType> = Vec::new();
        let num_types = self.types.type_defs.len() as u32;

        for mut op in self.ops.iter().copied() {
            if self.limits.max_types == 0
                && matches!(
                    op,
                    GcOp::StructNew(..) | GcOp::TakeStructCall(..) | GcOp::TakeTypedStructCall(..)
                )
            {
                continue;
            }
            if self.limits.num_params == 0 && matches!(op, GcOp::LocalGet(..) | GcOp::LocalSet(..))
            {
                continue;
            }
            if self.limits.num_globals == 0
                && matches!(op, GcOp::GlobalGet(..) | GcOp::GlobalSet(..))
            {
                continue;
            }

            op.fixup(&self.limits, num_types);

            if num_types == 0
                && matches!(
                    op,
                    GcOp::StructNew(..) | GcOp::TakeStructCall(..) | GcOp::TakeTypedStructCall(..)
                )
            {
                continue;
            }

            let mut operand_types = Vec::new();
            op.operand_types(&mut operand_types);
            for ty in operand_types {
                StackType::fixup_stack_type(ty, &mut stack, &mut new_ops, num_types);
            }

            // Finally, emit the op itself (updates stack abstractly)
            let mut result_types = Vec::new();
            StackType::emit(op, &mut stack, &mut new_ops, num_types, &mut result_types);
        }

        // Balance any leftovers with drops (works for any type)
        for _ in 0..stack.len() {
            new_ops.push(GcOp::Drop());
        }
        self.ops = new_ops;
    }

    /// Attempts to remove the last opcode from the sequence.
    ///
    /// Returns `true` if an opcode was successfully removed, or `false` if the
    /// list was already empty.
    pub fn pop(&mut self) -> bool {
        self.ops.pop().is_some()
    }
}

macro_rules! define_gc_ops {
    (
        $(
            $op:ident
            $( ( $($limit_var:ident : $limit:expr => $ty:ty),* ) )?
            : ($operand_req:expr, $params:expr) => ($result_type:expr, $results:expr) ,
        )*
    ) => {

        /// The operations that can be performed by the `gc` function.
        #[derive(Copy, Clone, Debug, Serialize, Deserialize)]
        pub enum GcOp {
            $(
                #[allow(missing_docs, reason = "macro-generated code")]
                $op ( $( $($ty),* )? ),
            )*
        }

        #[cfg(test)]
        pub(crate) const OP_NAMES: &'static [&'static str] = &[
            $( stringify!($op), )*
        ];

        impl GcOp {
            #[cfg(test)]
            pub(crate) fn name(&self) -> &'static str {
                match self { $( Self::$op(..) => stringify!($op), )* }
            }

            pub(crate) fn operands_len(&self) -> usize {
                match self { $( Self::$op(..) => $params, )* }
            }

            #[allow(unreachable_patterns, reason = "macro-generated code")]
            pub(crate) fn operand_types(&self, out: &mut Vec<StackType>) {
                match self {
                    // special-cases
                    Self::TakeTypedStructCall(t) => {
                        out.push(StackType::Struct(Some(*t)));
                    }
                    $(
                        Self::$op(..) => {
                            if let Some(req) = $operand_req {
                               for _ in 0..$params {
                                out.push(req);
                               }
                            }
                        }
                    ),*
                }
            }

            pub(crate) fn results_len(&self) -> usize {
                match self { $( Self::$op(..) => $results, )* }
            }

            #[allow(unreachable_patterns, reason = "macro-generated code")]
            pub(crate) fn result_types(&self, out: &mut Vec<StackType>) {
                match self {
                    // special-cases
                    Self::StructNew(t)        => {
                        out.push(StackType::Struct(Some(*t)));
                    }
                    $(
                        Self::$op(..) => {
                            if let Some(req) = $result_type {
                               for _ in 0..$results {
                                out.push(req);
                               }
                            }
                        }
                    ),*
                }
            }

            pub(crate) fn fixup(&mut self, limits: &GcOpsLimits, num_types: u32) {
                match self {
                    $(
                        Self::$op( $( $( $limit_var ),* )? ) => {
                            $( $(
                                let limit_fn = $limit as fn(&GcOpsLimits) -> $ty;
                                let limit = (limit_fn)(limits);
                                debug_assert!(limit > 0);
                                *$limit_var = *$limit_var % limit;
                            )* )?
                        }
                    )*
                }
                match self {
                    Self::StructNew(t)
                    | Self::TakeStructCall(t)
                    | Self::TakeTypedStructCall(t) => {
                        if num_types > 0 {
                            *t %= num_types;
                        }
                    }
                    _ => {}
                }
            }

            pub(crate) fn generate(
                ctx: &mut mutatis::Context,
                ops: &GcOps,
                stack: usize,
            ) -> mutatis::Result<(GcOp, usize)> {
                let mut valid_choices: Vec<
                    fn(&mut Context, &GcOpsLimits, usize) -> mutatis::Result<(GcOp, usize)>
                > = vec![];

                $(
                    #[allow(unused_comparisons, reason = "macro-generated code")]
                    if stack >= $params $($(
                        && {
                            let limit_fn = $limit as fn(&GcOpsLimits) -> $ty;
                            (limit_fn)(&ops.limits) > 0
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

        $(
            #[allow(non_snake_case, reason = "macro-generated code")]
            fn $op(
                _ctx: &mut mutatis::Context,
                _limits: &GcOpsLimits,
                stack: usize,
            ) -> mutatis::Result<(GcOp, usize)> {
                #[allow(unused_comparisons, reason = "macro-generated code")]
                { debug_assert!(stack >= $params); }

                let op = GcOp::$op(
                    $($({
                        let limit_fn = $limit as fn(&GcOpsLimits) -> $ty;
                        let limit = (limit_fn)(_limits);
                        debug_assert!(limit > 0);
                        m::range(0..=limit - 1).generate(_ctx)?
                    }),*)?
                );
                Ok((op, stack - $params + $results))
            }
        )*
    };
}

define_gc_ops! {
    Gc : (None, 0) => (Some(StackType::ExternRef), 3),

    MakeRefs : (None, 0) => (Some(StackType::ExternRef), 3),
    TakeRefs : (Some(StackType::ExternRef), 3) => (None, 0),

    // Add one to make sure that out of bounds table accesses are possible, but still rare.
    TableGet(elem_index: |ops| ops.table_size + 1 => u32)
        : (None, 0) => (Some(StackType::ExternRef), 1),
    TableSet(elem_index: |ops| ops.table_size + 1 => u32)
        : (Some(StackType::ExternRef), 1) => (None, 0),

    GlobalGet(global_index: |ops| ops.num_globals => u32)
        : (None, 0) => (Some(StackType::ExternRef), 1),
    GlobalSet(global_index: |ops| ops.num_globals => u32)
        : (Some(StackType::ExternRef), 1) => (None, 0),

    LocalGet(local_index: |ops| ops.num_params => u32)
        : (None, 0) => (Some(StackType::ExternRef), 1),
    LocalSet(local_index: |ops| ops.num_params => u32)
        : (Some(StackType::ExternRef), 1) => (None, 0),

    // Handled specially in result_types()
    StructNew(type_index: |ops| ops.max_types => u32)
        : (None, 0) => (Some(StackType::Anything), 1),
    TakeStructCall(type_index: |ops| ops.max_types => u32)
        : (Some(StackType::Struct(None)), 1) => (None, 0),
    // Handled specially in operand_types()
    // StackType::Anythng is just a placeholder
    TakeTypedStructCall(type_index: |ops| ops.max_types => u32)
        : (Some(StackType::Anything), 1) => (None, 0),

    Drop : (Some(StackType::Anything), 1) => (None, 0),

    Null : (None, 0) => (Some(StackType::ExternRef), 1),
}

impl GcOp {
    fn insert(
        self,
        func: &mut Function,
        scratch_local: u32,
        struct_type_base: u32,
        typed_first_func_index: u32,
    ) {
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
                func.instruction(&Instruction::StructNew(x + struct_type_base));
            }
            Self::TakeStructCall(_x) => {
                func.instruction(&Instruction::Call(take_structref_idx));
            }
            Self::TakeTypedStructCall(x) => {
                let f = typed_first_func_index + x;
                func.instruction(&Instruction::Call(f));
            }
        }
    }
}
