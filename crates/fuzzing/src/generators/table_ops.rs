//! Generating series of `table.get` and `table.set` operations.

use arbitrary::{Arbitrary, Result, Unstructured};
use std::ops::RangeInclusive;
use wasm_encoder::{
    CodeSection, EntityType, ExportKind, ExportSection, Function, FunctionSection, GlobalSection,
    ImportSection, Instruction, Module, TableSection, TableType, TypeSection, ValType,
};

/// A description of a Wasm module that makes a series of `externref` table
/// operations.
#[derive(Debug)]
pub struct TableOps {
    pub(crate) num_params: u8,
    pub(crate) num_globals: u8,
    pub(crate) table_size: u32,
    ops: Vec<TableOp>,
}

const NUM_PARAMS_RANGE: RangeInclusive<u8> = 1..=10;
const NUM_GLOBALS_RANGE: RangeInclusive<u8> = 1..=10;
const TABLE_SIZE_RANGE: RangeInclusive<u32> = 1..=100;
const MAX_OPS: usize = 100;

impl TableOps {
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
        let mut params: Vec<ValType> = Vec::with_capacity(self.num_params as usize);
        for _i in 0..self.num_params {
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
        imports.import("", "gc", EntityType::Function(0));
        imports.import("", "take_refs", EntityType::Function(2));
        imports.import("", "make_refs", EntityType::Function(3));

        // Define our table.
        let mut tables = TableSection::new();
        tables.table(TableType {
            element_type: ValType::ExternRef,
            minimum: self.table_size,
            maximum: None,
        });

        // Define our globals.
        let mut globals = GlobalSection::new();
        for _ in 0..self.num_globals {
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
        exports.export("run", ExportKind::Func, 3);

        // Give ourselves one scratch local that we can use in various `TableOp`
        // implementations.
        let mut func = Function::new(vec![(1, ValType::ExternRef)]);

        func.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        for op in &self.ops {
            op.insert(
                &mut func,
                self.num_params as u32,
                self.table_size,
                self.num_globals as u32,
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

impl<'a> Arbitrary<'a> for TableOps {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        let num_params = u.int_in_range(NUM_PARAMS_RANGE)?;
        let num_globals = u.int_in_range(NUM_GLOBALS_RANGE)?;
        let table_size = u.int_in_range(TABLE_SIZE_RANGE)?;

        let mut stack = 0;
        let mut ops = vec![];
        let mut choices = vec![];
        loop {
            let keep_going = ops.len() < MAX_OPS && u.arbitrary().unwrap_or(false);
            if !keep_going {
                break;
            }

            ops.push(TableOp::arbitrary(u, &mut stack, &mut choices)?);
        }

        // Drop any extant refs on the stack.
        for _ in 0..stack {
            ops.push(TableOp::Drop);
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
            $op:ident $( ( $($imm:ty),* $(,)* ) )? : $params:expr => $results:expr ,
        )*
    ) => {
        #[derive(Copy, Clone, Debug)]
        pub(crate) enum TableOp {
            $(
                $op $( ( $($imm),* ) )? ,
            )*
        }

        impl TableOp {
            fn arbitrary(
                u: &mut Unstructured,
                stack: &mut u32,
                choices: &mut Vec<fn(&mut Unstructured, &mut u32) -> Result<TableOp>>,
            ) -> Result<TableOp> {
                choices.clear();

                // Add all the choices of valid `TableOp`s we could generate.
                $(
                    #[allow(unused_comparisons)]
                    if *stack >= $params {
                        choices.push(|_u, stack| {
                            *stack = *stack - $params + $results;
                            Ok(TableOp::$op $( ( $( <$imm>::arbitrary(_u)? ),* ) )? )
                        });
                    }
                )*

                // Choose a table op to insert.
                let f = u.choose(&choices)?;
                f(u, stack)
            }
        }
	};
}

define_table_ops! {
    Gc : 0 => 3,

    MakeRefs : 0 => 3,
    TakeRefs : 3 => 0,

    TableGet(i32) : 0 => 1,
    TableSet(i32) : 1 => 0,

    GlobalGet(u32) : 0 => 1,
    GlobalSet(u32) : 1 => 0,

    LocalGet(u32) : 0 => 1,
    LocalSet(u32) : 1 => 0,

    Drop : 1 => 0,

    Null : 0 => 1,
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

        let scratch_local = num_params;

        match self {
            Self::Gc => {
                func.instruction(&Instruction::Call(gc_func_idx));
            }
            Self::MakeRefs => {
                func.instruction(&Instruction::Call(make_refs_func_idx));
            }
            Self::TakeRefs => {
                func.instruction(&Instruction::Call(take_refs_func_idx));
            }
            Self::TableGet(x) => {
                func.instruction(&Instruction::I32Const(x % table_mod));
                func.instruction(&Instruction::TableGet { table: 0 });
            }
            Self::TableSet(x) => {
                func.instruction(&Instruction::LocalSet(scratch_local));
                func.instruction(&Instruction::I32Const(x % table_mod));
                func.instruction(&Instruction::LocalGet(scratch_local));
                func.instruction(&Instruction::TableSet { table: 0 });
            }
            Self::GlobalGet(x) => {
                func.instruction(&Instruction::GlobalGet(x % num_globals));
            }
            Self::GlobalSet(x) => {
                func.instruction(&Instruction::GlobalSet(x % num_globals));
            }
            Self::LocalGet(x) => {
                func.instruction(&Instruction::LocalGet(x % num_params));
            }
            Self::LocalSet(x) => {
                func.instruction(&Instruction::LocalSet(x % num_params));
            }
            Self::Drop => {
                func.instruction(&Instruction::Drop);
            }
            Self::Null => {
                func.instruction(&Instruction::RefNull(wasm_encoder::ValType::ExternRef));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::SmallRng;
    use rand::{RngCore, SeedableRng};

    #[test]
    fn test_valid() {
        let mut rng = SmallRng::seed_from_u64(0);
        let mut buf = vec![0; 2048];
        for _ in 0..1024 {
            rng.fill_bytes(&mut buf);
            let u = Unstructured::new(&buf);
            if let Ok(ops) = TableOps::arbitrary_take_rest(u) {
                let wasm = ops.to_wasm_binary();
                let mut validator =
                    wasmparser::Validator::new_with_features(wasmparser::WasmFeatures {
                        reference_types: true,
                        ..Default::default()
                    });
                let result = validator.validate_all(&wasm);
                assert!(result.is_ok());
            }
        }
    }

    #[test]
    fn test_wat_string() {
        let ops = TableOps {
            num_params: 10,
            num_globals: 10,
            table_size: 20,
            ops: vec![
                TableOp::Gc,
                TableOp::MakeRefs,
                TableOp::TakeRefs,
                TableOp::TableGet(0),
                TableOp::TableSet(1),
                TableOp::GlobalGet(2),
                TableOp::GlobalSet(3),
                TableOp::LocalGet(4),
                TableOp::LocalSet(5),
                TableOp::Drop,
                TableOp::Null,
            ],
        };

        let expected = r#"
(module
  (type (;0;) (func (result externref externref externref)))
  (type (;1;) (func (param externref externref externref externref externref externref externref externref externref externref)))
  (type (;2;) (func (param externref externref externref)))
  (type (;3;) (func (result externref externref externref)))
  (import "" "gc" (func (;0;) (type 0)))
  (import "" "take_refs" (func (;1;) (type 2)))
  (import "" "make_refs" (func (;2;) (type 3)))
  (func (;3;) (type 1) (param externref externref externref externref externref externref externref externref externref externref)
    (local externref)
    loop  ;; label = @1
      call 0
      call 2
      call 1
      i32.const 0
      table.get 0
      local.set 10
      i32.const 1
      local.get 10
      table.set 0
      global.get 2
      global.set 3
      local.get 4
      local.set 5
      drop
      ref.null extern
      br 0 (;@1;)
    end
  )
  (table (;0;) 20 externref)
  (global (;0;) (mut externref) ref.null extern)
  (global (;1;) (mut externref) ref.null extern)
  (global (;2;) (mut externref) ref.null extern)
  (global (;3;) (mut externref) ref.null extern)
  (global (;4;) (mut externref) ref.null extern)
  (global (;5;) (mut externref) ref.null extern)
  (global (;6;) (mut externref) ref.null extern)
  (global (;7;) (mut externref) ref.null extern)
  (global (;8;) (mut externref) ref.null extern)
  (global (;9;) (mut externref) ref.null extern)
  (export "run" (func 3))
)
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
