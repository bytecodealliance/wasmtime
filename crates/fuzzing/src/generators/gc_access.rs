//! Generate Wasm modules that exercise GC object creation, field access, and
//! assertion patterns across different heap initialization states.

use arbitrary::{Arbitrary, Result, Unstructured};
use std::borrow::Cow;
use wasm_encoder::{
    CodeSection, ConstExpr, DataSection, ElementSection, Elements, Encode, EntityType, ExportKind,
    ExportSection, Function, FunctionSection, GlobalSection, GlobalType, ImportSection,
    Instruction, Module, RefType, StartSection, TableSection, TableType, TypeSection, ValType,
};

/// The storage type for a GC object's field or element.
#[derive(Arbitrary, Debug, Clone, Copy)]
enum ElemType {
    I8,
    I32,
    Anyref,
    I31ref,
}

impl ElemType {
    fn is_packed(self) -> bool {
        matches!(self, ElemType::I8)
    }

    fn is_numeric(self) -> bool {
        matches!(self, ElemType::I8 | ElemType::I32)
    }

    fn is_ref(self) -> bool {
        matches!(self, ElemType::Anyref | ElemType::I31ref)
    }

    fn storage_type(self) -> wasm_encoder::StorageType {
        match self {
            ElemType::I8 => wasm_encoder::StorageType::I8,
            ElemType::I32 => wasm_encoder::StorageType::Val(ValType::I32),
            ElemType::Anyref | ElemType::I31ref => {
                wasm_encoder::StorageType::Val(ValType::Ref(RefType::ANYREF))
            }
        }
    }
}

#[derive(Arbitrary, Debug, Clone, Copy)]
enum ObjectKind {
    Array,
    Struct,
}

#[derive(Arbitrary, Debug, Clone, Copy)]
struct ObjectType {
    kind: ObjectKind,
    elem_type: ElemType,
    mutable: bool,
}

impl ObjectType {
    fn is_array(self) -> bool {
        matches!(self.kind, ObjectKind::Array)
    }
}

/// How the GC heap is initialized before the test's `run` function executes.
#[derive(Arbitrary, Debug, Clone, Copy)]
enum HeapInit {
    /// The heap is left empty; no filler objects are allocated.
    Empty,
    /// The heap is filled to capacity with filler objects.
    Full,
    /// The heap is nearly full, leaving only a small amount of free space.
    AlmostFull,
    /// The heap is filled with two interleaved chains of filler objects, then
    /// one chain is dropped and a GC is triggered, producing a fragmented heap.
    Fragmented,
}

/// How the GC object under test is created in the `run` function.
#[derive(Debug, Clone, Copy)]
enum CreateMode {
    /// Retrieve the object from an immutable global.
    GlobalGet,
    /// Retrieve the object from a table slot.
    TableGet,
    /// Create the object by calling a helper function.
    Call,
    /// Receive the object as a function parameter via `local.get`.
    LocalGet,
    /// Create an array with `array.new` (fill with a single value).
    ArrayNew,
    /// Create an array with `array.new_default` (zero/null-initialized).
    ArrayNewDefault,
    /// Create an array with `array.new_fixed` (each element pushed individually).
    ArrayNewFixed,
    /// Create an array with `array.new_data` from a passive data segment.
    ArrayNewData,
    /// Create an array with `array.new_elem` from a passive element segment.
    ArrayNewElem,
    /// Create a struct with `struct.new`.
    StructNew,
    /// Create a struct with `struct.new_default` (zero/null-initialized).
    StructNewDefault,
}

impl CreateMode {
    fn arbitrary(u: &mut Unstructured<'_>, object_type: &ObjectType) -> Result<Self> {
        let mut modes: Vec<CreateMode> = vec![
            CreateMode::GlobalGet,
            CreateMode::TableGet,
            CreateMode::Call,
            CreateMode::LocalGet,
        ];
        if object_type.is_array() {
            modes.push(CreateMode::ArrayNew);
            modes.push(CreateMode::ArrayNewDefault);
            modes.push(CreateMode::ArrayNewFixed);
            if object_type.elem_type.is_ref() {
                modes.push(CreateMode::ArrayNewElem);
            } else {
                modes.push(CreateMode::ArrayNewData);
            }
        } else {
            modes.push(CreateMode::StructNew);
            modes.push(CreateMode::StructNewDefault);
        }
        Ok(modes[u.int_in_range(0..=modes.len() - 1)?])
    }

    fn is_default(self) -> bool {
        matches!(
            self,
            CreateMode::StructNewDefault | CreateMode::ArrayNewDefault
        )
    }

    fn is_idempotent(self) -> bool {
        matches!(
            self,
            CreateMode::GlobalGet | CreateMode::TableGet | CreateMode::LocalGet
        )
    }
}

/// The access operation performed on the GC object, exercised before and after
/// a GC cycle to verify that the object survives collection correctly.
#[derive(Debug, Clone, Copy)]
enum AccessMode {
    /// Test the object's runtime type with `ref.test`.
    RefTest,
    /// Read the struct field with `struct.get`.
    StructGet,
    /// Read a packed struct field with sign extension via `struct.get_s`.
    StructGetS,
    /// Read a packed struct field with zero extension via `struct.get_u`.
    StructGetU,
    /// Write the struct field with `struct.set`.
    StructSet,
    /// Read an array element with `array.get`.
    ArrayGet,
    /// Read a packed array element with sign extension via `array.get_s`.
    ArrayGetS,
    /// Read a packed array element with zero extension via `array.get_u`.
    ArrayGetU,
    /// Write an array element with `array.set`.
    ArraySet,
    /// Query the array length with `array.len`.
    ArrayLen,
    /// Fill the entire array with a value via `array.fill`.
    ArrayFill,
    /// Copy elements from a source array via `array.copy`.
    ArrayCopy,
    /// Initialize array elements from a passive data segment via `array.init_data`.
    ArrayInitData,
    /// Initialize array elements from a passive element segment via `array.init_elem`.
    ArrayInitElem,
}

impl AccessMode {
    fn arbitrary(u: &mut Unstructured<'_>, object_type: &ObjectType) -> Result<Self> {
        let mut modes: Vec<AccessMode> = vec![AccessMode::RefTest];
        if object_type.is_array() {
            if object_type.elem_type.is_packed() {
                modes.push(AccessMode::ArrayGetS);
                modes.push(AccessMode::ArrayGetU);
            } else {
                modes.push(AccessMode::ArrayGet);
            }
            modes.push(AccessMode::ArrayLen);
            if object_type.mutable {
                modes.push(AccessMode::ArraySet);
                modes.push(AccessMode::ArrayFill);
                modes.push(AccessMode::ArrayCopy);
                if object_type.elem_type.is_numeric() {
                    modes.push(AccessMode::ArrayInitData);
                } else if object_type.elem_type.is_ref() {
                    modes.push(AccessMode::ArrayInitElem);
                }
            }
        } else {
            if object_type.elem_type.is_packed() {
                modes.push(AccessMode::StructGetS);
                modes.push(AccessMode::StructGetU);
            } else {
                modes.push(AccessMode::StructGet);
            }
            if object_type.mutable {
                modes.push(AccessMode::StructSet);
            }
        }
        Ok(modes[u.int_in_range(0..=modes.len() - 1)?])
    }
}

/// A fuzz test case that generates a Wasm module exercising GC object creation,
/// access, and assertion patterns across different heap initialization states.
#[derive(Debug)]
pub struct GcAccess {
    object_type: ObjectType,
    heap_init: HeapInit,
    create_mode: CreateMode,
    access_mode: AccessMode,
    array_len: u32,
    creation_value: i32,
    mutation_value: i32,
}

impl<'a> Arbitrary<'a> for GcAccess {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        let object_type: ObjectType = u.arbitrary()?;
        let heap_init: HeapInit = u.arbitrary()?;
        let create_mode = CreateMode::arbitrary(u, &object_type)?;
        let access_mode = AccessMode::arbitrary(u, &object_type)?;
        let array_len = u.int_in_range(1..=1024)?;
        let creation_value: i32 = u.arbitrary()?;
        let mutation_value: i32 = u.arbitrary()?;
        Ok(GcAccess {
            object_type,
            heap_init,
            create_mode,
            access_mode,
            array_len,
            creation_value,
            mutation_value,
        })
    }
}

impl GcAccess {
    /// Generate the Wasm binary for this test case.
    ///
    /// The generated module imports `"wasmtime" "gc"` as a GC-triggering
    /// function and exports a `"run"` function. The `"run"` function creates a
    /// GC object, accesses it, triggers GC, then asserts the access result is
    /// unchanged.
    pub fn to_wasm(&self) -> Vec<u8> {
        let mut emitter = Emitter::new(self);
        emitter.emit()
    }
}

struct Emitter<'a> {
    cfg: &'a GcAccess,

    // Type indices.
    obj_type: Option<u32>,
    filler_type: Option<u32>,
    nested_obj_type: Option<u32>,
    void_fn_type: Option<u32>,
    make_obj_fn_type: Option<u32>,
    run_impl_fn_type: Option<u32>,

    // Function indices.
    next_func_idx: u32,
    gc_func: Option<u32>,
    make_obj_func: Option<u32>,
    init_func: Option<u32>,
    run_impl_func: Option<u32>,
    run_func: Option<u32>,

    // Global indices.
    global_g: Option<u32>,
    global_root: Option<u32>,
    global_root2: Option<u32>,

    // Table index.
    table_t: Option<u32>,

    // Segment indices.
    data_d: Option<u32>,
    data_dinit: Option<u32>,
    elem_e: Option<u32>,
    elem_einit: Option<u32>,
}

impl<'a> Emitter<'a> {
    fn new(cfg: &'a GcAccess) -> Self {
        Emitter {
            cfg,
            obj_type: None,
            filler_type: None,
            nested_obj_type: None,
            void_fn_type: None,
            make_obj_fn_type: None,
            run_impl_fn_type: None,
            next_func_idx: 0,
            gc_func: None,
            make_obj_func: None,
            init_func: None,
            run_impl_func: None,
            run_func: None,
            global_g: None,
            global_root: None,
            global_root2: None,
            table_t: None,
            data_d: None,
            data_dinit: None,
            elem_e: None,
            elem_einit: None,
        }
    }

    fn obj_ref_type(&self) -> ValType {
        ValType::Ref(RefType {
            nullable: true,
            heap_type: wasm_encoder::HeapType::Concrete(self.obj_type.unwrap()),
        })
    }

    fn obj_ref_type_non_null(&self) -> ValType {
        ValType::Ref(RefType {
            nullable: false,
            heap_type: wasm_encoder::HeapType::Concrete(self.obj_type.unwrap()),
        })
    }

    fn emit(&mut self) -> Vec<u8> {
        let mut module = Module::new();

        let types = self.types();
        sec(&mut module, types);
        let imports = self.imports();
        sec(&mut module, imports);
        let functions = self.functions();
        sec(&mut module, functions);
        if let Some(s) = self.tables() {
            sec(&mut module, s);
        }
        if let Some(s) = self.globals() {
            sec(&mut module, s);
        }
        let exports = self.exports();
        sec(&mut module, exports);
        sec(
            &mut module,
            StartSection {
                function_index: self.init_func.unwrap(),
            },
        );
        if let Some(s) = self.elems() {
            sec(&mut module, s);
        }
        let data = self.data();
        if data.is_some() {
            let count = self.data_d.is_some() as u32 + self.data_dinit.is_some() as u32;
            sec(&mut module, wasm_encoder::DataCountSection { count });
        }
        let code = self.code();
        sec(&mut module, code);
        if let Some(s) = data {
            sec(&mut module, s);
        }

        module.finish()
    }

    fn types(&mut self) -> TypeSection {
        let mut types = TypeSection::new();

        self.obj_type = Some(types.len());
        let field = wasm_encoder::FieldType {
            element_type: self.cfg.object_type.elem_type.storage_type(),
            mutable: self.cfg.object_type.mutable,
        };
        let obj_composite = if self.cfg.object_type.is_array() {
            wasm_encoder::CompositeInnerType::Array(wasm_encoder::ArrayType(field))
        } else {
            wasm_encoder::CompositeInnerType::Struct(wasm_encoder::StructType {
                fields: Box::new([field]),
            })
        };
        types.ty().subtype(&wasm_encoder::SubType {
            is_final: true,
            supertype_idx: None,
            composite_type: wasm_encoder::CompositeType {
                inner: obj_composite,
                shared: false,
                describes: None,
                descriptor: None,
            },
        });

        if !matches!(self.cfg.heap_init, HeapInit::Empty) {
            self.filler_type = Some(types.len());
            let fi = self.filler_type.unwrap();
            types.ty().subtype(&wasm_encoder::SubType {
                is_final: true,
                supertype_idx: None,
                composite_type: wasm_encoder::CompositeType {
                    inner: wasm_encoder::CompositeInnerType::Struct(wasm_encoder::StructType {
                        fields: Box::new([wasm_encoder::FieldType {
                            element_type: wasm_encoder::StorageType::Val(ValType::Ref(RefType {
                                nullable: true,
                                heap_type: wasm_encoder::HeapType::Concrete(fi),
                            })),
                            mutable: true,
                        }]),
                    }),
                    shared: false,
                    describes: None,
                    descriptor: None,
                },
            });
        }

        if self.cfg.object_type.elem_type.is_ref()
            && !self.cfg.create_mode.is_default()
            && matches!(self.cfg.object_type.elem_type, ElemType::Anyref)
        {
            self.nested_obj_type = Some(types.len());
            types.ty().subtype(&wasm_encoder::SubType {
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
            });
        }

        self.void_fn_type = Some(types.len());
        types.ty().function(vec![], vec![]);

        if matches!(self.cfg.create_mode, CreateMode::Call) {
            self.make_obj_fn_type = Some(types.len());
            types
                .ty()
                .function(vec![], vec![self.obj_ref_type_non_null()]);
        }

        if matches!(self.cfg.create_mode, CreateMode::LocalGet) {
            self.run_impl_fn_type = Some(types.len());
            types.ty().function(vec![self.obj_ref_type()], vec![]);
        }

        types
    }

    fn imports(&mut self) -> ImportSection {
        let mut imports = ImportSection::new();
        self.gc_func = Some(self.next_func_idx);
        self.next_func_idx += 1;
        imports.import(
            "wasmtime",
            "gc",
            EntityType::Function(self.void_fn_type.unwrap()),
        );
        imports
    }

    fn functions(&mut self) -> FunctionSection {
        let mut fns = FunctionSection::new();
        if let Some(t) = self.make_obj_fn_type {
            self.make_obj_func = Some(self.next_func_idx);
            self.next_func_idx += 1;
            fns.function(t);
        }
        self.init_func = Some(self.next_func_idx);
        self.next_func_idx += 1;
        fns.function(self.void_fn_type.unwrap());
        if let Some(t) = self.run_impl_fn_type {
            self.run_impl_func = Some(self.next_func_idx);
            self.next_func_idx += 1;
            fns.function(t);
        }
        self.run_func = Some(self.next_func_idx);
        self.next_func_idx += 1;
        fns.function(self.void_fn_type.unwrap());
        fns
    }

    fn tables(&mut self) -> Option<TableSection> {
        if !matches!(self.cfg.create_mode, CreateMode::TableGet) {
            return None;
        }
        let mut tables = TableSection::new();
        self.table_t = Some(tables.len());
        tables.table_with_init(
            TableType {
                element_type: RefType {
                    nullable: false,
                    heap_type: wasm_encoder::HeapType::Concrete(self.obj_type.unwrap()),
                },
                minimum: 1,
                maximum: Some(1),
                table64: false,
                shared: false,
            },
            &self.obj_const_expr(self.cfg.creation_value),
        );
        Some(tables)
    }

    fn globals(&mut self) -> Option<GlobalSection> {
        let mut globals = GlobalSection::new();
        let mut any = false;

        if matches!(self.cfg.create_mode, CreateMode::GlobalGet) {
            self.global_g = Some(globals.len());
            any = true;
            globals.global(
                GlobalType {
                    val_type: self.obj_ref_type_non_null(),
                    mutable: false,
                    shared: false,
                },
                &self.obj_const_expr(self.cfg.creation_value),
            );
        }

        if let Some(fi) = self.filler_type {
            if !matches!(self.cfg.heap_init, HeapInit::Empty) {
                self.global_root = Some(globals.len());
                any = true;
                globals.global(
                    GlobalType {
                        val_type: ValType::Ref(RefType {
                            nullable: true,
                            heap_type: wasm_encoder::HeapType::Concrete(fi),
                        }),
                        mutable: true,
                        shared: false,
                    },
                    &ConstExpr::ref_null(wasm_encoder::HeapType::Concrete(fi)),
                );
            }
            if matches!(self.cfg.heap_init, HeapInit::Fragmented) {
                self.global_root2 = Some(globals.len());
                any = true;
                globals.global(
                    GlobalType {
                        val_type: ValType::Ref(RefType {
                            nullable: true,
                            heap_type: wasm_encoder::HeapType::Concrete(fi),
                        }),
                        mutable: true,
                        shared: false,
                    },
                    &ConstExpr::ref_null(wasm_encoder::HeapType::Concrete(fi)),
                );
            }
        }

        if any { Some(globals) } else { None }
    }

    fn exports(&self) -> ExportSection {
        let mut exports = ExportSection::new();
        exports.export("run", ExportKind::Func, self.run_func.unwrap());
        exports
    }

    fn elems(&mut self) -> Option<ElementSection> {
        let mut elems = ElementSection::new();
        let mut any = false;

        if matches!(self.cfg.create_mode, CreateMode::ArrayNewElem) {
            self.elem_e = Some(elems.len());
            any = true;
            let exprs: Vec<ConstExpr> = (0..self.cfg.array_len)
                .map(|_| self.ref_value_const_expr(self.cfg.creation_value))
                .collect();
            elems.segment(wasm_encoder::ElementSegment {
                mode: wasm_encoder::ElementMode::Passive,
                elements: Elements::Expressions(RefType::ANYREF, Cow::Owned(exprs)),
            });
        }

        if matches!(self.cfg.access_mode, AccessMode::ArrayInitElem) {
            self.elem_einit = Some(elems.len());
            any = true;
            let expr = self.ref_value_const_expr(self.cfg.mutation_value);
            elems.segment(wasm_encoder::ElementSegment {
                mode: wasm_encoder::ElementMode::Passive,
                elements: Elements::Expressions(RefType::ANYREF, Cow::Owned(vec![expr])),
            });
        }

        if any { Some(elems) } else { None }
    }

    fn data(&mut self) -> Option<DataSection> {
        let mut data = DataSection::new();
        let mut any = false;

        if matches!(self.cfg.create_mode, CreateMode::ArrayNewData) {
            self.data_d = Some(data.len());
            any = true;
            data.passive(self.data_bytes(self.cfg.creation_value, self.cfg.array_len as usize));
        }
        if matches!(self.cfg.access_mode, AccessMode::ArrayInitData) {
            self.data_dinit = Some(data.len());
            any = true;
            data.passive(self.data_bytes(self.cfg.mutation_value, 1));
        }

        if any { Some(data) } else { None }
    }

    fn data_bytes(&self, val: i32, count: usize) -> Vec<u8> {
        match self.cfg.object_type.elem_type {
            ElemType::I32 => (val as u32)
                .to_le_bytes()
                .iter()
                .copied()
                .cycle()
                .take(4 * count)
                .collect(),
            ElemType::I8 => vec![(val & 0xFF) as u8; count],
            _ => unreachable!(),
        }
    }

    fn code(&self) -> CodeSection {
        let mut code = CodeSection::new();

        if self.make_obj_func.is_some() {
            let mut body = Function::new(vec![]);
            self.emit_make_obj(&mut body);
            finish(&mut code, body);
        }

        // init
        {
            let locals = if !matches!(self.cfg.heap_init, HeapInit::Empty) {
                vec![(1, ValType::I32)]
            } else {
                vec![]
            };
            let mut body = Function::new(locals);
            self.emit_init(&mut body);
            finish(&mut code, body);
        }

        // run_impl (local_get mode)
        if self.run_impl_func.is_some() {
            let mut locals = vec![
                (1, ValType::I32), // $i
                (1, ValType::I32), // $j
            ];
            if self.needs_before() {
                locals.push((1, ValType::I32)); // $before
            }
            if matches!(self.cfg.access_mode, AccessMode::ArrayCopy) {
                locals.push((1, self.obj_ref_type())); // $src
            }
            let mut body = Function::new(locals);
            self.emit_local_get_body(&mut body);
            finish(&mut code, body);
        }

        // run
        {
            if self.run_impl_func.is_some() {
                let mut body = Function::new(vec![]);
                self.emit_make_obj(&mut body);
                body.instruction(&Instruction::Call(self.run_impl_func.unwrap()));
                finish(&mut code, body);
            } else if self.cfg.create_mode.is_idempotent()
                && !matches!(self.cfg.create_mode, CreateMode::LocalGet)
            {
                let mut locals = vec![
                    (1, ValType::I32), // $i
                    (1, ValType::I32), // $j
                ];
                if self.needs_before() {
                    locals.push((1, ValType::I32));
                }
                if matches!(self.cfg.access_mode, AccessMode::ArrayCopy) {
                    locals.push((1, self.obj_ref_type()));
                }
                let mut body = Function::new(locals);
                self.emit_idempotent_body(&mut body);
                finish(&mut code, body);
            } else {
                let mut locals = vec![
                    (1, self.obj_ref_type()), // $x
                    (1, ValType::I32),        // $i
                    (1, ValType::I32),        // $j
                ];
                if self.needs_before() {
                    locals.push((1, ValType::I32));
                }
                if matches!(self.cfg.access_mode, AccessMode::ArrayCopy) {
                    locals.push((1, self.obj_ref_type()));
                }
                let mut body = Function::new(locals);
                self.emit_normal_body(&mut body);
                finish(&mut code, body);
            }
        }

        code
    }

    fn needs_before(&self) -> bool {
        match self.cfg.access_mode {
            AccessMode::RefTest
            | AccessMode::StructGetS
            | AccessMode::StructGetU
            | AccessMode::ArrayGetS
            | AccessMode::ArrayGetU
            | AccessMode::ArrayLen => true,
            AccessMode::StructGet | AccessMode::ArrayGet => {
                if self.cfg.object_type.elem_type.is_ref() {
                    // anyref get with default creates drops the value; nested obj
                    // asserts non-null; neither uses $before as i32
                    !self.cfg.create_mode.is_default() && !self.uses_nested_obj()
                } else {
                    true
                }
            }
            _ => false,
        }
    }

    fn uses_nested_obj(&self) -> bool {
        self.nested_obj_type.is_some()
    }

    fn emit_init(&self, body: &mut Function) {
        let local_i = 0u32;
        match self.cfg.heap_init {
            HeapInit::Empty => {}
            HeapInit::Full => self.emit_filler_loop(body, local_i, 4096),
            HeapInit::AlmostFull => self.emit_filler_loop(body, local_i, 4094),
            HeapInit::Fragmented => self.emit_fragmented(body, local_i),
        }
    }

    fn emit_filler_loop(&self, body: &mut Function, local_i: u32, count: i32) {
        let fi = self.filler_type.unwrap();
        let root = self.global_root.unwrap();

        body.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        body.instruction(&Instruction::GlobalGet(root));
        body.instruction(&Instruction::StructNew(fi));
        body.instruction(&Instruction::GlobalSet(root));
        body.instruction(&Instruction::LocalGet(local_i));
        body.instruction(&Instruction::I32Const(1));
        body.instruction(&Instruction::I32Add);
        body.instruction(&Instruction::LocalSet(local_i));
        body.instruction(&Instruction::LocalGet(local_i));
        body.instruction(&Instruction::I32Const(count));
        body.instruction(&Instruction::I32LtU);
        body.instruction(&Instruction::BrIf(0));
        body.instruction(&Instruction::End);
    }

    fn emit_fragmented(&self, body: &mut Function, local_i: u32) {
        let fi = self.filler_type.unwrap();
        let root = self.global_root.unwrap();
        let root2 = self.global_root2.unwrap();

        body.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        body.instruction(&Instruction::GlobalGet(root));
        body.instruction(&Instruction::StructNew(fi));
        body.instruction(&Instruction::GlobalSet(root));
        body.instruction(&Instruction::GlobalGet(root2));
        body.instruction(&Instruction::StructNew(fi));
        body.instruction(&Instruction::GlobalSet(root2));
        body.instruction(&Instruction::LocalGet(local_i));
        body.instruction(&Instruction::I32Const(1));
        body.instruction(&Instruction::I32Add);
        body.instruction(&Instruction::LocalSet(local_i));
        body.instruction(&Instruction::LocalGet(local_i));
        body.instruction(&Instruction::I32Const(2048));
        body.instruction(&Instruction::I32LtU);
        body.instruction(&Instruction::BrIf(0));
        body.instruction(&Instruction::End);

        body.instruction(&Instruction::RefNull(wasm_encoder::HeapType::Concrete(fi)));
        body.instruction(&Instruction::GlobalSet(root2));
        body.instruction(&Instruction::Call(self.gc_func.unwrap()));
    }

    fn emit_make_obj(&self, body: &mut Function) {
        let obj_type = self.obj_type.unwrap();
        if self.cfg.object_type.is_array() {
            self.emit_value(body, self.cfg.creation_value);
            body.instruction(&Instruction::I32Const(self.cfg.array_len as i32));
            body.instruction(&Instruction::ArrayNew(obj_type));
        } else {
            self.emit_value(body, self.cfg.creation_value);
            body.instruction(&Instruction::StructNew(obj_type));
        }
    }

    fn emit_create(&self, body: &mut Function) {
        let obj_type = self.obj_type.unwrap();
        match self.cfg.create_mode {
            CreateMode::StructNew => {
                self.emit_value(body, self.cfg.creation_value);
                body.instruction(&Instruction::StructNew(obj_type));
            }
            CreateMode::StructNewDefault => {
                body.instruction(&Instruction::StructNewDefault(obj_type));
            }
            CreateMode::ArrayNew => {
                self.emit_value(body, self.cfg.creation_value);
                body.instruction(&Instruction::I32Const(self.cfg.array_len as i32));
                body.instruction(&Instruction::ArrayNew(obj_type));
            }
            CreateMode::ArrayNewDefault => {
                body.instruction(&Instruction::I32Const(self.cfg.array_len as i32));
                body.instruction(&Instruction::ArrayNewDefault(obj_type));
            }
            CreateMode::ArrayNewFixed => {
                for _ in 0..self.cfg.array_len {
                    self.emit_value(body, self.cfg.creation_value);
                }
                body.instruction(&Instruction::ArrayNewFixed {
                    array_type_index: obj_type,
                    array_size: self.cfg.array_len,
                });
            }
            CreateMode::ArrayNewData => {
                body.instruction(&Instruction::I32Const(0));
                body.instruction(&Instruction::I32Const(self.cfg.array_len as i32));
                body.instruction(&Instruction::ArrayNewData {
                    array_type_index: obj_type,
                    array_data_index: self.data_d.unwrap(),
                });
            }
            CreateMode::ArrayNewElem => {
                body.instruction(&Instruction::I32Const(0));
                body.instruction(&Instruction::I32Const(self.cfg.array_len as i32));
                body.instruction(&Instruction::ArrayNewElem {
                    array_type_index: obj_type,
                    array_elem_index: self.elem_e.unwrap(),
                });
            }
            CreateMode::GlobalGet => {
                body.instruction(&Instruction::GlobalGet(self.global_g.unwrap()));
            }
            CreateMode::TableGet => {
                body.instruction(&Instruction::I32Const(0));
                body.instruction(&Instruction::TableGet(self.table_t.unwrap()));
            }
            CreateMode::Call => {
                body.instruction(&Instruction::Call(self.make_obj_func.unwrap()));
            }
            CreateMode::LocalGet => unreachable!(),
        }
    }

    fn obj_const_expr(&self, val: i32) -> ConstExpr {
        let obj_type = self.obj_type.unwrap();
        let mut bytes = Vec::new();
        if self.cfg.object_type.is_array() {
            self.push_value_instrs(&mut bytes, val);
            Instruction::I32Const(self.cfg.array_len as i32).encode(&mut bytes);
            Instruction::ArrayNew(obj_type).encode(&mut bytes);
        } else {
            self.push_value_instrs(&mut bytes, val);
            Instruction::StructNew(obj_type).encode(&mut bytes);
        }
        ConstExpr::raw(bytes)
    }

    fn ref_value_const_expr(&self, val: i32) -> ConstExpr {
        let mut bytes = Vec::new();
        if let Some(nested) = self.nested_obj_type {
            Instruction::StructNew(nested).encode(&mut bytes);
        } else {
            Instruction::I32Const(val).encode(&mut bytes);
            Instruction::RefI31.encode(&mut bytes);
        }
        ConstExpr::raw(bytes)
    }

    fn push_value_instrs(&self, buf: &mut Vec<u8>, val: i32) {
        match self.cfg.object_type.elem_type {
            ElemType::I8 | ElemType::I32 => {
                Instruction::I32Const(val).encode(buf);
            }
            ElemType::I31ref => {
                Instruction::I32Const(val).encode(buf);
                Instruction::RefI31.encode(buf);
            }
            ElemType::Anyref => {
                if self.cfg.create_mode.is_default() {
                    Instruction::RefNull(wasm_encoder::HeapType::Abstract {
                        shared: false,
                        ty: wasm_encoder::AbstractHeapType::Any,
                    })
                    .encode(buf);
                } else if let Some(nested) = self.nested_obj_type {
                    Instruction::StructNew(nested).encode(buf);
                } else {
                    Instruction::I32Const(val).encode(buf);
                    Instruction::RefI31.encode(buf);
                }
            }
        }
    }

    fn emit_value(&self, body: &mut Function, val: i32) {
        match self.cfg.object_type.elem_type {
            ElemType::I8 | ElemType::I32 => {
                body.instruction(&Instruction::I32Const(val));
            }
            ElemType::I31ref => {
                body.instruction(&Instruction::I32Const(val));
                body.instruction(&Instruction::RefI31);
            }
            ElemType::Anyref => {
                if self.cfg.create_mode.is_default() {
                    body.instruction(&Instruction::RefNull(wasm_encoder::HeapType::Abstract {
                        shared: false,
                        ty: wasm_encoder::AbstractHeapType::Any,
                    }));
                } else if let Some(nested) = self.nested_obj_type {
                    body.instruction(&Instruction::StructNew(nested));
                } else {
                    body.instruction(&Instruction::I32Const(val));
                    body.instruction(&Instruction::RefI31);
                }
            }
        }
    }

    /// Compute the i32 value we expect to read back from a numeric field
    /// written with `val`, after packed storage truncation and unsigned
    /// extension via `{array,struct}.get_u`.
    fn expected_numeric_readback(&self, val: i32) -> i32 {
        match self.cfg.object_type.elem_type {
            ElemType::I8 => val & 0xFF,
            ElemType::I32 => val,
            _ => unreachable!(),
        }
    }

    fn emit_mutation_value(&self, body: &mut Function) {
        match self.cfg.object_type.elem_type {
            ElemType::I8 | ElemType::I32 => {
                body.instruction(&Instruction::I32Const(self.cfg.mutation_value));
            }
            ElemType::I31ref | ElemType::Anyref => {
                body.instruction(&Instruction::I32Const(self.cfg.mutation_value));
                body.instruction(&Instruction::RefI31);
            }
        }
    }

    fn emit_obj_ref(&self, body: &mut Function, r: ObjRef) {
        match r {
            ObjRef::Local(l) => {
                body.instruction(&Instruction::LocalGet(l));
            }
            ObjRef::Global => {
                body.instruction(&Instruction::GlobalGet(self.global_g.unwrap()));
            }
            ObjRef::Table => {
                body.instruction(&Instruction::I32Const(0));
                body.instruction(&Instruction::TableGet(self.table_t.unwrap()));
            }
        }
    }

    fn idempotent_ref(&self) -> ObjRef {
        match self.cfg.create_mode {
            CreateMode::GlobalGet => ObjRef::Global,
            CreateMode::TableGet => ObjRef::Table,
            _ => unreachable!(),
        }
    }

    fn emit_access(
        &self,
        body: &mut Function,
        r: ObjRef,
        local_before: Option<u32>,
        local_src: Option<u32>,
    ) {
        match self.cfg.access_mode {
            AccessMode::RefTest => {
                self.emit_obj_ref(body, r);
                body.instruction(&Instruction::RefTestNonNull(
                    wasm_encoder::HeapType::Concrete(self.obj_type.unwrap()),
                ));
                body.instruction(&Instruction::LocalSet(local_before.unwrap()));
            }
            AccessMode::StructGet => self.emit_get_access(body, r, local_before),
            AccessMode::StructGetS => {
                self.emit_obj_ref(body, r);
                body.instruction(&Instruction::StructGetS {
                    struct_type_index: self.obj_type.unwrap(),
                    field_index: 0,
                });
                body.instruction(&Instruction::LocalSet(local_before.unwrap()));
            }
            AccessMode::StructGetU => {
                self.emit_obj_ref(body, r);
                body.instruction(&Instruction::StructGetU {
                    struct_type_index: self.obj_type.unwrap(),
                    field_index: 0,
                });
                body.instruction(&Instruction::LocalSet(local_before.unwrap()));
            }
            AccessMode::StructSet => {
                self.emit_obj_ref(body, r);
                self.emit_mutation_value(body);
                body.instruction(&Instruction::StructSet {
                    struct_type_index: self.obj_type.unwrap(),
                    field_index: 0,
                });
            }
            AccessMode::ArrayGet => self.emit_get_access(body, r, local_before),
            AccessMode::ArrayGetS => {
                self.emit_obj_ref(body, r);
                body.instruction(&Instruction::I32Const(0));
                body.instruction(&Instruction::ArrayGetS(self.obj_type.unwrap()));
                body.instruction(&Instruction::LocalSet(local_before.unwrap()));
            }
            AccessMode::ArrayGetU => {
                self.emit_obj_ref(body, r);
                body.instruction(&Instruction::I32Const(0));
                body.instruction(&Instruction::ArrayGetU(self.obj_type.unwrap()));
                body.instruction(&Instruction::LocalSet(local_before.unwrap()));
            }
            AccessMode::ArraySet => {
                self.emit_obj_ref(body, r);
                body.instruction(&Instruction::I32Const(0));
                self.emit_mutation_value(body);
                body.instruction(&Instruction::ArraySet(self.obj_type.unwrap()));
            }
            AccessMode::ArrayLen => {
                self.emit_obj_ref(body, r);
                body.instruction(&Instruction::ArrayLen);
                body.instruction(&Instruction::LocalSet(local_before.unwrap()));
            }
            AccessMode::ArrayFill => {
                self.emit_obj_ref(body, r);
                body.instruction(&Instruction::I32Const(0));
                self.emit_mutation_value(body);
                body.instruction(&Instruction::I32Const(self.cfg.array_len as i32));
                body.instruction(&Instruction::ArrayFill(self.obj_type.unwrap()));
            }
            AccessMode::ArrayCopy => {
                let src = local_src.unwrap();
                self.emit_mutation_value(body);
                body.instruction(&Instruction::I32Const(self.cfg.array_len as i32));
                body.instruction(&Instruction::ArrayNew(self.obj_type.unwrap()));
                body.instruction(&Instruction::LocalSet(src));
                self.emit_obj_ref(body, r);
                body.instruction(&Instruction::I32Const(0));
                body.instruction(&Instruction::LocalGet(src));
                body.instruction(&Instruction::I32Const(0));
                body.instruction(&Instruction::I32Const(self.cfg.array_len as i32));
                body.instruction(&Instruction::ArrayCopy {
                    array_type_index_dst: self.obj_type.unwrap(),
                    array_type_index_src: self.obj_type.unwrap(),
                });
            }
            AccessMode::ArrayInitData => {
                self.emit_obj_ref(body, r);
                body.instruction(&Instruction::I32Const(0));
                body.instruction(&Instruction::I32Const(0));
                body.instruction(&Instruction::I32Const(1));
                body.instruction(&Instruction::ArrayInitData {
                    array_type_index: self.obj_type.unwrap(),
                    array_data_index: self.data_dinit.unwrap(),
                });
            }
            AccessMode::ArrayInitElem => {
                self.emit_obj_ref(body, r);
                body.instruction(&Instruction::I32Const(0));
                body.instruction(&Instruction::I32Const(0));
                body.instruction(&Instruction::I32Const(1));
                body.instruction(&Instruction::ArrayInitElem {
                    array_type_index: self.obj_type.unwrap(),
                    array_elem_index: self.elem_einit.unwrap(),
                });
            }
        }
    }

    fn emit_get_access(&self, body: &mut Function, r: ObjRef, local_before: Option<u32>) {
        if self.cfg.object_type.elem_type.is_ref() {
            if self.cfg.create_mode.is_default() {
                self.emit_read_ref(body, r);
                body.instruction(&Instruction::Drop);
            } else if self.uses_nested_obj() {
                self.emit_read_ref(body, r);
                body.instruction(&Instruction::Drop);
            } else {
                self.emit_read_ref(body, r);
                body.instruction(&Instruction::RefCastNonNull(
                    wasm_encoder::HeapType::Abstract {
                        shared: false,
                        ty: wasm_encoder::AbstractHeapType::I31,
                    },
                ));
                body.instruction(&Instruction::I31GetU);
                body.instruction(&Instruction::LocalSet(local_before.unwrap()));
            }
        } else if self.cfg.object_type.is_array() {
            self.emit_obj_ref(body, r);
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::ArrayGet(self.obj_type.unwrap()));
            body.instruction(&Instruction::LocalSet(local_before.unwrap()));
        } else {
            self.emit_obj_ref(body, r);
            body.instruction(&Instruction::StructGet {
                struct_type_index: self.obj_type.unwrap(),
                field_index: 0,
            });
            body.instruction(&Instruction::LocalSet(local_before.unwrap()));
        }
    }

    fn emit_assert(&self, body: &mut Function, r: ObjRef, local_before: Option<u32>) {
        match self.cfg.access_mode {
            AccessMode::RefTest => {
                body.instruction(&Instruction::LocalGet(local_before.unwrap()));
                self.emit_obj_ref(body, r);
                body.instruction(&Instruction::RefTestNonNull(
                    wasm_encoder::HeapType::Concrete(self.obj_type.unwrap()),
                ));
                body.instruction(&Instruction::I32Ne);
                self.emit_trap_if(body);
            }
            AccessMode::StructGet | AccessMode::ArrayGet => {
                self.emit_get_assert(body, r, local_before)
            }
            AccessMode::StructGetS => {
                body.instruction(&Instruction::LocalGet(local_before.unwrap()));
                self.emit_obj_ref(body, r);
                body.instruction(&Instruction::StructGetS {
                    struct_type_index: self.obj_type.unwrap(),
                    field_index: 0,
                });
                body.instruction(&Instruction::I32Ne);
                self.emit_trap_if(body);
            }
            AccessMode::StructGetU => {
                body.instruction(&Instruction::LocalGet(local_before.unwrap()));
                self.emit_obj_ref(body, r);
                body.instruction(&Instruction::StructGetU {
                    struct_type_index: self.obj_type.unwrap(),
                    field_index: 0,
                });
                body.instruction(&Instruction::I32Ne);
                self.emit_trap_if(body);
            }
            AccessMode::StructSet => {
                self.emit_set_assert(body, r);
            }
            AccessMode::ArrayGetS => {
                body.instruction(&Instruction::LocalGet(local_before.unwrap()));
                self.emit_obj_ref(body, r);
                body.instruction(&Instruction::I32Const(0));
                body.instruction(&Instruction::ArrayGetS(self.obj_type.unwrap()));
                body.instruction(&Instruction::I32Ne);
                self.emit_trap_if(body);
            }
            AccessMode::ArrayGetU => {
                body.instruction(&Instruction::LocalGet(local_before.unwrap()));
                self.emit_obj_ref(body, r);
                body.instruction(&Instruction::I32Const(0));
                body.instruction(&Instruction::ArrayGetU(self.obj_type.unwrap()));
                body.instruction(&Instruction::I32Ne);
                self.emit_trap_if(body);
            }
            AccessMode::ArraySet => {
                self.emit_set_assert(body, r);
            }
            AccessMode::ArrayLen => {
                body.instruction(&Instruction::LocalGet(local_before.unwrap()));
                self.emit_obj_ref(body, r);
                body.instruction(&Instruction::ArrayLen);
                body.instruction(&Instruction::I32Ne);
                self.emit_trap_if(body);
            }
            AccessMode::ArrayFill => {
                self.emit_mutation_assert(body, r);
            }
            AccessMode::ArrayCopy => {
                self.emit_mutation_assert(body, r);
            }
            AccessMode::ArrayInitData => {
                self.emit_read_field(body, r, 0);
                let expected = self.expected_numeric_readback(self.cfg.mutation_value);
                body.instruction(&Instruction::I32Const(expected));
                body.instruction(&Instruction::I32Ne);
                self.emit_trap_if(body);
            }
            AccessMode::ArrayInitElem => {
                self.emit_obj_ref(body, r);
                body.instruction(&Instruction::I32Const(0));
                body.instruction(&Instruction::ArrayGet(self.obj_type.unwrap()));
                body.instruction(&Instruction::RefCastNonNull(
                    wasm_encoder::HeapType::Abstract {
                        shared: false,
                        ty: wasm_encoder::AbstractHeapType::I31,
                    },
                ));
                body.instruction(&Instruction::I31GetU);
                body.instruction(&Instruction::I32Const(
                    self.cfg.mutation_value & 0x7FFF_FFFF,
                ));
                body.instruction(&Instruction::I32Ne);
                self.emit_trap_if(body);
            }
        }
    }

    fn emit_get_assert(&self, body: &mut Function, r: ObjRef, local_before: Option<u32>) {
        if self.cfg.object_type.elem_type.is_ref() {
            if self.cfg.create_mode.is_default() {
                self.emit_read_ref(body, r);
                body.instruction(&Instruction::RefIsNull);
                body.instruction(&Instruction::I32Eqz);
                self.emit_trap_if(body);
            } else if self.uses_nested_obj() {
                self.emit_read_ref(body, r);
                body.instruction(&Instruction::RefIsNull);
                self.emit_trap_if(body);
            } else {
                body.instruction(&Instruction::LocalGet(local_before.unwrap()));
                self.emit_read_ref(body, r);
                body.instruction(&Instruction::RefCastNonNull(
                    wasm_encoder::HeapType::Abstract {
                        shared: false,
                        ty: wasm_encoder::AbstractHeapType::I31,
                    },
                ));
                body.instruction(&Instruction::I31GetU);
                body.instruction(&Instruction::I32Ne);
                self.emit_trap_if(body);
            }
        } else if self.cfg.object_type.is_array() {
            body.instruction(&Instruction::LocalGet(local_before.unwrap()));
            self.emit_obj_ref(body, r);
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::ArrayGet(self.obj_type.unwrap()));
            body.instruction(&Instruction::I32Ne);
            self.emit_trap_if(body);
        } else {
            body.instruction(&Instruction::LocalGet(local_before.unwrap()));
            self.emit_obj_ref(body, r);
            body.instruction(&Instruction::StructGet {
                struct_type_index: self.obj_type.unwrap(),
                field_index: 0,
            });
            body.instruction(&Instruction::I32Ne);
            self.emit_trap_if(body);
        }
    }

    fn emit_set_assert(&self, body: &mut Function, r: ObjRef) {
        if self.cfg.object_type.elem_type.is_ref() {
            self.emit_read_ref(body, r);
            body.instruction(&Instruction::RefCastNonNull(
                wasm_encoder::HeapType::Abstract {
                    shared: false,
                    ty: wasm_encoder::AbstractHeapType::I31,
                },
            ));
            body.instruction(&Instruction::I31GetU);
            body.instruction(&Instruction::I32Const(
                self.cfg.mutation_value & 0x7FFF_FFFF,
            ));
            body.instruction(&Instruction::I32Ne);
            self.emit_trap_if(body);
        } else {
            self.emit_read_field(body, r, 0);
            let expected = self.expected_numeric_readback(self.cfg.mutation_value);
            body.instruction(&Instruction::I32Const(expected));
            body.instruction(&Instruction::I32Ne);
            self.emit_trap_if(body);
        }
    }

    fn emit_mutation_assert(&self, body: &mut Function, r: ObjRef) {
        if self.cfg.object_type.elem_type.is_ref() {
            self.emit_obj_ref(body, r);
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::ArrayGet(self.obj_type.unwrap()));
            body.instruction(&Instruction::RefCastNonNull(
                wasm_encoder::HeapType::Abstract {
                    shared: false,
                    ty: wasm_encoder::AbstractHeapType::I31,
                },
            ));
            body.instruction(&Instruction::I31GetU);
            body.instruction(&Instruction::I32Const(
                self.cfg.mutation_value & 0x7FFF_FFFF,
            ));
        } else {
            self.emit_read_field(body, r, 0);
            let expected = self.expected_numeric_readback(self.cfg.mutation_value);
            body.instruction(&Instruction::I32Const(expected));
        }
        body.instruction(&Instruction::I32Ne);
        self.emit_trap_if(body);
    }

    fn emit_read_ref(&self, body: &mut Function, r: ObjRef) {
        if self.cfg.object_type.is_array() {
            self.emit_obj_ref(body, r);
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::ArrayGet(self.obj_type.unwrap()));
        } else {
            self.emit_obj_ref(body, r);
            body.instruction(&Instruction::StructGet {
                struct_type_index: self.obj_type.unwrap(),
                field_index: 0,
            });
        }
    }

    fn emit_read_field(&self, body: &mut Function, r: ObjRef, idx: u32) {
        if self.cfg.object_type.is_array() {
            self.emit_obj_ref(body, r);
            body.instruction(&Instruction::I32Const(idx as i32));
            if self.cfg.object_type.elem_type.is_packed() {
                body.instruction(&Instruction::ArrayGetU(self.obj_type.unwrap()));
            } else {
                body.instruction(&Instruction::ArrayGet(self.obj_type.unwrap()));
            }
        } else {
            self.emit_obj_ref(body, r);
            if self.cfg.object_type.elem_type.is_packed() {
                body.instruction(&Instruction::StructGetU {
                    struct_type_index: self.obj_type.unwrap(),
                    field_index: idx,
                });
            } else {
                body.instruction(&Instruction::StructGet {
                    struct_type_index: self.obj_type.unwrap(),
                    field_index: idx,
                });
            }
        }
    }

    fn emit_trap_if(&self, body: &mut Function) {
        body.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        body.instruction(&Instruction::Unreachable);
        body.instruction(&Instruction::End);
    }

    /// Normal (non-idempotent, non-local_get) run body.
    ///
    /// Layout follows INSTRUCTIONS.md: outer loop with create + access + gc +
    /// assert, then inner loop with access + assert, then trailing access +
    /// assert.
    fn emit_normal_body(&self, body: &mut Function) {
        // locals: $x=0, $i=1, $j=2, [$before=3, [$src=4]]
        let local_x = 0u32;
        let local_i = 1u32;
        let local_j = 2u32;
        let local_before = if self.needs_before() {
            Some(3u32)
        } else {
            None
        };
        let local_src = if matches!(self.cfg.access_mode, AccessMode::ArrayCopy) {
            Some(if self.needs_before() { 4u32 } else { 3u32 })
        } else {
            None
        };
        let r = ObjRef::Local(local_x);

        // i = 0 (already 0)
        // (loop $outer
        body.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        //   (if (i32.ge_u $i 10) (return))
        body.instruction(&Instruction::LocalGet(local_i));
        body.instruction(&Instruction::I32Const(10));
        body.instruction(&Instruction::I32GeU);
        body.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        body.instruction(&Instruction::Return);
        body.instruction(&Instruction::End);

        //   (local.set $x CREATE)
        self.emit_create(body);
        body.instruction(&Instruction::LocalSet(local_x));

        //   ACCESS
        self.emit_access(body, r, local_before, local_src);
        //   gc()
        body.instruction(&Instruction::Call(self.gc_func.unwrap()));
        //   ASSERT
        self.emit_assert(body, r, local_before);

        //   j = 0
        body.instruction(&Instruction::I32Const(0));
        body.instruction(&Instruction::LocalSet(local_j));

        //   (block $break (loop $inner
        body.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        body.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        //     (if (i32.lt_u $j 10)
        body.instruction(&Instruction::LocalGet(local_j));
        body.instruction(&Instruction::I32Const(10));
        body.instruction(&Instruction::I32LtU);
        body.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));

        //       (then ACCESS; ASSERT; j += 1; br $inner)
        self.emit_access(body, r, local_before, local_src);
        self.emit_assert(body, r, local_before);
        body.instruction(&Instruction::LocalGet(local_j));
        body.instruction(&Instruction::I32Const(1));
        body.instruction(&Instruction::I32Add);
        body.instruction(&Instruction::LocalSet(local_j));
        body.instruction(&Instruction::Br(1)); // br $inner

        //       (else ACCESS; ASSERT; j += 1; br $break)
        body.instruction(&Instruction::Else);
        self.emit_access(body, r, local_before, local_src);
        self.emit_assert(body, r, local_before);
        body.instruction(&Instruction::LocalGet(local_j));
        body.instruction(&Instruction::I32Const(1));
        body.instruction(&Instruction::I32Add);
        body.instruction(&Instruction::LocalSet(local_j));
        body.instruction(&Instruction::Br(2)); // br $break

        body.instruction(&Instruction::End); // end if
        body.instruction(&Instruction::End); // end loop $inner
        body.instruction(&Instruction::End); // end block $break

        //   ACCESS; ASSERT
        self.emit_access(body, r, local_before, local_src);
        self.emit_assert(body, r, local_before);

        //   i += 1
        body.instruction(&Instruction::LocalGet(local_i));
        body.instruction(&Instruction::I32Const(1));
        body.instruction(&Instruction::I32Add);
        body.instruction(&Instruction::LocalSet(local_i));

        //   br $outer
        body.instruction(&Instruction::Br(0));

        body.instruction(&Instruction::End); // end loop $outer
    }

    /// Idempotent (global_get / table_get) run body — no $x local.
    fn emit_idempotent_body(&self, body: &mut Function) {
        // locals: $i=0, $j=1, [$before=2, [$src=3]]
        let local_i = 0u32;
        let local_j = 1u32;
        let local_before = if self.needs_before() {
            Some(2u32)
        } else {
            None
        };
        let local_src = if matches!(self.cfg.access_mode, AccessMode::ArrayCopy) {
            Some(if self.needs_before() { 3u32 } else { 2u32 })
        } else {
            None
        };
        let r = self.idempotent_ref();

        body.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        body.instruction(&Instruction::LocalGet(local_i));
        body.instruction(&Instruction::I32Const(10));
        body.instruction(&Instruction::I32GeU);
        body.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        body.instruction(&Instruction::Return);
        body.instruction(&Instruction::End);

        self.emit_access(body, r, local_before, local_src);
        body.instruction(&Instruction::Call(self.gc_func.unwrap()));
        self.emit_assert(body, r, local_before);

        body.instruction(&Instruction::I32Const(0));
        body.instruction(&Instruction::LocalSet(local_j));

        body.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        body.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        body.instruction(&Instruction::LocalGet(local_j));
        body.instruction(&Instruction::I32Const(10));
        body.instruction(&Instruction::I32LtU);
        body.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));

        self.emit_access(body, r, local_before, local_src);
        self.emit_assert(body, r, local_before);
        body.instruction(&Instruction::LocalGet(local_j));
        body.instruction(&Instruction::I32Const(1));
        body.instruction(&Instruction::I32Add);
        body.instruction(&Instruction::LocalSet(local_j));
        body.instruction(&Instruction::Br(1));

        body.instruction(&Instruction::Else);
        self.emit_access(body, r, local_before, local_src);
        self.emit_assert(body, r, local_before);
        body.instruction(&Instruction::LocalGet(local_j));
        body.instruction(&Instruction::I32Const(1));
        body.instruction(&Instruction::I32Add);
        body.instruction(&Instruction::LocalSet(local_j));
        body.instruction(&Instruction::Br(2));

        body.instruction(&Instruction::End); // end if
        body.instruction(&Instruction::End); // end loop $inner
        body.instruction(&Instruction::End); // end block $break

        self.emit_access(body, r, local_before, local_src);
        self.emit_assert(body, r, local_before);

        body.instruction(&Instruction::LocalGet(local_i));
        body.instruction(&Instruction::I32Const(1));
        body.instruction(&Instruction::I32Add);
        body.instruction(&Instruction::LocalSet(local_i));

        body.instruction(&Instruction::Br(0));
        body.instruction(&Instruction::End); // end loop $outer
    }

    /// local_get mode — param $p is the object ref.
    fn emit_local_get_body(&self, body: &mut Function) {
        // param: $p=0, locals: $i=1, $j=2, [$before=3, [$src=4]]
        let param_p = 0u32;
        let local_i = 1u32;
        let local_j = 2u32;
        let local_before = if self.needs_before() {
            Some(3u32)
        } else {
            None
        };
        let local_src = if matches!(self.cfg.access_mode, AccessMode::ArrayCopy) {
            Some(if self.needs_before() { 4u32 } else { 3u32 })
        } else {
            None
        };
        let r = ObjRef::Local(param_p);

        body.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        body.instruction(&Instruction::LocalGet(local_i));
        body.instruction(&Instruction::I32Const(10));
        body.instruction(&Instruction::I32GeU);
        body.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        body.instruction(&Instruction::Return);
        body.instruction(&Instruction::End);

        self.emit_access(body, r, local_before, local_src);
        body.instruction(&Instruction::Call(self.gc_func.unwrap()));
        self.emit_assert(body, r, local_before);

        body.instruction(&Instruction::I32Const(0));
        body.instruction(&Instruction::LocalSet(local_j));

        body.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        body.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        body.instruction(&Instruction::LocalGet(local_j));
        body.instruction(&Instruction::I32Const(10));
        body.instruction(&Instruction::I32LtU);
        body.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));

        self.emit_access(body, r, local_before, local_src);
        self.emit_assert(body, r, local_before);
        body.instruction(&Instruction::LocalGet(local_j));
        body.instruction(&Instruction::I32Const(1));
        body.instruction(&Instruction::I32Add);
        body.instruction(&Instruction::LocalSet(local_j));
        body.instruction(&Instruction::Br(1));

        body.instruction(&Instruction::Else);
        self.emit_access(body, r, local_before, local_src);
        self.emit_assert(body, r, local_before);
        body.instruction(&Instruction::LocalGet(local_j));
        body.instruction(&Instruction::I32Const(1));
        body.instruction(&Instruction::I32Add);
        body.instruction(&Instruction::LocalSet(local_j));
        body.instruction(&Instruction::Br(2));

        body.instruction(&Instruction::End); // end if
        body.instruction(&Instruction::End); // end loop $inner
        body.instruction(&Instruction::End); // end block $break

        self.emit_access(body, r, local_before, local_src);
        self.emit_assert(body, r, local_before);

        body.instruction(&Instruction::LocalGet(local_i));
        body.instruction(&Instruction::I32Const(1));
        body.instruction(&Instruction::I32Add);
        body.instruction(&Instruction::LocalSet(local_i));

        body.instruction(&Instruction::Br(0));
        body.instruction(&Instruction::End); // end loop $outer
    }
}

/// How the test references the GC object when performing accesses and
/// assertions.
#[derive(Clone, Copy)]
enum ObjRef {
    /// The object is held in a local variable.
    Local(u32),
    /// The object is retrieved from an immutable global each time.
    Global,
    /// The object is retrieved from a table slot each time.
    Table,
}

fn sec(module: &mut Module, section: impl wasm_encoder::Section) {
    module.section(&section);
}

fn finish(code: &mut CodeSection, mut f: Function) {
    f.instruction(&Instruction::End);
    code.function(&f);
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasmparser::Validator;

    #[test]
    fn gc_access_generates_valid_wasm() {
        crate::test::test_n_times(1024, |input: GcAccess, _u| {
            let wasm = input.to_wasm();
            validate(&wasm);
            Ok(())
        })
    }

    fn validate(wasm: &[u8]) {
        let mut validator = Validator::new_with_features(wasmparser::WasmFeatures::all());
        let err = match validator.validate_all(wasm) {
            Ok(_) => return,
            Err(e) => e,
        };
        drop(std::fs::write("test.wasm", wasm));
        if let Ok(text) = wasmprinter::print_bytes(wasm) {
            drop(std::fs::write("test.wat", &text));
        }
        panic!("wasm failed to validate: {err}");
    }
}
