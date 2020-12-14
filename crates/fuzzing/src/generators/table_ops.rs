//! Generating series of `table.get` and `table.set` operations.

use arbitrary::Arbitrary;
use std::ops::Range;
use wasm_encoder::{
    CodeSection, Export, ExportSection, Function, FunctionSection, ImportSection, ImportType,
    Instruction, Limits, Module, TableSection, TableType, TypeSection, ValType,
};

/// A description of a Wasm module that makes a series of `externref` table
/// operations.
#[derive(Arbitrary, Debug)]
pub struct TableOps {
    num_params: u8,
    table_size: u32,
    ops: Vec<TableOp>,
}

const NUM_PARAMS_RANGE: Range<u8> = 1..10;
const TABLE_SIZE_RANGE: Range<u32> = 1..100;
const MAX_OPS: usize = 1000;

impl TableOps {
    /// Get the number of parameters this module's "run" function takes.
    pub fn num_params(&self) -> u8 {
        let num_params = std::cmp::max(self.num_params, NUM_PARAMS_RANGE.start);
        let num_params = std::cmp::min(num_params, NUM_PARAMS_RANGE.end);
        num_params
    }

    /// Get the size of the table that this module uses.
    pub fn table_size(&self) -> u32 {
        let table_size = std::cmp::max(self.table_size, TABLE_SIZE_RANGE.start);
        let table_size = std::cmp::min(table_size, TABLE_SIZE_RANGE.end);
        table_size
    }

    /// Convert into a a wasm_encoded binary.
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

        // Import the GC function.
        let mut imports = ImportSection::new();
        imports.import("", "gc", ImportType::Function(0));

        // Define our table.
        let mut tables = TableSection::new();
        tables.table(TableType {
            element_type: ValType::ExternRef,
            limits: Limits {
                min: self.table_size(),
                max: None,
            },
        });

        // Encode the type section for the "run" function.
        let mut types = TypeSection::new();
        let mut params: Vec<ValType> = Vec::with_capacity(self.num_params() as usize);
        for _i in 0..self.num_params() {
            params.push(ValType::ExternRef);
        }
        let results = vec![];
        types.function(params, results);

        // Define the "run" function export.
        let mut functions = FunctionSection::new();
        functions.function(0);

        let mut exports = ExportSection::new();
        exports.export("run", Export::Function(0));

        let mut params: Vec<(u32, ValType)> = Vec::with_capacity(self.num_params() as usize);
        for i in 0..self.num_params() {
            params.push((0, ValType::ExternRef));
        }
        let mut func = Function::new(params);

        for op in self.ops.iter().take(MAX_OPS) {
            op.insert(&mut func);
        }

        let mut code = CodeSection::new();
        code.function(&func);

        module
            .section(&types)
            .section(&imports)
            .section(&functions)
            .section(&tables)
            .section(&exports)
            .section(&code);

        module.finish()
    }
}

#[derive(Arbitrary, Debug)]
pub(crate) enum TableOp {
    // `(call 0)`
    Gc,
    // `(drop (table.get x))`
    Get(u32),
    // `(table.set x (local.get y))`
    SetFromParam(u32, u8),
    // `(table.set x (table.get y))`
    SetFromGet(u32, u32),
}

impl TableOp {
    fn insert(&self, func: &mut Function) {
        match self {
            Self::Gc => {
                func.instruction(Instruction::Call(0));
            }
            Self::Get(x) => {
                func.instruction(Instruction::Drop);
                func.instruction(Instruction::TableGet { table: *x });
            }
            Self::SetFromParam(x, y) => {
                func.instruction(Instruction::TableSet { table: *x });
                func.instruction(Instruction::LocalGet((*y).into()));
            }
            Self::SetFromGet(x, y) => {
                func.instruction(Instruction::TableSet { table: *x });
                func.instruction(Instruction::TableGet { table: *y });
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
            num_params: 2,
            table_size: 10,
            ops: vec![
                TableOp::Gc,
                TableOp::Get(0),
                TableOp::SetFromParam(1, 2),
                TableOp::SetFromGet(3, 4),
            ],
        };

        let expected = r#"
(module
  (type (;0;) (func (param externref externref)))
  (import "" "gc" (func (;0;) (type 0)))
  (func (;1;) (type 0) (param externref externref)
    call 0
    drop
    table.get 0
    table.set 1
    local.get 2
    table.set 3
    table.get 4)
  (table (;0;) 10 externref)
  (export "run" (func 0)))
"#;
        let actual = ops.to_wasm_binary();
        let actual = wasmprinter::print_bytes(&actual).unwrap();
        assert_eq!(actual.trim(), expected.trim());
    }
}
