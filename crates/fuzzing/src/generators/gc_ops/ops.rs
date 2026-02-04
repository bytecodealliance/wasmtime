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

/// The base offsets and indices for various Wasm entities within
// their index spaces in the the encoded Wasm binary.
#[derive(Clone, Copy)]
struct WasmEncodingBases {
    struct_type_base: u32,
    typed_first_func_index: u32,
    struct_local_idx: u32,
    typed_local_base: u32,
    struct_global_idx: u32,
    typed_global_base: u32,
    struct_table_idx: u32,
    typed_table_base: u32,
}

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
        let params_len =
            u32::try_from(params.len()).expect("params len should be within u32 range");
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
                nullable: true,
                heap_type: wasm_encoder::HeapType::Abstract {
                    shared: false,
                    ty: wasm_encoder::AbstractHeapType::Struct,
                },
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
                    nullable: true,
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

        let struct_table_idx = tables.len();
        tables.table(TableType {
            element_type: RefType {
                nullable: true,
                heap_type: wasm_encoder::HeapType::Abstract {
                    shared: false,
                    ty: wasm_encoder::AbstractHeapType::Struct,
                },
            },
            minimum: u64::from(self.limits.table_size),
            maximum: None,
            table64: false,
            shared: false,
        });

        let typed_table_base = tables.len();
        for i in 0..struct_count {
            let concrete = struct_type_base + i;
            tables.table(TableType {
                element_type: RefType {
                    nullable: true,
                    heap_type: wasm_encoder::HeapType::Concrete(concrete),
                },
                minimum: u64::from(self.limits.table_size),
                maximum: None,
                table64: false,
                shared: false,
            });
        }

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

        // Add exactly one (ref.null struct) global.
        let struct_global_idx = globals.len();
        globals.global(
            wasm_encoder::GlobalType {
                val_type: ValType::Ref(RefType {
                    nullable: true,
                    heap_type: wasm_encoder::HeapType::Abstract {
                        shared: false,
                        ty: wasm_encoder::AbstractHeapType::Struct,
                    },
                }),
                mutable: true,
                shared: false,
            },
            &ConstExpr::ref_null(wasm_encoder::HeapType::Abstract {
                shared: false,
                ty: wasm_encoder::AbstractHeapType::Struct,
            }),
        );

        // Add one typed (ref <type>) global per struct type.
        let typed_global_base = globals.len();
        for i in 0..struct_count {
            let concrete = struct_type_base + i;
            globals.global(
                wasm_encoder::GlobalType {
                    val_type: ValType::Ref(RefType {
                        nullable: true,
                        heap_type: wasm_encoder::HeapType::Concrete(concrete),
                    }),
                    mutable: true,
                    shared: false,
                },
                &ConstExpr::ref_null(wasm_encoder::HeapType::Concrete(concrete)),
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
        let mut local_decls: Vec<(u32, ValType)> = vec![(1, ValType::EXTERNREF)];

        let scratch_local = params_len;
        let struct_local_idx = scratch_local + 1;
        local_decls.push((
            1,
            ValType::Ref(RefType {
                nullable: true,
                heap_type: wasm_encoder::HeapType::Abstract {
                    shared: false,
                    ty: wasm_encoder::AbstractHeapType::Struct,
                },
            }),
        ));

        let typed_local_base: u32 = struct_local_idx + 1;
        for i in 0..struct_count {
            let concrete = struct_type_base + i;
            local_decls.push((
                1,
                ValType::Ref(RefType {
                    nullable: true,
                    heap_type: wasm_encoder::HeapType::Concrete(concrete),
                }),
            ));
        }

        let storage_bases = WasmEncodingBases {
            struct_type_base,
            typed_first_func_index,
            struct_local_idx,
            typed_local_base,
            struct_global_idx,
            typed_global_base,
            struct_table_idx,
            typed_table_base,
        };

        let mut func = Function::new(local_decls);
        func.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        for op in &self.ops {
            op.insert(&mut func, scratch_local, storage_bases);
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
        let num_types = u32::try_from(self.types.type_defs.len())
            .expect("types len should be within u32 range");

        let mut operand_types = Vec::new();

        for mut op in self.ops.iter().copied() {
            if self.limits.max_types == 0
                && matches!(
                    op,
                    GcOp::StructNew(..)
                        | GcOp::TakeStructCall(..)
                        | GcOp::TakeTypedStructCall(..)
                        | GcOp::TypedStructLocalSet(..)
                        | GcOp::TypedStructLocalGet(..)
                        | GcOp::TypedStructGlobalSet(..)
                        | GcOp::TypedStructGlobalGet(..)
                        | GcOp::TypedStructTableSet(..)
                        | GcOp::TypedStructTableGet(..)
                        | GcOp::StructTableSet(..)
                        | GcOp::StructTableGet(..)
                        | GcOp::StructLocalSet(..)
                        | GcOp::StructLocalGet(..)
                        | GcOp::StructGlobalSet(..)
                        | GcOp::StructGlobalGet(..)
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

            let op = if let Some(op) = op.fixup(&self.limits, num_types) {
                op
            } else {
                continue;
            };

            operand_types.clear();
            op.operand_types(&mut operand_types);
            for ty in &operand_types {
                StackType::fixup(*ty, &mut stack, &mut new_ops, num_types);
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
            : [ $($operand:expr),* $(,)? ] => [ $($result:expr),* $(,)? ] ,
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

            #[allow(unreachable_patterns, reason = "macro-generated code")]
            pub(crate) fn operand_types(&self, out: &mut Vec<Option<StackType>>) {

                match self {
                    Self::TakeTypedStructCall(t) => {
                        out.push(Some(StackType::Struct(Some(*t))));
                    }
                    Self::TypedStructLocalSet(t) => {
                        out.push(Some(StackType::Struct(Some(*t))));
                    }
                    Self::TypedStructGlobalSet(t) => {
                        out.push(Some(StackType::Struct(Some(*t))));
                    }
                    Self::TypedStructTableSet(_, t) => {
                        out.push(Some(StackType::Struct(Some(*t))));
                    }
                    $(
                        Self::$op(..) => {
                            $( out.push($operand); )*
                        }
                    ),*
                }
            }

            #[allow(unreachable_patterns, reason = "macro-generated code")]
            pub(crate) fn result_types(&self, out: &mut Vec<StackType>) {
                match self {
                    Self::StructNew(t)        => {
                        out.push(StackType::Struct(Some(*t)));
                    }
                    Self::TypedStructLocalGet(t) => {
                        out.push(StackType::Struct(Some(*t)));
                    }
                    Self::TypedStructGlobalGet(t) => {
                        out.push(StackType::Struct(Some(*t)));
                    }
                    Self::TypedStructTableGet(_, t) => {
                        out.push(StackType::Struct(Some(*t)));
                    }
                    $( Self::$op(..) => { $( out.push($result); )* }, )*
                }
            }

            pub(crate) fn fixup(&mut self, limits: &GcOpsLimits, num_types: u32) -> Option<Self> {
                match self {
                    $(
                        Self::$op( $( $( $limit_var ),* )? ) => {
                            $( $(
                                let limit_fn = $limit as fn(&GcOpsLimits) -> $ty;
                                let limit = (limit_fn)(limits);
                                if limit == 0 {
                                    return None;
                                }
                                *$limit_var = *$limit_var % limit;
                            )* )?
                        }
                    )*
                }
                match self {
                    Self::StructNew(t)
                    | Self::TakeStructCall(t)
                    | Self::TakeTypedStructCall(t)
                    | Self::TypedStructLocalSet(t)
                    | Self::TypedStructLocalGet(t)
                    | Self::TypedStructGlobalSet(t)
                    | Self::TypedStructGlobalGet(t)
                    | Self::TypedStructTableSet(_, t)
                    | Self::TypedStructTableGet(_, t)

                    => {
                        if num_types == 0 {
                            return None;
                        }
                        *t %= num_types;
                    }
                    _ => {}
                }

                Some(*self)
            }

            /// Generate an arbitrary op without stack-depth awareness.
            /// The fixup pass will make the sequence valid.
            pub(crate) fn generate(
                ctx: &mut mutatis::Context,
                ops: &GcOps,
            ) -> mutatis::Result<GcOp> {
                let mut valid_choices: Vec<
                    fn(&mut Context, &GcOpsLimits) -> mutatis::Result<GcOp>
                > = vec![];

                $(
                    valid_choices.push($op);
                )*

                let f = *ctx.rng()
                    .choose(&valid_choices)
                    .expect("should always have a valid op choice");
                (f)(ctx, &ops.limits)
            }
        }

        $(
            #[allow(non_snake_case, reason = "macro-generated code")]
            fn $op(
                _ctx: &mut mutatis::Context,
                _limits: &GcOpsLimits,
            ) -> mutatis::Result<GcOp> {
                let op = GcOp::$op(
                    $($({
                        let limit_fn = $limit as fn(&GcOpsLimits) -> $ty;
                        let limit = (limit_fn)(_limits);
                        // Generate a value even if limit is 0; fixup will handle it
                        if limit > 0 {
                            m::range(0..=limit - 1).generate(_ctx)?
                        } else {
                            0
                        }
                    }),*)?
                );
                Ok(op)
            }
        )*
    };
}

define_gc_ops! {
    Gc : [] => [StackType::ExternRef, StackType::ExternRef, StackType::ExternRef],

    MakeRefs : [] => [StackType::ExternRef, StackType::ExternRef, StackType::ExternRef],
    TakeRefs : [Some(StackType::ExternRef), Some(StackType::ExternRef), Some(StackType::ExternRef)] => [],

    // Add one to make sure that out of bounds table accesses are possible, but still rare.
    TableGet(elem_index: |ops| ops.table_size + 1 => u32)
        : [] => [StackType::ExternRef],
    TableSet(elem_index: |ops| ops.table_size + 1 => u32)
        : [Some(StackType::ExternRef)] => [],

    GlobalGet(global_index: |ops| ops.num_globals => u32)
        : [] => [StackType::ExternRef],
    GlobalSet(global_index: |ops| ops.num_globals => u32)
        : [Some(StackType::ExternRef)] => [],

    LocalGet(local_index: |ops| ops.num_params => u32)
        : [] => [StackType::ExternRef],
    LocalSet(local_index: |ops| ops.num_params => u32)
        : [Some(StackType::ExternRef)] => [],

    // `StructNew` result is special-cased to push `Struct(Some(t))`, so results list is empty.
    StructNew(type_index: |ops| ops.max_types => u32)
        : [] => [],

    TakeStructCall(type_index: |ops| ops.max_types => u32)
        : [Some(StackType::Struct(None))] => [],

    // `TakeTypedStructCall` operand is special-cased to require `Struct(Some(t))`, so operands list is empty.
    TakeTypedStructCall(type_index: |ops| ops.max_types => u32)
        : [] => [],

    StructLocalSet() : [Some(StackType::Struct(None))] => [],
    StructLocalGet() : [] => [StackType::Struct(None)],
    // `TypedStructLocalSet` operand is special-cased to require `Struct(Some(t))`, so operands list is empty.
    TypedStructLocalSet(type_index: |ops| ops.max_types => u32) : [] => [],
    // `TypedStructLocalGet` result is special-cased to push `Struct(Some(t))`, so results list is empty.
    TypedStructLocalGet(type_index: |ops| ops.max_types => u32) : [] => [],

    StructGlobalSet() : [Some(StackType::Struct(None))] => [],
    StructGlobalGet() : [] => [StackType::Struct(None)],
    // `TypedStructGlobalSet` operand is special-cased to require `Struct(Some(t))`, so operands list is empty.
    TypedStructGlobalSet(type_index: |ops| ops.max_types => u32) : [] => [],
    // `TypedStructGlobalGet` result is special-cased to push `Struct(Some(t))`, so results list is empty.
    TypedStructGlobalGet(type_index: |ops| ops.max_types => u32) : [] => [],

    StructTableSet(elem_index: |ops| ops.table_size => u32) : [Some(StackType::Struct(None))] => [],
    StructTableGet(elem_index: |ops| ops.table_size => u32) : [] => [StackType::Struct(None)],
    // `TypedStructTableSet` operand is special-cased to require `Struct(Some(t))`, so operands list is empty.
    TypedStructTableSet(elem_index: |ops| ops.table_size => u32, type_index: |ops| ops.max_types => u32) : [] => [],
    // `TypedStructTableGet` result is special-cased to push `Struct(Some(t))`, so results list is empty.
    TypedStructTableGet(elem_index: |ops| ops.table_size => u32, type_index: |ops| ops.max_types => u32) : [] => [],

    Drop : [None] => [],

    Null : [] => [StackType ::ExternRef],
}

impl GcOp {
    fn insert(self, func: &mut Function, scratch_local: u32, storage_bases: WasmEncodingBases) {
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
                func.instruction(&Instruction::StructNew(x + storage_bases.struct_type_base));
            }
            Self::TakeStructCall(_x) => {
                func.instruction(&Instruction::Call(take_structref_idx));
            }
            Self::TakeTypedStructCall(x) => {
                let f = storage_bases.typed_first_func_index + x;
                func.instruction(&Instruction::Call(f));
            }
            Self::StructLocalGet() => {
                func.instruction(&Instruction::LocalGet(storage_bases.struct_local_idx));
            }
            Self::TypedStructLocalGet(x) => {
                func.instruction(&Instruction::LocalGet(storage_bases.typed_local_base + x));
            }
            Self::StructLocalSet() => {
                func.instruction(&Instruction::LocalSet(storage_bases.struct_local_idx));
            }
            Self::TypedStructLocalSet(x) => {
                func.instruction(&Instruction::LocalSet(storage_bases.typed_local_base + x));
            }
            Self::StructGlobalGet() => {
                func.instruction(&Instruction::GlobalGet(storage_bases.struct_global_idx));
            }
            Self::TypedStructGlobalGet(x) => {
                func.instruction(&Instruction::GlobalGet(storage_bases.typed_global_base + x));
            }
            Self::StructGlobalSet() => {
                func.instruction(&Instruction::GlobalSet(storage_bases.struct_global_idx));
            }
            Self::TypedStructGlobalSet(x) => {
                func.instruction(&Instruction::GlobalSet(storage_bases.typed_global_base + x));
            }
            Self::StructTableGet(elem_index) => {
                func.instruction(&Instruction::I32Const(elem_index.cast_signed()));
                func.instruction(&Instruction::TableGet(storage_bases.struct_table_idx));
            }
            Self::TypedStructTableGet(elem_index, x) => {
                func.instruction(&Instruction::I32Const(elem_index.cast_signed()));
                func.instruction(&Instruction::TableGet(storage_bases.typed_table_base + x));
            }
            Self::StructTableSet(elem_index) => {
                // Use struct_local_idx (anyref) to temporarily store the value before table.set
                func.instruction(&Instruction::LocalSet(storage_bases.struct_local_idx));
                func.instruction(&Instruction::I32Const(elem_index.cast_signed()));
                func.instruction(&Instruction::LocalGet(storage_bases.struct_local_idx));
                func.instruction(&Instruction::TableSet(storage_bases.struct_table_idx));
            }
            Self::TypedStructTableSet(elem_index, x) => {
                func.instruction(&Instruction::LocalSet(storage_bases.typed_local_base + x));
                func.instruction(&Instruction::I32Const(elem_index.cast_signed()));
                func.instruction(&Instruction::LocalGet(storage_bases.typed_local_base + x));
                func.instruction(&Instruction::TableSet(storage_bases.typed_table_base + x));
            }
        }
    }
}
