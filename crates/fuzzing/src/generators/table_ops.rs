//! Generating series of `table.get` and `table.set` operations.

use arbitrary::Arbitrary;
use std::ops::Range;
use wasm_encoder::{
    CodeSection, EntityType, Export, ExportSection, Function, FunctionSection, GlobalSection,
    ImportSection, Instruction, Module, TableSection, TableType, TypeSection, ValType,
};

/// A description of a Wasm module that makes a series of `externref` table
/// operations.
#[derive(Arbitrary, Debug)]
pub struct TableOps {
    num_params: u8,
    num_globals: u8,
    table_size: u32,
    ops: Vec<TableOp>,
}

const NUM_PARAMS_RANGE: Range<u8> = 1..10;
const NUM_GLOBALS_RANGE: Range<u8> = 1..10;
const TABLE_SIZE_RANGE: Range<u32> = 1..100;
const MAX_OPS: usize = 100;

impl TableOps {
    /// Get the number of parameters this module's "run" function takes.
    pub fn num_params(&self) -> u8 {
        let num_params = std::cmp::max(self.num_params, NUM_PARAMS_RANGE.start);
        let num_params = std::cmp::min(num_params, NUM_PARAMS_RANGE.end);
        num_params
    }

    /// Get the number of globals this module has.
    pub fn num_globals(&self) -> u8 {
        let num_globals = std::cmp::max(self.num_globals, NUM_GLOBALS_RANGE.start);
        let num_globals = std::cmp::min(num_globals, NUM_GLOBALS_RANGE.end);
        num_globals
    }

    /// Get the size of the table that this module uses.
    pub fn table_size(&self) -> u32 {
        let table_size = std::cmp::max(self.table_size, TABLE_SIZE_RANGE.start);
        let table_size = std::cmp::min(table_size, TABLE_SIZE_RANGE.end);
        table_size
    }

    /// Serialize this module into a Wasm binary.
    ///
    /// The module requires a single import: `(import "" "gc" (func))`. This
    /// should be a function to trigger GC.
    ///
    /// The single export of the module is a function "run" that takes
    /// `self.num_params()` parameters of type `externref`.
    ///
    /// The "run" function is guaranteed to terminate (no loops or recursive
    /// calls), but is not guaranteed to avoid traps (might access out-of-bounds
    /// of the table).
    pub fn to_wasm_binary(&self) -> Vec<u8> {
        let mut module = Module::new();

        // Encode the types for all functions that we are using.
        let mut types = TypeSection::new();

        // 0: "gc"
        types.function(
            vec![],
            // Return a bunch of stuff from `gc` so that we exercise GCing when
            // there is return pointer space allocated on the stack. This is
            // especially important because the x64 backend currently
            // dynamically adjusts the stack pointer for each call that uses
            // return pointers rather than statically allocating space in the
            // stack frame.
            vec![ValType::ExternRef, ValType::ExternRef, ValType::ExternRef],
        );

        // 1: "run"
        let mut params: Vec<ValType> = Vec::with_capacity(self.num_params() as usize);
        for _i in 0..self.num_params() {
            params.push(ValType::ExternRef);
        }
        let results = vec![];
        types.function(params, results);

        // 2: `take_refs`
        types.function(
            vec![ValType::ExternRef, ValType::ExternRef, ValType::ExternRef],
            vec![],
        );

        // 3: `make_refs`
        types.function(
            vec![],
            vec![ValType::ExternRef, ValType::ExternRef, ValType::ExternRef],
        );

        // Import the GC function.
        let mut imports = ImportSection::new();
        imports.import("", Some("gc"), EntityType::Function(0));
        imports.import("", Some("take_refs"), EntityType::Function(2));
        imports.import("", Some("make_refs"), EntityType::Function(3));

        // Define our table.
        let mut tables = TableSection::new();
        tables.table(TableType {
            element_type: ValType::ExternRef,
            minimum: self.table_size(),
            maximum: None,
        });

        // Define our globals.
        let mut globals = GlobalSection::new();
        for _ in 0..self.num_globals() {
            globals.global(
                wasm_encoder::GlobalType {
                    val_type: wasm_encoder::ValType::ExternRef,
                    mutable: true,
                },
                &Instruction::RefNull(wasm_encoder::ValType::ExternRef),
            );
        }

        // Define the "run" function export.
        let mut functions = FunctionSection::new();
        functions.function(1);

        let mut exports = ExportSection::new();
        exports.export("run", Export::Function(3));

        // Give ourselves one scratch local that we can use in various `TableOp`
        // implementations.
        let mut func = Function::new(vec![(1, ValType::ExternRef)]);

        func.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        for op in self.ops.iter().take(MAX_OPS) {
            op.insert(
                &mut func,
                self.num_params() as u32,
                self.table_size(),
                self.num_globals() as u32,
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
}

#[derive(Arbitrary, Copy, Clone, Debug)]
pub(crate) enum TableOp {
    // `call $gc; drop; drop; drop;`
    Gc,

    // `(drop (table.get x))`
    Get(i32),

    // `(drop (global.get i))`
    GetGlobal(u32),

    // `(table.set x (local.get y))`
    SetFromParam(i32, u32),

    // `(table.set x (table.get y))`
    SetFromGet(i32, i32),

    // `call $make_refs; table.set x; table.set y; table.set z`
    SetFromMake(i32, i32, i32),

    // `(global.set x (local.get y))`
    SetGlobalFromParam(u32, u32),

    // `(global.set x (table.get y))`
    SetGlobalFromGet(u32, i32),

    // `call $make_refs; global.set x; global.set y; global.set z`
    SetGlobalFromMake(u32, u32, u32),

    // `call $make_refs; drop; drop; drop;`
    Make,

    // `local.get x; local.get y; local.get z; call $take_refs`
    TakeFromParams(u32, u32, u32),

    // `table.get x; table.get y; table.get z; call $take_refs`
    TakeFromGet(i32, i32, i32),

    // `global.get x; global.get y; global.get z; call $take_refs`
    TakeFromGlobalGet(u32, u32, u32),

    // `call $make_refs; call $take_refs`
    TakeFromMake,

    // `call $gc; call $take_refs`
    TakeFromGc,
}

impl TableOp {
    fn insert(self, func: &mut Function, num_params: u32, table_size: u32, num_globals: u32) {
        assert!(num_params > 0);
        assert!(table_size > 0);

        // Add one to make sure that out of bounds table accesses are possible,
        // but still rare.
        let table_mod = table_size as i32 + 1;

        let gc_func_idx = 0;
        let take_refs_func_idx = 1;
        let make_refs_func_idx = 2;

        match self {
            Self::Gc => {
                func.instruction(&Instruction::Call(gc_func_idx));
                func.instruction(&Instruction::Drop);
                func.instruction(&Instruction::Drop);
                func.instruction(&Instruction::Drop);
            }
            Self::Get(x) => {
                func.instruction(&Instruction::I32Const(x % table_mod));
                func.instruction(&Instruction::TableGet { table: 0 });
                func.instruction(&Instruction::Drop);
            }
            Self::SetFromParam(x, y) => {
                func.instruction(&Instruction::I32Const(x % table_mod));
                func.instruction(&Instruction::LocalGet(y % num_params));
                func.instruction(&Instruction::TableSet { table: 0 });
            }
            Self::SetFromGet(x, y) => {
                func.instruction(&Instruction::I32Const(x % table_mod));
                func.instruction(&Instruction::I32Const(y % table_mod));
                func.instruction(&Instruction::TableGet { table: 0 });
                func.instruction(&Instruction::TableSet { table: 0 });
            }
            Self::SetFromMake(x, y, z) => {
                func.instruction(&Instruction::Call(make_refs_func_idx));

                func.instruction(&Instruction::LocalSet(num_params));
                func.instruction(&Instruction::I32Const(x % table_mod));
                func.instruction(&Instruction::LocalGet(num_params));
                func.instruction(&Instruction::TableSet { table: 0 });

                func.instruction(&Instruction::LocalSet(num_params));
                func.instruction(&Instruction::I32Const(y % table_mod));
                func.instruction(&Instruction::LocalGet(num_params));
                func.instruction(&Instruction::TableSet { table: 0 });

                func.instruction(&Instruction::LocalSet(num_params));
                func.instruction(&Instruction::I32Const(z % table_mod));
                func.instruction(&Instruction::LocalGet(num_params));
                func.instruction(&Instruction::TableSet { table: 0 });
            }
            TableOp::Make => {
                func.instruction(&Instruction::Call(make_refs_func_idx));
                func.instruction(&Instruction::Drop);
                func.instruction(&Instruction::Drop);
                func.instruction(&Instruction::Drop);
            }
            TableOp::TakeFromParams(x, y, z) => {
                func.instruction(&Instruction::LocalGet(x % num_params));
                func.instruction(&Instruction::LocalGet(y % num_params));
                func.instruction(&Instruction::LocalGet(z % num_params));
                func.instruction(&Instruction::Call(take_refs_func_idx));
            }
            TableOp::TakeFromGet(x, y, z) => {
                func.instruction(&Instruction::I32Const(x % table_mod));
                func.instruction(&Instruction::TableGet { table: 0 });

                func.instruction(&Instruction::I32Const(y % table_mod));
                func.instruction(&Instruction::TableGet { table: 0 });

                func.instruction(&Instruction::I32Const(z % table_mod));
                func.instruction(&Instruction::TableGet { table: 0 });

                func.instruction(&Instruction::Call(take_refs_func_idx));
            }
            TableOp::TakeFromMake => {
                func.instruction(&Instruction::Call(make_refs_func_idx));
                func.instruction(&Instruction::Call(take_refs_func_idx));
            }
            Self::TakeFromGc => {
                func.instruction(&Instruction::Call(gc_func_idx));
                func.instruction(&Instruction::Call(take_refs_func_idx));
            }
            TableOp::GetGlobal(x) => {
                func.instruction(&Instruction::GlobalGet(x % num_globals));
                func.instruction(&Instruction::Drop);
            }
            TableOp::SetGlobalFromParam(global, param) => {
                func.instruction(&Instruction::LocalGet(param % num_params));
                func.instruction(&Instruction::GlobalSet(global % num_globals));
            }
            TableOp::SetGlobalFromGet(global, x) => {
                func.instruction(&Instruction::I32Const(x));
                func.instruction(&Instruction::TableGet { table: 0 });
                func.instruction(&Instruction::GlobalSet(global % num_globals));
            }
            TableOp::SetGlobalFromMake(x, y, z) => {
                func.instruction(&Instruction::Call(make_refs_func_idx));
                func.instruction(&Instruction::GlobalSet(x % num_globals));
                func.instruction(&Instruction::GlobalSet(y % num_globals));
                func.instruction(&Instruction::GlobalSet(z % num_globals));
            }
            TableOp::TakeFromGlobalGet(x, y, z) => {
                func.instruction(&Instruction::GlobalGet(x % num_globals));
                func.instruction(&Instruction::GlobalGet(y % num_globals));
                func.instruction(&Instruction::GlobalGet(z % num_globals));
                func.instruction(&Instruction::Call(take_refs_func_idx));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wat_string() {
        let ops = TableOps {
            num_params: 5,
            num_globals: 1,
            table_size: 20,
            ops: vec![
                TableOp::Gc,
                TableOp::Get(0),
                TableOp::SetFromParam(1, 2),
                TableOp::SetFromGet(3, 4),
                TableOp::SetFromMake(5, 6, 7),
                TableOp::Make,
                TableOp::TakeFromParams(8, 9, 10),
                TableOp::TakeFromGet(11, 12, 13),
                TableOp::TakeFromMake,
                TableOp::GetGlobal(14),
                TableOp::SetGlobalFromParam(15, 16),
                TableOp::SetGlobalFromGet(17, 18),
                TableOp::SetGlobalFromMake(19, 20, 21),
                TableOp::TakeFromGlobalGet(22, 23, 24),
            ],
        };

        let expected = r#"
(module
  (type (;0;) (func (result externref externref externref)))
  (type (;1;) (func (param externref externref externref externref externref)))
  (type (;2;) (func (param externref externref externref)))
  (type (;3;) (func (result externref externref externref)))
  (import "" "gc" (func (;0;) (type 0)))
  (import "" "take_refs" (func (;1;) (type 2)))
  (import "" "make_refs" (func (;2;) (type 3)))
  (func (;3;) (type 1) (param externref externref externref externref externref)
    (local externref)
    loop  ;; label = @1
      call 0
      drop
      drop
      drop
      i32.const 0
      table.get 0
      drop
      i32.const 1
      local.get 2
      table.set 0
      i32.const 3
      i32.const 4
      table.get 0
      table.set 0
      call 2
      local.set 5
      i32.const 5
      local.get 5
      table.set 0
      local.set 5
      i32.const 6
      local.get 5
      table.set 0
      local.set 5
      i32.const 7
      local.get 5
      table.set 0
      call 2
      drop
      drop
      drop
      local.get 3
      local.get 4
      local.get 0
      call 1
      i32.const 11
      table.get 0
      i32.const 12
      table.get 0
      i32.const 13
      table.get 0
      call 1
      call 2
      call 1
      global.get 0
      drop
      local.get 1
      global.set 0
      i32.const 18
      table.get 0
      global.set 0
      call 2
      global.set 0
      global.set 0
      global.set 0
      global.get 0
      global.get 0
      global.get 0
      call 1
      br 0 (;@1;)
    end)
  (table (;0;) 20 externref)
  (global (;0;) (mut externref) ref.null extern)
  (export "run" (func 3)))
"#;
        eprintln!("expected WAT = {}", expected);

        let actual = ops.to_wasm_binary();
        if let Err(e) = wasmparser::validate(&actual) {
            panic!("TableOps should generate valid Wasm; got error: {}", e);
        }

        let actual = wasmprinter::print_bytes(&actual).unwrap();
        eprintln!("actual WAT = {}", actual);

        assert_eq!(actual.trim(), expected.trim());
    }
}
