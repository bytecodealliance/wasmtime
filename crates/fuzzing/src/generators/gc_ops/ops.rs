//! Operations for the `gc` operations.

use crate::generators::gc_ops::stack::StackType;
use crate::generators::gc_ops::{
    limits::{GcOpsLimits, MAX_INLINE_CONSTRUCTION},
    types::{CompositeType, EmitCtx, FieldType, RecGroupId, StructField, TypeId, Types, emit_new},
};
use mutatis::Generate;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use wasm_encoder::{
    AbstractHeapType, BlockType, CodeSection, ConstExpr, EntityType, ExportKind, ExportSection,
    Function, FunctionSection, GlobalSection, GlobalType, HeapType, ImportSection, Instruction,
    Module, RefType, TableSection, TableType, TypeSection, ValType,
};

/// The abstract `struct` heap type; `wasm_encoder` has no shorthand for it.
const STRUCT: HeapType = HeapType::Abstract {
    shared: false,
    ty: AbstractHeapType::Struct,
};

/// The abstract `array` heap type.
const ARRAY: HeapType = HeapType::Abstract {
    shared: false,
    ty: AbstractHeapType::Array,
};

/// The abstract `eq` heap type.
const EQ: HeapType = HeapType::Abstract {
    shared: false,
    ty: AbstractHeapType::Eq,
};

/// `structref`, i.e. `(ref null struct)`.
const STRUCTREF: RefType = RefType {
    nullable: true,
    heap_type: STRUCT,
};

/// `(ref null $index)`.
fn concrete(index: u32) -> RefType {
    RefType {
        nullable: true,
        heap_type: HeapType::Concrete(index),
    }
}

/// Pick a same-kind concrete type index for `raw`, or `None` when there are no
/// types of that kind (so the caller drops the op).
fn pick_type_index(indices: &[u32], raw: u32) -> Option<u32> {
    if indices.is_empty() {
        None
    } else {
        Some(indices[usize::try_from(raw).unwrap() % indices.len()])
    }
}

/// Returns the element field if the indexed type is an array
fn array_element<'a>(
    types: &'a Types,
    encoding_order: &[TypeId],
    type_index: u32,
) -> Option<&'a StructField> {
    encoding_order
        .get(usize::try_from(type_index).unwrap())
        .and_then(|tid| types.type_defs.get(tid))
        .and_then(|def| match &def.composite_type {
            CompositeType::Array(at) => Some(&at.element),
            CompositeType::Struct(_) => None,
        })
}

/// The fields of the struct type at `type_index`, or `None` if that index is
/// out of range or names an array type.
fn struct_fields<'a>(
    types: &'a Types,
    encoding_order: &[TypeId],
    type_index: u32,
) -> Option<&'a [StructField]> {
    encoding_order
        .get(usize::try_from(type_index).unwrap())
        .and_then(|tid| types.type_defs.get(tid))
        .and_then(|def| match &def.composite_type {
            CompositeType::Struct(st) => Some(st.fields.as_slice()),
            CompositeType::Array(_) => None,
        })
}

/// A table of `size` nullable references with no maximum.
fn nullable_table(element_type: RefType, size: u32) -> TableType {
    TableType {
        element_type,
        minimum: u64::from(size),
        maximum: None,
        table64: false,
        shared: false,
    }
}

/// Append a mutable `(ref null heap_type)` global initialized to null; returns its index.
fn null_ref_global(globals: &mut GlobalSection, heap_type: HeapType) -> u32 {
    let index = globals.len();
    globals.global(
        GlobalType {
            val_type: ValType::Ref(RefType {
                nullable: true,
                heap_type,
            }),
            mutable: true,
            shared: false,
        },
        &ConstExpr::ref_null(heap_type),
    );
    index
}

/// `table.get` of a constant index.
fn table_get(func: &mut Function, elem_index: u32, table: u32) {
    func.instruction(&Instruction::I32Const(elem_index.cast_signed()));
    func.instruction(&Instruction::TableGet(table));
}

/// `table.set` of a constant index with the value on top of the stack, parked
/// in `tmp` (a local of the table's element type) while the index is pushed.
fn table_set_via(func: &mut Function, tmp: u32, elem_index: u32, table: u32) {
    func.instruction(&Instruction::LocalSet(tmp));
    func.instruction(&Instruction::I32Const(elem_index.cast_signed()));
    func.instruction(&Instruction::LocalGet(tmp));
    func.instruction(&Instruction::TableSet(table));
}

/// Pop the reference on top of the stack into `tmp` and run `body` only if it
/// is non-null; `body` reads it back with `local.get tmp`.
fn if_non_null(func: &mut Function, tmp: u32, body: impl FnOnce(&mut Function)) {
    func.instruction(&Instruction::LocalTee(tmp));
    func.instruction(&Instruction::RefIsNull);
    func.instruction(&Instruction::If(BlockType::Empty));
    func.instruction(&Instruction::Else);
    body(func);
    func.instruction(&Instruction::End);
}

/// The `struct.get` variant for a field: `_s`/`_u` if packed, plain otherwise.
fn struct_get_instruction(
    struct_type_index: u32,
    field_index: u32,
    field_type: FieldType,
    unsigned: bool,
) -> Instruction<'static> {
    match (field_type.is_packed(), unsigned) {
        (false, _) => Instruction::StructGet {
            struct_type_index,
            field_index,
        },
        (true, true) => Instruction::StructGetU {
            struct_type_index,
            field_index,
        },
        (true, false) => Instruction::StructGetS {
            struct_type_index,
            field_index,
        },
    }
}

/// The `array.get` variant for an element type: `_s`/`_u` if packed, plain otherwise.
fn array_get_instruction(
    array_type_index: u32,
    element_type: FieldType,
    unsigned: bool,
) -> Instruction<'static> {
    match (element_type.is_packed(), unsigned) {
        (false, _) => Instruction::ArrayGet(array_type_index),
        (true, true) => Instruction::ArrayGetU(array_type_index),
        (true, false) => Instruction::ArrayGetS(array_type_index),
    }
}

/// Indices of the function types of the host imports and `run`.
struct HostTypes {
    gc: u32,
    run: u32,
    take_refs: u32,
    make_refs: u32,
    take_struct: u32,
    take_eq: u32,
    take_i31: u32,
    take_array: u32,
}

/// Function indices of the host imports.
#[derive(Clone, Copy)]
struct HostFuncs {
    gc: u32,
    take_refs: u32,
    make_refs: u32,
    take_struct: u32,
    take_eq: u32,
    take_i31: u32,
    take_array: u32,
    /// Bank of `take_struct_N` / `take_array_N` imports; see `RootBanks`.
    typed_base: u32,
}

impl HostFuncs {
    fn typed(&self, dense: u32) -> u32 {
        self.typed_base + dense
    }
}

/// One slot per abstract reference type plus a bank of typed slots. A bank is a
/// run of consecutive slots, one per concrete type, at `base + dense`.
#[derive(Clone, Copy)]
struct RootBanks {
    structref: u32,
    eqref: u32,
    i31ref: u32,
    arrayref: u32,
    typed_base: u32,
}

impl RootBanks {
    fn typed(&self, dense: u32) -> u32 {
        self.typed_base + dense
    }
}

/// Locals of `run` after its `externref` parameters; see `RootBanks` for banks.
#[derive(Clone, Copy)]
struct LocalBanks {
    /// Temporary for `table.set` on the `externref` table.
    extern_scratch: u32,
    structref: u32,
    eqref: u32,
    i31ref: u32,
    arrayref: u32,
    /// Typed roots, also the temporaries of null-guarded ops on that type.
    typed_base: u32,
    /// Second operand for ops on two values of one type (`array.copy`).
    typed2_base: u32,
    /// One shared prototype per type; see `Types::prototype_types`.
    proto_base: u32,
}

impl LocalBanks {
    fn typed(&self, dense: u32) -> u32 {
        self.typed_base + dense
    }

    fn typed2(&self, dense: u32) -> u32 {
        self.typed2_base + dense
    }

    fn proto(&self, dense: u32) -> u32 {
        self.proto_base + dense
    }
}

/// Where everything lives in the encoded module. `dense` arguments are positions
/// in the encoding order of the concrete types, which is what `type_index` holds.
#[derive(Clone, Copy)]
pub(crate) struct WasmEncodingBases {
    funcs: HostFuncs,
    /// Wasm index of the first concrete type.
    struct_type_base: u32,
    locals: LocalBanks,
    globals: RootBanks,
    tables: RootBanks,
    /// Length of every array `ArrayNew` / `ArrayNewDefault` creates.
    array_length: u32,
}

impl WasmEncodingBases {
    fn wasm_type(&self, dense: u32) -> u32 {
        self.struct_type_base + dense
    }

    /// The slots of one root kind. `index` is the op's own immediate: the param or
    /// global of an `Extern` access, unused for the other kinds.
    fn root_slots(&self, kind: RefKind, index: u32) -> RootSlots {
        let (l, g, t) = (&self.locals, &self.globals, &self.tables);
        match kind {
            RefKind::Extern => RootSlots {
                local: index,
                global: index,
                table: 0,
                tmp: l.extern_scratch,
            },
            RefKind::Struct => RootSlots {
                local: l.structref,
                global: g.structref,
                table: t.structref,
                tmp: l.structref,
            },
            RefKind::Eq => RootSlots {
                local: l.eqref,
                global: g.eqref,
                table: t.eqref,
                tmp: l.eqref,
            },
            RefKind::I31 => RootSlots {
                local: l.i31ref,
                global: g.i31ref,
                table: t.i31ref,
                tmp: l.i31ref,
            },
            RefKind::Array => RootSlots {
                local: l.arrayref,
                global: g.arrayref,
                table: t.arrayref,
                tmp: l.arrayref,
            },
            RefKind::Typed(d) => RootSlots {
                local: l.typed(d),
                global: g.typed(d),
                table: t.typed(d),
                tmp: l.typed(d),
            },
        }
    }
}

/// The kinds of reference the module keeps roots for; `Typed` is a dense type index.
#[derive(Clone, Copy, Debug)]
enum RefKind {
    Extern,
    Struct,
    Eq,
    I31,
    Array,
    Typed(u32),
}

/// Where a root access reads or writes. The `u32` is the param or global index
/// for `Extern` (unused for other kinds), or the table element index.
#[derive(Clone, Copy, Debug)]
enum Storage {
    Local(u32),
    Global(u32),
    Table(u32),
}

impl Storage {
    fn index(self) -> u32 {
        match self {
            Self::Local(i) | Self::Global(i) | Self::Table(i) => i,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Dir {
    Get,
    Set,
}

/// The local, global, table and `table.set` temporary of one root kind.
#[derive(Clone, Copy)]
struct RootSlots {
    local: u32,
    global: u32,
    table: u32,
    tmp: u32,
}

/// Local declarations of a function, handing out indices in declaration order.
struct LocalDecls {
    decls: Vec<(u32, ValType)>,
    next: u32,
}

impl LocalDecls {
    fn new(num_params: u32) -> Self {
        Self {
            decls: Vec::new(),
            next: num_params,
        }
    }

    fn declare(&mut self, ty: ValType) -> u32 {
        let index = self.next;
        self.next += 1;
        self.decls.push((1, ty));
        index
    }

    /// Declare a bank of `count` locals; returns its base index.
    fn declare_bank(&mut self, count: u32, ty: impl Fn(u32) -> ValType) -> u32 {
        let base = self.next;
        for i in 0..count {
            self.declare(ty(i));
        }
        base
    }
}

/// The function types of the host imports and of `run`, which takes `num_params` `externref`s.
fn host_function_types(types: &mut TypeSection, num_params: u32) -> HostTypes {
    let three_refs = vec![ValType::EXTERNREF, ValType::EXTERNREF, ValType::EXTERNREF];

    // `gc` returns a bunch of stuff so that we exercise GCing when there is
    // return pointer space allocated on the stack. This is especially
    // important because the x64 backend currently dynamically adjusts the
    // stack pointer for each call that uses return pointers rather than
    // statically allocating space in the stack frame.
    let gc = types.len();
    types.ty().function(vec![], three_refs.clone());

    let run = types.len();
    types.ty().function(
        vec![ValType::EXTERNREF; usize::try_from(num_params).unwrap()],
        vec![],
    );

    let take_refs = types.len();
    types.ty().function(three_refs.clone(), vec![]);

    let make_refs = types.len();
    types.ty().function(vec![], three_refs);

    let take_struct = types.len();
    types.ty().function(vec![ValType::Ref(STRUCTREF)], vec![]);

    let take_eq = types.len();
    types
        .ty()
        .function(vec![ValType::Ref(RefType::EQREF)], vec![]);

    // `take_i31` also receives the guest's inline `i31.get_s` / `i31.get_u`
    // results, so the host can check its view of the i31 against them.
    let take_i31 = types.len();
    types.ty().function(
        vec![ValType::Ref(RefType::I31REF), ValType::I32, ValType::I32],
        vec![],
    );

    let take_array = types.len();
    types
        .ty()
        .function(vec![ValType::Ref(RefType::ARRAYREF)], vec![]);

    HostTypes {
        gc,
        run,
        take_refs,
        make_refs,
        take_struct,
        take_eq,
        take_i31,
        take_array,
    }
}

/// One `(func (param (ref null $t)))` type per concrete type; returns the first's index.
fn typed_take_types(types: &mut TypeSection, struct_type_base: u32, concrete_count: u32) -> u32 {
    // Not `types.len()`: a rec group is one section entry but `concrete_count` type indices.
    let base = struct_type_base + concrete_count;
    for i in 0..concrete_count {
        types
            .ty()
            .function(vec![ValType::Ref(concrete(struct_type_base + i))], vec![]);
    }
    base
}

/// Tables of `table_size`: `externref` (table 0), one per abstract reference type, one per concrete type.
fn encode_tables(
    table_size: u32,
    struct_type_base: u32,
    concrete_count: u32,
) -> (TableSection, RootBanks) {
    let mut tables = TableSection::new();
    tables.table(nullable_table(RefType::EXTERNREF, table_size));

    let structref = tables.len();
    tables.table(nullable_table(STRUCTREF, table_size));

    let eqref = tables.len();
    tables.table(nullable_table(RefType::EQREF, table_size));

    let i31ref = tables.len();
    tables.table(nullable_table(RefType::I31REF, table_size));

    let arrayref = tables.len();
    tables.table(nullable_table(RefType::ARRAYREF, table_size));

    let typed_base = tables.len();
    for i in 0..concrete_count {
        tables.table(nullable_table(concrete(struct_type_base + i), table_size));
    }

    let banks = RootBanks {
        structref,
        eqref,
        i31ref,
        arrayref,
        typed_base,
    };
    (tables, banks)
}

/// Null-initialized globals: `num_globals` `externref`s, one per abstract reference type, one per concrete type.
fn encode_globals(
    num_globals: u32,
    struct_type_base: u32,
    concrete_count: u32,
) -> (GlobalSection, RootBanks) {
    let mut globals = GlobalSection::new();
    for _ in 0..num_globals {
        null_ref_global(&mut globals, HeapType::EXTERN);
    }
    let structref = null_ref_global(&mut globals, STRUCT);
    let eqref = null_ref_global(&mut globals, EQ);
    let i31ref = null_ref_global(&mut globals, HeapType::I31);
    let arrayref = null_ref_global(&mut globals, ARRAY);
    let typed_base = globals.len();
    for i in 0..concrete_count {
        null_ref_global(&mut globals, HeapType::Concrete(struct_type_base + i));
    }

    let banks = RootBanks {
        structref,
        eqref,
        i31ref,
        arrayref,
        typed_base,
    };
    (globals, banks)
}

/// The locals of `run`; see `LocalBanks` for what each one is.
fn declare_locals(
    num_params: u32,
    struct_type_base: u32,
    concrete_count: u32,
) -> (LocalDecls, LocalBanks) {
    let mut locals = LocalDecls::new(num_params);
    let extern_scratch = locals.declare(ValType::EXTERNREF);
    let structref = locals.declare(ValType::Ref(STRUCTREF));
    let eqref = locals.declare(ValType::Ref(RefType::EQREF));
    let i31ref = locals.declare(ValType::Ref(RefType::I31REF));
    let arrayref = locals.declare(ValType::Ref(RefType::ARRAYREF));

    let typed = |i| ValType::Ref(concrete(struct_type_base + i));
    let typed_base = locals.declare_bank(concrete_count, typed);
    let typed2_base = locals.declare_bank(concrete_count, typed);
    let proto_base = locals.declare_bank(concrete_count, typed);

    let banks = LocalBanks {
        extern_scratch,
        structref,
        eqref,
        i31ref,
        arrayref,
        typed_base,
        typed2_base,
        proto_base,
    };
    (locals, banks)
}
/// A description of a Wasm module that performs a series of GC operations on
/// `externref`s, `i31`s, and struct and array objects of its own types.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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
        let mut encoding_order_grouped = Vec::with_capacity(self.types.rec_groups.len());
        self.fixup(&mut encoding_order_grouped);

        // Flat encoding order (dense index -> TypeId), for naming the typed
        // host imports and for field lookups during encoding.
        let encoding_order: Vec<TypeId> = encoding_order_grouped
            .iter()
            .flat_map(|(_, members)| members.iter().copied())
            .collect();

        let mut types = TypeSection::new();
        let host_types = host_function_types(&mut types, self.limits.num_params);
        let struct_type_base = types.len();
        let type_ids_to_index =
            self.encode_concrete_types(&mut types, &encoding_order_grouped, struct_type_base);
        let concrete_count = u32::try_from(type_ids_to_index.len()).unwrap();
        let typed_fn_type_base = typed_take_types(&mut types, struct_type_base, concrete_count);

        let (imports, funcs) = self.encode_imports(
            &host_types,
            typed_fn_type_base,
            struct_type_base,
            &encoding_order,
        );
        let (tables, table_banks) =
            encode_tables(self.limits.table_size, struct_type_base, concrete_count);
        let (globals, global_banks) =
            encode_globals(self.limits.num_globals, struct_type_base, concrete_count);
        let (local_decls, local_banks) =
            declare_locals(self.limits.num_params, struct_type_base, concrete_count);

        // `run` is the first (and only) defined function, right after the imports.
        let mut functions = FunctionSection::new();
        let mut exports = ExportSection::new();
        functions.function(host_types.run);
        exports.export("run", ExportKind::Func, imports.len());

        let bases = WasmEncodingBases {
            funcs,
            struct_type_base,
            locals: local_banks,
            globals: global_banks,
            tables: table_banks,
            array_length: self.limits.array_length,
        };
        let func = self.encode_run_body(local_decls, bases, &type_ids_to_index, &encoding_order);
        let mut code = CodeSection::new();
        code.function(&func);

        let mut module = Module::new();
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

    /// Emit every rec group in encoding order; returns the Wasm type index of each type.
    fn encode_concrete_types(
        &self,
        types: &mut TypeSection,
        encoding_order_grouped: &[(RecGroupId, Vec<TypeId>)],
        struct_type_base: u32,
    ) -> BTreeMap<TypeId, u32> {
        // Build the type-id-to-wasm-index map from the pre-computed
        // encoding order (rec groups in topo order, members sorted by
        // supertype-first within each group).
        let mut type_ids_to_index: BTreeMap<TypeId, u32> = BTreeMap::new();
        let mut next_idx = struct_type_base;
        for (_, members) in encoding_order_grouped {
            for &tid in members {
                type_ids_to_index.insert(tid, next_idx);
                next_idx += 1;
            }
        }

        let encode_ty_id = |ty_id: &TypeId| -> wasm_encoder::SubType {
            let def = &self.types.type_defs[ty_id];
            let inner = match &def.composite_type {
                CompositeType::Struct(st) => {
                    let fields: Box<[wasm_encoder::FieldType]> = st
                        .fields
                        .iter()
                        .map(|f| wasm_encoder::FieldType {
                            element_type: f.field_type.to_storage_type(&type_ids_to_index),
                            mutable: f.mutable,
                        })
                        .collect();
                    wasm_encoder::CompositeInnerType::Struct(wasm_encoder::StructType { fields })
                }
                CompositeType::Array(at) => {
                    let element = wasm_encoder::FieldType {
                        element_type: at.element.field_type.to_storage_type(&type_ids_to_index),
                        mutable: at.element.mutable,
                    };
                    wasm_encoder::CompositeInnerType::Array(wasm_encoder::ArrayType(element))
                }
            };
            wasm_encoder::SubType {
                is_final: def.is_final,
                supertype_idx: def.supertype.map(|st| type_ids_to_index[&st]),
                composite_type: wasm_encoder::CompositeType {
                    inner,
                    shared: false,
                    describes: None,
                    descriptor: None,
                },
            }
        };

        for (_, group_members) in encoding_order_grouped {
            let members: Vec<wasm_encoder::SubType> =
                group_members.iter().map(encode_ty_id).collect();
            types.ty().rec(members);
        }

        type_ids_to_index
    }

    /// The host imports: the fixed ones, then one `take_*` per concrete type.
    fn encode_imports(
        &self,
        host_types: &HostTypes,
        typed_fn_type_base: u32,
        struct_type_base: u32,
        encoding_order: &[TypeId],
    ) -> (ImportSection, HostFuncs) {
        let mut imports = ImportSection::new();
        let mut import_func = |name: &str, type_idx: u32| -> u32 {
            let index = imports.len();
            imports.import("", name, EntityType::Function(type_idx));
            index
        };
        let gc = import_func("gc", host_types.gc);
        let take_refs = import_func("take_refs", host_types.take_refs);
        let make_refs = import_func("make_refs", host_types.make_refs);
        let take_struct = import_func("take_struct", host_types.take_struct);
        let take_eq = import_func("take_eq", host_types.take_eq);
        let take_i31 = import_func("take_i31", host_types.take_i31);
        let take_array = import_func("take_array", host_types.take_array);

        // The import name records the kind so the host can define it appropriately.
        let typed_base = imports.len();
        for (i, tid) in (0u32..).zip(encoding_order) {
            let wasm_idx = struct_type_base + i;
            let is_array = self
                .types
                .type_defs
                .get(tid)
                .map(|def| def.composite_type.is_array())
                .unwrap_or(false);
            let name = if is_array {
                format!("take_array_{wasm_idx}")
            } else {
                format!("take_struct_{wasm_idx}")
            };
            imports.import("", &name, EntityType::Function(typed_fn_type_base + i));
        }

        let funcs = HostFuncs {
            gc,
            take_refs,
            make_refs,
            take_struct,
            take_eq,
            take_i31,
            take_array,
            typed_base,
        };
        (imports, funcs)
    }

    /// The body of `run`: an endless loop that refills the prototypes, then runs every op.
    fn encode_run_body(
        &self,
        locals: LocalDecls,
        bases: WasmEncodingBases,
        type_ids_to_index: &BTreeMap<TypeId, u32>,
        encoding_order: &[TypeId],
    ) -> Function {
        let mut inhabitable = BTreeMap::new();
        self.types.inhabitable(&mut inhabitable);
        let struct_ref_target = self.types.least_rank_inhabitable_struct(&inhabitable);

        // Map each prototyped type to the local holding its instance. A type
        // absent from this map is cheap enough to rebuild at every use.
        let proto_order = self
            .types
            .prototype_types(&inhabitable, MAX_INLINE_CONSTRUCTION);
        let protos: BTreeMap<TypeId, u32> = proto_order
            .iter()
            .map(|tid| {
                let dense = type_ids_to_index[tid] - bases.struct_type_base;
                (*tid, bases.locals.proto(dense))
            })
            .collect();

        let ctx = EmitCtx {
            types: &self.types,
            struct_ref_target,
            protos: &protos,
            type_ids_to_index,
            bases,
            encoding_order,
        };

        let mut func = Function::new(locals.decls);
        func.instruction(&Instruction::Loop(BlockType::Empty));

        // Refill the prototypes at the top of every iteration, so the ops below
        // never read a null local and each iteration allocates a fresh set of
        // objects for the collector to find and reclaim. `proto_order` is in
        // rank order, so a prototype's referents are already built.
        for tid in &proto_order {
            emit_new(*tid, &mut func, ctx);
            func.instruction(&Instruction::LocalSet(protos[tid]));
        }
        for op in &self.ops {
            op.encode(&mut func, ctx);
        }
        func.instruction(&Instruction::Br(0));
        func.instruction(&Instruction::End);
        func.instruction(&Instruction::End);
        func
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
    pub fn fixup(&mut self, encoding_order_grouped: &mut Vec<(RecGroupId, Vec<TypeId>)>) {
        self.limits.fixup_limits();
        self.types.fixup_types(&self.limits, encoding_order_grouped);
        let encoding_order: Vec<TypeId> = encoding_order_grouped
            .iter()
            .flat_map(|(_, members)| members.iter().copied())
            .collect();

        let mut new_ops = Vec::with_capacity(self.ops.len());
        let mut stack: Vec<StackType> = Vec::new();
        let num_types = u32::try_from(self.types.type_defs.len()).unwrap();

        // Concrete encoding indices split by kind, so typed ops can be remapped
        // onto a same-kind type (struct ops never point at an array, etc.).
        let mut struct_type_indices = Vec::new();
        let mut array_type_indices = Vec::new();
        for (i, tid) in encoding_order.iter().enumerate() {
            let i = u32::try_from(i).unwrap();
            match self.types.type_defs.get(tid) {
                Some(def) if def.composite_type.is_array() => array_type_indices.push(i),
                Some(_) => struct_type_indices.push(i),
                None => {}
            }
        }

        let mut operand_types = Vec::new();
        for op in &self.ops {
            let Some(op) = op.fixup_immediates(
                &self.limits,
                num_types,
                &struct_type_indices,
                &array_type_indices,
            ) else {
                continue;
            };
            let op = StackType::fixup_cast(op, &self.types, &encoding_order);

            debug_assert!(operand_types.is_empty());
            op.operand_types(&mut operand_types);
            for ty in operand_types.drain(..) {
                StackType::fixup_operand(
                    ty,
                    &mut stack,
                    &mut new_ops,
                    num_types,
                    &self.types,
                    &encoding_order,
                );
            }

            // Finally, emit the op itself (updates stack abstractly)
            let mut result_types = Vec::new();
            StackType::emit(op, &mut stack, &mut new_ops, num_types, &mut result_types);
        }

        // Drop any remaining values on the operand stack.
        for _ in 0..stack.len() {
            new_ops.push(GcOp::Drop);
        }

        log::trace!("ops after fixup: {new_ops:#?}");
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

macro_rules! for_each_gc_op {
    ( $mac:ident ) => {
        $mac! {
            #[operands([])]
            #[results([ExternRef, ExternRef, ExternRef])]
            Gc,

            #[operands([])]
            #[results([ExternRef, ExternRef, ExternRef])]
            MakeRefs,

            #[operands([Some(ExternRef), Some(ExternRef), Some(ExternRef)])]
            #[results([])]
            TakeRefs,

            #[operands([])]
            #[results([ExternRef])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                // Add one to make sure that out-of-bounds table accesses are
                // possible, but still rare.
                elem_index = elem_index % (limits.table_size + 1);
            })]
            TableGet { elem_index: u32 },

            #[operands([Some(ExternRef)])]
            #[results([])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                // Add one to make sure that out-of-bounds table accesses are
                // possible, but still rare.
                elem_index = elem_index % (limits.table_size + 1);
            })]
            TableSet { elem_index: u32 },

            #[operands([])]
            #[results([ExternRef])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                global_index = global_index.checked_rem(limits.num_globals)?;
            })]
            GlobalGet { global_index:  u32 },

            #[operands([Some(ExternRef)])]
            #[results([])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                global_index = global_index.checked_rem(limits.num_globals)?;
            })]
            GlobalSet { global_index: u32 },

            #[operands([])]
            #[results([ExternRef])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                local_index = local_index.checked_rem(limits.num_params)?;
            })]
            LocalGet { local_index: u32 },

            #[operands([Some(ExternRef)])]
            #[results([])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                local_index = local_index.checked_rem(limits.num_params)?;
            })]
            LocalSet { local_index: u32 },

            #[operands([])]
            #[results([Struct(Some(type_index))])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                type_index = pick_type_index(struct_type_indices, type_index)?;
            })]
            StructNew { type_index: u32 },

            #[operands([])]
            #[results([Struct(Some(type_index))])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                type_index = pick_type_index(struct_type_indices, type_index)?;
            })]
            StructNewDefault { type_index: u32 },

            #[operands([Some(Struct(None))])]
            #[results([])]
            TakeStructCall,

            #[operands([Some(Struct(Some(type_index)))])]
            #[results([])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                type_index = pick_type_index(struct_type_indices, type_index)?;
            })]
            TakeTypedStructCall { type_index: u32 },

            #[operands([Some(Struct(None))])]
            #[results([])]
            StructLocalSet,

            #[operands([])]
            #[results([Struct(None)])]
            StructLocalGet,

            #[operands([Some(Struct(Some(type_index)))])]
            #[results([])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                type_index = pick_type_index(struct_type_indices, type_index)?;
            })]
            TypedStructLocalSet { type_index: u32 },

            #[operands([])]
            #[results([Struct(Some(type_index))])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                type_index = pick_type_index(struct_type_indices, type_index)?;
            })]
            TypedStructLocalGet { type_index: u32 },

            #[operands([Some(Struct(None))])]
            #[results([])]
            StructGlobalSet,

            #[operands([])]
            #[results([Struct(None)])]
            StructGlobalGet,

            #[operands([Some(Struct(Some(type_index)))])]
            #[results([])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                type_index = pick_type_index(struct_type_indices, type_index)?;
            })]
            TypedStructGlobalSet { type_index: u32 },

            #[operands([])]
            #[results([Struct(Some(type_index))])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                type_index = pick_type_index(struct_type_indices, type_index)?;
            })]
            TypedStructGlobalGet { type_index: u32 },

            #[operands([Some(Struct(None))])]
            #[results([])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                // Add one to make sure that out-of-bounds table accesses are
                // possible, but still rare.
                elem_index = elem_index % (limits.table_size + 1);
            })]
            StructTableSet { elem_index: u32 },

            #[operands([])]
            #[results([Struct(None)])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                // Add one to make sure that out-of-bounds table accesses are
                // possible, but still rare.
                elem_index = elem_index % (limits.table_size + 1);
            })]
            StructTableGet { elem_index: u32 },

            #[operands([Some(Struct(Some(type_index)))])]
            #[results([])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                // Add one to make sure that out-of-bounds table accesses are
                // possible, but still rare.
                elem_index = elem_index % (limits.table_size + 1);
                type_index = pick_type_index(struct_type_indices, type_index)?;
            })]
            TypedStructTableSet { elem_index: u32, type_index: u32 },

            #[operands([])]
            #[results([Struct(Some(type_index))])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                // Add one to make sure that out-of-bounds table accesses are
                // possible, but still rare.
                elem_index = elem_index % (limits.table_size + 1);
                type_index = pick_type_index(struct_type_indices, type_index)?;
            })]
            TypedStructTableGet { elem_index: u32, type_index: u32 },

            #[operands([None])]
            #[results([])]
            Drop,

            #[operands([])]
            #[results([ExternRef])]
            NullExtern,

            #[operands([])]
            #[results([Struct(None)])]
            NullStruct,

            #[operands([])]
            #[results([Eq])]
            NullEq,

            #[operands([Some(Eq)])]
            #[results([])]
            TakeEqCall,

            #[operands([Some(Eq)])]
            #[results([])]
            EqLocalSet,

            #[operands([])]
            #[results([Eq])]
            EqLocalGet,

            #[operands([Some(Eq)])]
            #[results([])]
            EqGlobalSet,

            #[operands([])]
            #[results([Eq])]
            EqGlobalGet,

            #[operands([Some(Eq)])]
            #[results([])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                // Add one to make sure that out-of-bounds table accesses are
                // possible, but still rare.
                elem_index = elem_index % (limits.table_size + 1);
            })]
            EqTableSet { elem_index: u32 },

            #[operands([])]
            #[results([Eq])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                // Add one to make sure that out-of-bounds table accesses are
                // possible, but still rare.
                elem_index = elem_index % (limits.table_size + 1);
            })]
            EqTableGet { elem_index: u32 },

            #[operands([])]
            #[results([Struct(Some(type_index))])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                type_index = pick_type_index(struct_type_indices, type_index)?;
            })]
            NullTypedStruct { type_index: u32 },

            #[operands([Some(Struct(Some(sub_type_index)))])]
            #[results([Struct(Some(super_type_index))])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                sub_type_index = pick_type_index(struct_type_indices, sub_type_index)?;
                super_type_index = pick_type_index(struct_type_indices, super_type_index)?;
            })]
            RefCastUpward { sub_type_index: u32, super_type_index: u32 },

            #[operands([Some(Struct(Some(super_type_index)))])]
            #[results([Struct(Some(sub_type_index))])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                sub_type_index = pick_type_index(struct_type_indices, sub_type_index)?;
                super_type_index = pick_type_index(struct_type_indices, super_type_index)?;
            })]
            RefCastDownward { sub_type_index: u32, super_type_index: u32 },

            #[operands([Some(Struct(Some(type_index)))])]
            #[results([])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                type_index = pick_type_index(struct_type_indices, type_index)?;
            })]
            StructGet { type_index: u32, field_index: u32 },

            #[operands([Some(Struct(Some(type_index)))])]
            #[results([])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                type_index = pick_type_index(struct_type_indices, type_index)?;
            })]
            StructGetU { type_index: u32, field_index: u32 },

            #[operands([Some(Struct(Some(type_index)))])]
            #[results([])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                type_index = pick_type_index(struct_type_indices, type_index)?;
            })]
            StructSet { type_index: u32, field_index: u32 },

            #[operands([])]
            #[results([I31])]
            NullI31,

            #[operands([])]
            #[results([I31])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                // Any `i32` is a valid operand to `ref.i31` (it wraps to 31
                // bits), so no clamping is needed.
            })]
            RefI31 { value: u32 },

            #[operands([Some(I31)])]
            #[results([])]
            I31LocalSet,

            #[operands([])]
            #[results([I31])]
            I31LocalGet,

            #[operands([Some(I31)])]
            #[results([])]
            I31GlobalSet,

            #[operands([])]
            #[results([I31])]
            I31GlobalGet,

            #[operands([Some(I31)])]
            #[results([])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                // Add one to make sure that out-of-bounds table accesses are
                // possible, but still rare.
                elem_index = elem_index % (limits.table_size + 1);
            })]
            I31TableSet { elem_index: u32 },

            #[operands([])]
            #[results([I31])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                // Add one to make sure that out-of-bounds table accesses are
                // possible, but still rare.
                elem_index = elem_index % (limits.table_size + 1);
            })]
            I31TableGet { elem_index: u32 },

            #[operands([Some(I31)])]
            #[results([])]
            TakeI31Call,

            #[operands([Some(I31)])]
            #[results([])]
            I31GetS,

            #[operands([Some(I31)])]
            #[results([])]
            I31GetU,

            #[operands([Some(Struct(None))])]
            #[results([Eq])]
            StructRefAsEq,

            #[operands([Some(Struct(Some(type_index)))])]
            #[results([Eq])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                type_index = pick_type_index(struct_type_indices, type_index)?;
            })]
            TypedStructRefAsEq { type_index: u32 },

            #[operands([Some(I31)])]
            #[results([Eq])]
            I31RefAsEq,

            #[operands([])]
            #[results([Array(Some(type_index))])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                type_index = pick_type_index(array_type_indices, type_index)?;
            })]
            ArrayNewDefault { type_index: u32 },

            #[operands([])]
            #[results([Array(Some(type_index))])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                type_index = pick_type_index(array_type_indices, type_index)?;
            })]
            ArrayNew { type_index: u32 },

            #[operands([])]
            #[results([Array(Some(type_index))])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                n = n % (limits.array_length + 1);
                type_index = pick_type_index(array_type_indices, type_index)?;
            })]
            ArrayNewFixed { type_index: u32, n: u32 },

            #[operands([])]
            #[results([Array(None)])]
            NullArray,

            #[operands([])]
            #[results([Array(Some(type_index))])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                type_index = pick_type_index(array_type_indices, type_index)?;
            })]
            NullTypedArray { type_index: u32 },

            #[operands([Some(Array(None))])]
            #[results([])]
            TakeArrayCall,

            #[operands([Some(Array(Some(type_index)))])]
            #[results([])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                type_index = pick_type_index(array_type_indices, type_index)?;
            })]
            TakeTypedArrayCall { type_index: u32 },

            #[operands([Some(Array(None))])]
            #[results([])]
            ArrayLocalSet,

            #[operands([])]
            #[results([Array(None)])]
            ArrayLocalGet,

            #[operands([Some(Array(Some(type_index)))])]
            #[results([])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                type_index = pick_type_index(array_type_indices, type_index)?;
            })]
            TypedArrayLocalSet { type_index: u32 },

            #[operands([])]
            #[results([Array(Some(type_index))])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                type_index = pick_type_index(array_type_indices, type_index)?;
            })]
            TypedArrayLocalGet { type_index: u32 },

            #[operands([Some(Array(None))])]
            #[results([])]
            ArrayGlobalSet,

            #[operands([])]
            #[results([Array(None)])]
            ArrayGlobalGet,

            #[operands([Some(Array(Some(type_index)))])]
            #[results([])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                type_index = pick_type_index(array_type_indices, type_index)?;
            })]
            TypedArrayGlobalSet { type_index: u32 },

            #[operands([])]
            #[results([Array(Some(type_index))])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                type_index = pick_type_index(array_type_indices, type_index)?;
            })]
            TypedArrayGlobalGet { type_index: u32 },

            #[operands([Some(Array(None))])]
            #[results([])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                // Add one to make sure that out-of-bounds table accesses are
                // possible, but still rare.
                elem_index = elem_index % (limits.table_size + 1);
            })]
            ArrayTableSet { elem_index: u32 },

            #[operands([])]
            #[results([Array(None)])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                // Add one to make sure that out-of-bounds table accesses are
                // possible, but still rare.
                elem_index = elem_index % (limits.table_size + 1);
            })]
            ArrayTableGet { elem_index: u32 },

            #[operands([Some(Array(Some(type_index)))])]
            #[results([])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                // Add one to make sure that out-of-bounds table accesses are
                // possible, but still rare.
                elem_index = elem_index % (limits.table_size + 1);
                type_index = pick_type_index(array_type_indices, type_index)?;
            })]
            TypedArrayTableSet { elem_index: u32, type_index: u32 },

            #[operands([])]
            #[results([Array(Some(type_index))])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                // Add one to make sure that out-of-bounds table accesses are
                // possible, but still rare.
                elem_index = elem_index % (limits.table_size + 1);
                type_index = pick_type_index(array_type_indices, type_index)?;
            })]
            TypedArrayTableGet { elem_index: u32, type_index: u32 },

            #[operands([Some(Array(Some(sub_type_index)))])]
            #[results([Array(Some(super_type_index))])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                sub_type_index = pick_type_index(array_type_indices, sub_type_index)?;
                super_type_index = pick_type_index(array_type_indices, super_type_index)?;
            })]
            ArrayRefCastUpward { sub_type_index: u32, super_type_index: u32 },

            #[operands([Some(Array(Some(super_type_index)))])]
            #[results([Array(Some(sub_type_index))])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                sub_type_index = pick_type_index(array_type_indices, sub_type_index)?;
                super_type_index = pick_type_index(array_type_indices, super_type_index)?;
            })]
            ArrayRefCastDownward { sub_type_index: u32, super_type_index: u32 },

            #[operands([Some(Array(Some(type_index)))])]
            #[results([])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                // Keep most indices in-bounds: an array is exactly `array_length`
                // long, so `% (array_length + 1)` is out-of-bounds only for one
                // index value, making OOB traps possible but rare.
                index = index % (limits.array_length + 1);
                type_index = pick_type_index(array_type_indices, type_index)?;
            })]
            ArrayGet { type_index: u32, index: u32 },

            #[operands([Some(Array(Some(type_index)))])]
            #[results([])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                index = index % (limits.array_length + 1);
                type_index = pick_type_index(array_type_indices, type_index)?;
            })]
            ArrayGetU { type_index: u32, index: u32 },

            #[operands([Some(Array(Some(type_index)))])]
            #[results([])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                index = index % (limits.array_length + 1);
                type_index = pick_type_index(array_type_indices, type_index)?;
            })]
            ArraySet { type_index: u32, index: u32 },

            #[operands([Some(Array(Some(type_index)))])]
            #[results([])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                offset = offset % (limits.array_length + 1);
                len = len % (limits.array_length - offset + 2);
                type_index = pick_type_index(array_type_indices, type_index)?;
            })]
            ArrayFill { type_index: u32, offset: u32, len: u32 },

            #[operands([Some(Array(Some(type_index))), Some(Array(Some(type_index)))])]
            #[results([])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                dst_offset = dst_offset % (limits.array_length + 1);
                src_offset = src_offset % (limits.array_length + 1);
                len = len % (limits.array_length - dst_offset.max(src_offset) + 2);
                type_index = pick_type_index(array_type_indices, type_index)?;
            })]
            ArrayCopy { type_index: u32, dst_offset: u32, src_offset: u32, len: u32 },

            #[operands([Some(Array(None))])]
            #[results([])]
            ArrayLen,

            #[operands([Some(Array(None))])]
            #[results([Eq])]
            ArrayRefAsEq,

            #[operands([Some(Array(Some(type_index)))])]
            #[results([Eq])]
            #[fixup(|limits, num_types, struct_type_indices, array_type_indices| {
                type_index = pick_type_index(array_type_indices, type_index)?;
            })]
            TypedArrayRefAsEq { type_index: u32 },
        }
    };
}

macro_rules! define_gc_op_variants {
    (
        $(
            $( #[$attr:meta] )*
            $op:ident $( { $( $field:ident : $field_ty:ty ),* } )? ,
        )*
    ) => {
        /// The operations that can be performed by the `gc` function.
        #[derive(
            Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, mutatis::Mutate,
        )]
        #[mutatis(default_mutate = false)]
        #[allow(missing_docs, reason = "self-describing")]
        pub enum GcOp {
            $(
                $op $( { $( #[mutatis(default_mutate)] $field : $field_ty ),* } )? ,
            )*
        }
    };
}
for_each_gc_op!(define_gc_op_variants);

macro_rules! define_op_names {
    (
        $(
            $( #[$attr:meta] )*
            $op:ident $( { $( $field:ident : $field_ty:ty ),* } )? ,
        )*
    ) => {
        #[cfg(test)]
        pub(crate) const OP_NAMES: &[&str] = &[
            $(stringify!($op)),*
        ];
    }
}
for_each_gc_op!(define_op_names);

impl GcOp {
    #[cfg(test)]
    pub(crate) fn name(&self) -> &'static str {
        macro_rules! define_gc_op_name {
            (
                $(
                    $( #[$attr:meta] )*
                    $op:ident $( { $( $field:ident : $field_ty:ty ),* } )? ,
                )*
            ) => {
                match self {
                    $(
                        Self::$op $( { $($field: _),* } )? => stringify!($op),
                    )*
                }
            };
        }
        for_each_gc_op!(define_gc_op_name)
    }

    pub(crate) fn operand_types(&self, out: &mut Vec<Option<StackType>>) {
        macro_rules! define_gc_op_operand_types {
            (
                $(
                    #[operands($operands:expr)]
                    $( #[$attr:meta] )*
                    $op:ident $( { $( $field:ident : $field_ty:ty ),* } )? ,
                )*
            ) => {{
                use StackType::*;
                match self {
                    $(
                        Self::$op $( { $($field),* } )? => {
                            $(
                                $(
                                    #[allow(unused, reason = "macro code")]
                                    let $field = *$field;
                                )*
                            )?
                            let operands: [Option<StackType>; _] = $operands;
                            out.extend(operands);
                        }
                    )*
                }
            }};
        }
        for_each_gc_op!(define_gc_op_operand_types)
    }

    pub(crate) fn result_types(&self, out: &mut Vec<StackType>) {
        macro_rules! define_gc_op_result_types {
            (
                $(
                    #[operands($operands:expr)]
                    #[results($results:expr)]
                    $( #[$attr:meta] )*
                    $op:ident $( { $( $field:ident : $field_ty:ty ),* } )? ,
                )*
            ) => {{
                use StackType::*;
                match self {
                    $(
                        Self::$op $( { $($field),* } )? => {
                            $(
                                $(
                                    #[allow(unused, reason = "macro code")]
                                    let $field = *$field;
                                )*
                            )?
                            let results: [StackType; _] = $results;
                            out.extend(results);
                        }
                    )*
                }
            }};
        }
        for_each_gc_op!(define_gc_op_result_types)
    }

    /// Fix up an op's immediates. `struct_type_indices` / `array_type_indices`
    /// are the concrete encoding indices of each kind; a typed op remaps its
    /// type index into the matching set so struct ops never point at an array
    /// (or vice versa), and drops itself if no type of that kind exists.
    pub(crate) fn fixup_immediates(
        &self,
        limits: &GcOpsLimits,
        num_types: u32,
        struct_type_indices: &[u32],
        array_type_indices: &[u32],
    ) -> Option<Self> {
        macro_rules! define_gc_op_fixup {
            (
                $(
                    #[operands($operands:expr)]
                    #[results($results:expr)]
                    $( #[fixup(|$limits:ident, $num_types:ident, $structs:ident, $arrays:ident| $fixup:expr)] )?
                    $op:ident $( { $( $field:ident : $field_ty:ty ),* } )? ,
                )*
            ) => {{
                let _ = (limits, num_types, struct_type_indices, array_type_indices);
                match self {
                    $(
                        Self::$op $( { $($field),* } )? => {
                            $(
                                $(
                                    #[allow(unused_mut, unused_assignments, reason = "macro code")]
                                    let mut $field = *$field;
                                )*
                                #[allow(unused_variables, reason = "macro code")]
                                let $limits = limits;
                                #[allow(unused_variables, reason = "macro code")]
                                let $num_types = num_types;
                                #[allow(unused_variables, reason = "macro code")]
                                let $structs = struct_type_indices;
                                #[allow(unused_variables, reason = "macro code")]
                                let $arrays = array_type_indices;
                                $fixup;
                            )?
                            Some(Self::$op $( { $( $field ),* } )? )
                        }
                    )*
                }
            }};
        }
        for_each_gc_op!(define_gc_op_fixup)
    }

    fn encode(&self, func: &mut Function, cx: EmitCtx<'_>) {
        let bases = cx.bases;
        use Dir::{Get, Set};
        use RefKind::{Array, Eq, Extern, I31, Struct, Typed};
        use Storage::{Global, Local, Table};
        match *self {
            // Host calls.
            Self::Gc => {
                func.instruction(&Instruction::Call(bases.funcs.gc));
            }
            Self::MakeRefs => {
                func.instruction(&Instruction::Call(bases.funcs.make_refs));
            }
            Self::TakeRefs => {
                func.instruction(&Instruction::Call(bases.funcs.take_refs));
            }
            Self::TakeStructCall => {
                func.instruction(&Instruction::Call(bases.funcs.take_struct));
            }
            Self::TakeEqCall => {
                func.instruction(&Instruction::Call(bases.funcs.take_eq));
            }
            Self::TakeArrayCall => {
                func.instruction(&Instruction::Call(bases.funcs.take_array));
            }
            Self::TakeTypedStructCall { type_index } | Self::TakeTypedArrayCall { type_index } => {
                func.instruction(&Instruction::Call(bases.funcs.typed(type_index)));
            }
            Self::TakeI31Call => encode_take_i31(func, bases),

            // Nulls and `i31` values.
            Self::NullExtern => {
                func.instruction(&Instruction::RefNull(HeapType::EXTERN));
            }
            Self::NullStruct => {
                func.instruction(&Instruction::RefNull(STRUCT));
            }
            Self::NullEq => {
                func.instruction(&Instruction::RefNull(EQ));
            }
            Self::NullI31 => {
                func.instruction(&Instruction::RefNull(HeapType::I31));
            }
            Self::NullArray => {
                func.instruction(&Instruction::RefNull(ARRAY));
            }
            Self::NullTypedStruct { type_index } | Self::NullTypedArray { type_index } => {
                let ty = HeapType::Concrete(bases.wasm_type(type_index));
                func.instruction(&Instruction::RefNull(ty));
            }
            Self::RefI31 { value } => {
                func.instruction(&Instruction::I32Const(value.cast_signed()));
                func.instruction(&Instruction::RefI31);
            }

            // Allocation.
            Self::StructNew { type_index } => encode_struct_new(func, cx, type_index, false),
            Self::StructNewDefault { type_index } => encode_struct_new(func, cx, type_index, true),
            Self::ArrayNew { type_index } => encode_array_new(func, cx, type_index),
            Self::ArrayNewDefault { type_index } => encode_array_new_default(func, cx, type_index),
            Self::ArrayNewFixed { type_index, n } => {
                encode_array_new_fixed(func, cx, type_index, n)
            }

            // Casts. Upcasting to `eqref` is implicit in Wasm subtyping, so
            // those ops emit nothing; only the abstract stack type changes.
            Self::RefCastUpward {
                super_type_index, ..
            }
            | Self::ArrayRefCastUpward {
                super_type_index, ..
            } => encode_upcast(func, bases, super_type_index),
            Self::RefCastDownward {
                sub_type_index,
                super_type_index,
            }
            | Self::ArrayRefCastDownward {
                sub_type_index,
                super_type_index,
            } => encode_downcast(func, bases, sub_type_index, super_type_index),
            Self::StructRefAsEq
            | Self::TypedStructRefAsEq { .. }
            | Self::ArrayRefAsEq
            | Self::TypedArrayRefAsEq { .. }
            | Self::I31RefAsEq => {}

            // Field, element and payload access.
            Self::StructGet {
                type_index,
                field_index,
            } => encode_struct_get(func, cx, type_index, field_index, false),
            Self::StructGetU {
                type_index,
                field_index,
            } => encode_struct_get(func, cx, type_index, field_index, true),
            Self::StructSet {
                type_index,
                field_index,
            } => encode_struct_set(func, cx, type_index, field_index),
            Self::ArrayGet { type_index, index } => {
                encode_array_get(func, cx, type_index, index, false)
            }
            Self::ArrayGetU { type_index, index } => {
                encode_array_get(func, cx, type_index, index, true)
            }
            Self::ArraySet { type_index, index } => encode_array_set(func, cx, type_index, index),
            Self::ArrayFill {
                type_index,
                offset,
                len,
            } => encode_array_fill(func, cx, type_index, offset, len),
            Self::ArrayCopy {
                type_index,
                dst_offset,
                src_offset,
                len,
            } => encode_array_copy(func, cx, type_index, dst_offset, src_offset, len),
            Self::ArrayLen => encode_array_len(func, bases),
            Self::I31GetS => encode_i31_get(func, bases, true),
            Self::I31GetU => encode_i31_get(func, bases, false),

            Self::Drop => {
                func.instruction(&Instruction::Drop);
            }

            // Root reads and writes; see `encode_root`.
            Self::LocalGet { local_index } => {
                encode_root(func, bases, Extern, Local(local_index), Get)
            }
            Self::LocalSet { local_index } => {
                encode_root(func, bases, Extern, Local(local_index), Set)
            }
            Self::GlobalGet { global_index } => {
                encode_root(func, bases, Extern, Global(global_index), Get)
            }
            Self::GlobalSet { global_index } => {
                encode_root(func, bases, Extern, Global(global_index), Set)
            }
            Self::TableGet { elem_index } => {
                encode_root(func, bases, Extern, Table(elem_index), Get)
            }
            Self::TableSet { elem_index } => {
                encode_root(func, bases, Extern, Table(elem_index), Set)
            }

            Self::StructLocalGet => encode_root(func, bases, Struct, Local(0), Get),
            Self::StructLocalSet => encode_root(func, bases, Struct, Local(0), Set),
            Self::StructGlobalGet => encode_root(func, bases, Struct, Global(0), Get),
            Self::StructGlobalSet => encode_root(func, bases, Struct, Global(0), Set),
            Self::StructTableGet { elem_index } => {
                encode_root(func, bases, Struct, Table(elem_index), Get)
            }
            Self::StructTableSet { elem_index } => {
                encode_root(func, bases, Struct, Table(elem_index), Set)
            }

            Self::EqLocalGet => encode_root(func, bases, Eq, Local(0), Get),
            Self::EqLocalSet => encode_root(func, bases, Eq, Local(0), Set),
            Self::EqGlobalGet => encode_root(func, bases, Eq, Global(0), Get),
            Self::EqGlobalSet => encode_root(func, bases, Eq, Global(0), Set),
            Self::EqTableGet { elem_index } => encode_root(func, bases, Eq, Table(elem_index), Get),
            Self::EqTableSet { elem_index } => encode_root(func, bases, Eq, Table(elem_index), Set),

            Self::I31LocalGet => encode_root(func, bases, I31, Local(0), Get),
            Self::I31LocalSet => encode_root(func, bases, I31, Local(0), Set),
            Self::I31GlobalGet => encode_root(func, bases, I31, Global(0), Get),
            Self::I31GlobalSet => encode_root(func, bases, I31, Global(0), Set),
            Self::I31TableGet { elem_index } => {
                encode_root(func, bases, I31, Table(elem_index), Get)
            }
            Self::I31TableSet { elem_index } => {
                encode_root(func, bases, I31, Table(elem_index), Set)
            }

            Self::ArrayLocalGet => encode_root(func, bases, Array, Local(0), Get),
            Self::ArrayLocalSet => encode_root(func, bases, Array, Local(0), Set),
            Self::ArrayGlobalGet => encode_root(func, bases, Array, Global(0), Get),
            Self::ArrayGlobalSet => encode_root(func, bases, Array, Global(0), Set),
            Self::ArrayTableGet { elem_index } => {
                encode_root(func, bases, Array, Table(elem_index), Get)
            }
            Self::ArrayTableSet { elem_index } => {
                encode_root(func, bases, Array, Table(elem_index), Set)
            }

            // Typed struct and array ops share the typed banks; only their
            // abstract stack type differs.
            Self::TypedStructLocalGet { type_index } | Self::TypedArrayLocalGet { type_index } => {
                encode_root(func, bases, Typed(type_index), Local(0), Get)
            }
            Self::TypedStructLocalSet { type_index } | Self::TypedArrayLocalSet { type_index } => {
                encode_root(func, bases, Typed(type_index), Local(0), Set)
            }
            Self::TypedStructGlobalGet { type_index }
            | Self::TypedArrayGlobalGet { type_index } => {
                encode_root(func, bases, Typed(type_index), Global(0), Get)
            }
            Self::TypedStructGlobalSet { type_index }
            | Self::TypedArrayGlobalSet { type_index } => {
                encode_root(func, bases, Typed(type_index), Global(0), Set)
            }
            Self::TypedStructTableGet {
                elem_index,
                type_index,
            }
            | Self::TypedArrayTableGet {
                elem_index,
                type_index,
            } => encode_root(func, bases, Typed(type_index), Table(elem_index), Get),
            Self::TypedStructTableSet {
                elem_index,
                type_index,
            }
            | Self::TypedArrayTableSet {
                elem_index,
                type_index,
            } => encode_root(func, bases, Typed(type_index), Table(elem_index), Set),
        };
    }
}

/// A root read or write: one instruction, or the `table.set` idiom.
fn encode_root(
    func: &mut Function,
    bases: WasmEncodingBases,
    kind: RefKind,
    storage: Storage,
    dir: Dir,
) {
    let slots = bases.root_slots(kind, storage.index());
    match (storage, dir) {
        (Storage::Local(_), Dir::Get) => {
            func.instruction(&Instruction::LocalGet(slots.local));
        }
        (Storage::Local(_), Dir::Set) => {
            func.instruction(&Instruction::LocalSet(slots.local));
        }
        (Storage::Global(_), Dir::Get) => {
            func.instruction(&Instruction::GlobalGet(slots.global));
        }
        (Storage::Global(_), Dir::Set) => {
            func.instruction(&Instruction::GlobalSet(slots.global));
        }
        (Storage::Table(elem), Dir::Get) => table_get(func, elem, slots.table),
        (Storage::Table(elem), Dir::Set) => {
            return table_set_via(func, slots.tmp, elem, slots.table);
        }
    };
}

/// Differential check: pass the i31ref plus the guest's inline `i31.get_s` /
/// `i31.get_u` results to the host, which re-derives and compares them.
fn encode_take_i31(func: &mut Function, bases: WasmEncodingBases) {
    let i31 = bases.locals.i31ref;
    func.instruction(&Instruction::LocalTee(i31));
    func.instruction(&Instruction::RefIsNull);
    func.instruction(&Instruction::If(BlockType::Empty));
    // Null branch: `take_i31(null, 0, 0)`.
    func.instruction(&Instruction::RefNull(HeapType::I31));
    func.instruction(&Instruction::I32Const(0));
    func.instruction(&Instruction::I32Const(0));
    func.instruction(&Instruction::Call(bases.funcs.take_i31));
    func.instruction(&Instruction::Else);
    // Non-null branch: `take_i31(ref, i31.get_s, i31.get_u)`.
    func.instruction(&Instruction::LocalGet(i31));
    func.instruction(&Instruction::LocalGet(i31));
    func.instruction(&Instruction::I31GetS);
    func.instruction(&Instruction::LocalGet(i31));
    func.instruction(&Instruction::I31GetU);
    func.instruction(&Instruction::Call(bases.funcs.take_i31));
    func.instruction(&Instruction::End);
}

/// `struct.new` of default field values, or `struct.new_default` when asked and
/// every field is defaultable (a non-nullable reference is not).
fn encode_struct_new(func: &mut Function, cx: EmitCtx<'_>, type_index: u32, prefer_default: bool) {
    let fields = struct_fields(cx.types, cx.encoding_order, type_index).unwrap_or(&[]);
    let wasm_type = cx.bases.wasm_type(type_index);
    if prefer_default && fields.iter().all(|f| f.field_type.is_defaultable()) {
        func.instruction(&Instruction::StructNewDefault(wasm_type));
    } else {
        for field in fields {
            field.field_type.emit_default_const(func, cx);
        }
        func.instruction(&Instruction::StructNew(wasm_type));
    }
}

/// `array.new` of `array_length` default elements.
fn encode_array_new(func: &mut Function, cx: EmitCtx<'_>, type_index: u32) {
    if let Some(element) = array_element(cx.types, cx.encoding_order, type_index) {
        element.field_type.emit_default_const(func, cx);
    }
    func.instruction(&Instruction::I32Const(cx.bases.array_length.cast_signed()));
    func.instruction(&Instruction::ArrayNew(cx.bases.wasm_type(type_index)));
}

/// `array.new_default` of `array_length` elements, or `array.new` when the
/// element is not defaultable.
fn encode_array_new_default(func: &mut Function, cx: EmitCtx<'_>, type_index: u32) {
    let wasm_type = cx.bases.wasm_type(type_index);
    let len = cx.bases.array_length.cast_signed();
    match array_element(cx.types, cx.encoding_order, type_index) {
        Some(element) if !element.field_type.is_defaultable() => {
            element.field_type.emit_default_const(func, cx);
            func.instruction(&Instruction::I32Const(len));
            func.instruction(&Instruction::ArrayNew(wasm_type));
        }
        _ => {
            func.instruction(&Instruction::I32Const(len));
            func.instruction(&Instruction::ArrayNewDefault(wasm_type));
        }
    }
}

/// `array.new_fixed` of `n` default elements.
fn encode_array_new_fixed(func: &mut Function, cx: EmitCtx<'_>, type_index: u32, n: u32) {
    if let Some(element) = array_element(cx.types, cx.encoding_order, type_index) {
        for _ in 0..n {
            element.field_type.emit_default_const(func, cx);
        }
    }
    func.instruction(&Instruction::ArrayNewFixed {
        array_type_index: cx.bases.wasm_type(type_index),
        array_size: n,
    });
}

/// The value on the stack is already the subtype, so this cast always succeeds.
fn encode_upcast(func: &mut Function, bases: WasmEncodingBases, super_type_index: u32) {
    let heap_type = HeapType::Concrete(bases.wasm_type(super_type_index));
    func.instruction(&Instruction::RefCastNullable(heap_type));
}

/// A downcast that never traps: `ref.test` first, and `ref.null` when it fails.
fn encode_downcast(
    func: &mut Function,
    bases: WasmEncodingBases,
    sub_type_index: u32,
    super_type_index: u32,
) {
    let sub_wasm_type = bases.wasm_type(sub_type_index);
    let sub_heap_type = HeapType::Concrete(sub_wasm_type);
    let temp_local = bases.locals.typed(super_type_index);

    func.instruction(&Instruction::LocalTee(temp_local));
    func.instruction(&Instruction::RefTestNullable(sub_heap_type));
    func.instruction(&Instruction::If(BlockType::Result(ValType::Ref(concrete(
        sub_wasm_type,
    )))));
    func.instruction(&Instruction::LocalGet(temp_local));
    func.instruction(&Instruction::RefCastNullable(sub_heap_type));
    func.instruction(&Instruction::Else);
    func.instruction(&Instruction::RefNull(sub_heap_type));
    func.instruction(&Instruction::End);
}

/// Null-guarded `struct.get` of field `field_index % len`; the value is dropped.
fn encode_struct_get(
    func: &mut Function,
    cx: EmitCtx<'_>,
    type_index: u32,
    field_index: u32,
    unsigned: bool,
) {
    let wasm_type = cx.bases.wasm_type(type_index);
    match struct_fields(cx.types, cx.encoding_order, type_index) {
        Some(fields) if !fields.is_empty() => {
            let typed_local = cx.bases.locals.typed(type_index);
            let idx = field_index % u32::try_from(fields.len()).unwrap();
            let field_type = fields[usize::try_from(idx).unwrap()].field_type;
            let get = struct_get_instruction(wasm_type, idx, field_type, unsigned);
            if_non_null(func, typed_local, |func| {
                func.instruction(&Instruction::LocalGet(typed_local));
                func.instruction(&get);
                // Field values are not tracked on the abstract stack.
                func.instruction(&Instruction::Drop);
            });
        }
        _ => {
            func.instruction(&Instruction::Drop);
        }
    }
}

/// Null-guarded `struct.set` of a default value into the first mutable field at
/// or after `field_index`, wrapping around; no mutable field just drops the ref.
fn encode_struct_set(func: &mut Function, cx: EmitCtx<'_>, type_index: u32, field_index: u32) {
    let wasm_type = cx.bases.wasm_type(type_index);
    let mutable_field = struct_fields(cx.types, cx.encoding_order, type_index)
        .filter(|fields| !fields.is_empty())
        .and_then(|fields| {
            let len = fields.len();
            let start = usize::try_from(field_index).unwrap() % len;
            (0..len)
                .map(|offset| (start + offset) % len)
                .find(|&i| fields[i].mutable)
                .map(|i| (u32::try_from(i).unwrap(), fields[i].field_type))
        });

    match mutable_field {
        Some((idx, field_type)) => {
            let typed_local = cx.bases.locals.typed(type_index);
            if_non_null(func, typed_local, |func| {
                func.instruction(&Instruction::LocalGet(typed_local));
                field_type.emit_default_const(func, cx);
                func.instruction(&Instruction::StructSet {
                    struct_type_index: wasm_type,
                    field_index: idx,
                });
            });
        }
        None => {
            func.instruction(&Instruction::Drop);
        }
    }
}

/// Null-guarded `array.get`; the element is dropped. Fixup keeps `index` mostly in bounds.
fn encode_array_get(
    func: &mut Function,
    cx: EmitCtx<'_>,
    type_index: u32,
    index: u32,
    unsigned: bool,
) {
    let wasm_type = cx.bases.wasm_type(type_index);
    let typed_local = cx.bases.locals.typed(type_index);
    match array_element(cx.types, cx.encoding_order, type_index) {
        Some(element) => {
            let get = array_get_instruction(wasm_type, element.field_type, unsigned);
            if_non_null(func, typed_local, |func| {
                func.instruction(&Instruction::LocalGet(typed_local));
                func.instruction(&Instruction::I32Const(index.cast_signed()));
                func.instruction(&get);
                // The element value is not tracked on the abstract stack.
                func.instruction(&Instruction::Drop);
            });
        }
        None => {
            func.instruction(&Instruction::Drop);
        }
    }
}

/// Null-guarded `array.set` of a default value; an immutable element drops the ref.
fn encode_array_set(func: &mut Function, cx: EmitCtx<'_>, type_index: u32, index: u32) {
    let wasm_type = cx.bases.wasm_type(type_index);
    let typed_local = cx.bases.locals.typed(type_index);
    match array_element(cx.types, cx.encoding_order, type_index) {
        Some(element) if element.mutable => {
            if_non_null(func, typed_local, |func| {
                func.instruction(&Instruction::LocalGet(typed_local));
                func.instruction(&Instruction::I32Const(index.cast_signed()));
                element.field_type.emit_default_const(func, cx);
                func.instruction(&Instruction::ArraySet(wasm_type));
            });
        }
        _ => {
            func.instruction(&Instruction::Drop);
        }
    }
}

/// Null-guarded `array.fill` with a default value; an immutable element drops the ref.
fn encode_array_fill(func: &mut Function, cx: EmitCtx<'_>, type_index: u32, offset: u32, len: u32) {
    let wasm_type = cx.bases.wasm_type(type_index);
    let typed_local = cx.bases.locals.typed(type_index);
    match array_element(cx.types, cx.encoding_order, type_index) {
        Some(element) if element.mutable => {
            if_non_null(func, typed_local, |func| {
                func.instruction(&Instruction::LocalGet(typed_local));
                func.instruction(&Instruction::I32Const(offset.cast_signed()));
                element.field_type.emit_default_const(func, cx);
                func.instruction(&Instruction::I32Const(len.cast_signed()));
                func.instruction(&Instruction::ArrayFill(wasm_type));
            });
        }
        _ => {
            func.instruction(&Instruction::Drop);
        }
    }
}

/// `array.copy` between the two arrays on the stack, skipped if either is null.
/// The source is parked in the second typed bank while the destination is checked.
fn encode_array_copy(
    func: &mut Function,
    cx: EmitCtx<'_>,
    type_index: u32,
    dst_offset: u32,
    src_offset: u32,
    len: u32,
) {
    let wasm_type = cx.bases.wasm_type(type_index);
    let dst_local = cx.bases.locals.typed(type_index);
    let src_local = cx.bases.locals.typed2(type_index);
    match array_element(cx.types, cx.encoding_order, type_index) {
        Some(element) if element.mutable => {
            func.instruction(&Instruction::LocalSet(src_local));
            func.instruction(&Instruction::LocalSet(dst_local));
            func.instruction(&Instruction::LocalGet(dst_local));
            func.instruction(&Instruction::RefIsNull);
            func.instruction(&Instruction::LocalGet(src_local));
            func.instruction(&Instruction::RefIsNull);
            func.instruction(&Instruction::I32Or);
            func.instruction(&Instruction::If(BlockType::Empty));
            func.instruction(&Instruction::Else);
            func.instruction(&Instruction::LocalGet(dst_local));
            func.instruction(&Instruction::I32Const(dst_offset.cast_signed()));
            func.instruction(&Instruction::LocalGet(src_local));
            func.instruction(&Instruction::I32Const(src_offset.cast_signed()));
            func.instruction(&Instruction::I32Const(len.cast_signed()));
            func.instruction(&Instruction::ArrayCopy {
                array_type_index_dst: wasm_type,
                array_type_index_src: wasm_type,
            });
            func.instruction(&Instruction::End);
        }
        _ => {
            func.instruction(&Instruction::Drop);
            func.instruction(&Instruction::Drop);
        }
    }
}

/// Null-guarded `array.len`; the length is dropped.
fn encode_array_len(func: &mut Function, bases: WasmEncodingBases) {
    let array_local = bases.locals.arrayref;
    if_non_null(func, array_local, |func| {
        func.instruction(&Instruction::LocalGet(array_local));
        func.instruction(&Instruction::ArrayLen);
        func.instruction(&Instruction::Drop);
    });
}

/// Null-guarded `i31.get_s` / `i31.get_u`; the value is dropped.
fn encode_i31_get(func: &mut Function, bases: WasmEncodingBases, signed: bool) {
    let i31_local = bases.locals.i31ref;
    let get = if signed {
        Instruction::I31GetS
    } else {
        Instruction::I31GetU
    };
    if_non_null(func, i31_local, |func| {
        func.instruction(&Instruction::LocalGet(i31_local));
        func.instruction(&get);
        func.instruction(&Instruction::Drop);
    });
}
