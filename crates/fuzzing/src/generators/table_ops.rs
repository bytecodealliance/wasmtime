//! Generating series of `table.get` and `table.set` operations.

use mutatis::{Generate, DefaultMutate, Candidates, Mutate, Context, Result as MutResult};
use mutatis::mutators as m;
use std::ops::RangeInclusive;
use wasm_encoder::{
    CodeSection, ConstExpr, EntityType, ExportKind, ExportSection, Function, FunctionSection,
    GlobalSection, ImportSection, Instruction, Module, RefType, TableSection, TableType,
    TypeSection, ValType,
};

/// A description of a Wasm module that makes a series of `externref` table
/// operations.
#[derive(Debug)]
pub struct TableOps {
    pub(crate) num_params: u32,
    pub(crate) num_globals: u32,
    pub(crate) table_size: i32,
    ops: Vec<TableOp>,
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
}

#[derive(Debug)]
pub struct TableOpsMutator;

impl Mutate<TableOps> for TableOpsMutator {
    fn mutate(&mut self, c: &mut Candidates<'_>, ops: &mut TableOps) -> mutatis::Result<()> {
        c.mutation(|ctx| {
            let mut stack = 0;
            add_table_op_mutatis(ops, ctx, &mut stack)
        })?;
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

impl Generate<TableOps> for TableOpsMutator {
    fn generate(&mut self, ctx: &mut Context) -> MutResult<TableOps> {
        let num_params = m::range(NUM_PARAMS_RANGE).generate(ctx)?;
        let num_globals = m::range(NUM_GLOBALS_RANGE).generate(ctx)?;
        let table_size = m::range(TABLE_SIZE_RANGE).generate(ctx)?;
        let mut ops = Vec::new();
        let mut stack = 0u32;
        let mut temp_ops = TableOps {
            num_params,
            num_globals,
            table_size,
            ops: vec![
                TableOp::Null,
                TableOp::Drop,
                TableOp::Gc,
                TableOp::LocalSet(0),
                TableOp::LocalGet(0),
                TableOp::GlobalSet(0),
                TableOp::GlobalGet(0),
            ],
        };
        while ops.len() < MAX_OPS {
            temp_ops.ops = ops.clone(); 
            let add_result = add_table_op_mutatis(&mut temp_ops, ctx, &mut stack);
            if let Ok(()) = add_result {
                if let Some(last) = temp_ops.ops.last() {
                    ops.push(*last);
                }
            } else {
                break;
            }
        }
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
            $op:ident $( ( $($limit:expr => $ty:ty),* ) )? : $params:expr => $results:expr ,
        )*
    ) => {
        #[derive(Copy, Clone, Debug)]
        pub(crate) enum TableOp {
            $(
                $op $( ( $($ty),* ) )?,
            )*
        }
        #[expect(unused_comparisons)]
        fn add_table_op_mutatis(
            ops: &mut TableOps,
            ctx: &mut mutatis::Context,
            stack: &mut u32,
        ) -> mutatis::Result<()> {
            use mutatis::Generate;
            use mutatis::mutators as m;

            let mut valid_choices = vec![];

            $(
                if $( $(($limit as fn(&TableOps) -> $ty)(ops) > 0 &&)* )? *stack >= $params {
                    valid_choices.push(stringify!($op));
                }
            )*
            let selected = ctx.rng().choose(&valid_choices).unwrap();
            let op = match *selected {
                $(
                    stringify!($op) => {
                        *stack = *stack - $params + $results;
                        TableOp::$op $(
                            (
                                $(
                                    m::range(0..=($limit as fn(&TableOps) -> $ty)(ops) - 1).generate(ctx)?,
                                )*
                            )
                        )?
                    }
                )*
                _ => unreachable!(),
            };

            ops.ops.push(op);
            Ok(())
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
            Self::Drop => {
                func.instruction(&Instruction::Drop);
            }
            Self::Null => {
                func.instruction(&Instruction::RefNull(wasm_encoder::HeapType::EXTERN));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! default_table_ops {
        ($num_params:expr, $num_globals:expr, $table_size:expr) => {
            TableOps {
                num_params: $num_params,
                num_globals: $num_globals,
                table_size: $table_size,
                ops: vec![
                    TableOp::Null,
                    TableOp::Drop,
                    TableOp::Gc,
                    TableOp::LocalSet(0),
                    TableOp::LocalGet(0),
                    TableOp::GlobalSet(0),
                    TableOp::GlobalGet(0),
                ],
            }
        };
    }    
    
    #[test]
    fn mutate_table_ops_with_default_mutator() -> mutatis::Result<()> {
        use mutatis::Session;
        use wasmparser::Validator;
        let mut res = default_table_ops![5, 5, 5];
        let mut session = Session::new();
    
        for _ in 0..10 {
            session.mutate(&mut res)?;
            let wasm = res.to_wasm_binary();
            let mut validator = Validator::new();
            let wat = wasmprinter::print_bytes(&wasm).unwrap();
            let result = validator.validate_all(&wasm);
            println!("{wat}");
            assert!(result.is_ok());
        }
        Ok(())
    }    

    #[test]
    fn test_tableops_mutate_with() -> mutatis::Result<()> {
        let mut res = default_table_ops![5, 5, 5];
        let mut generator = TableOpsMutator;
        let mut session = mutatis::Session::new();
        for _ in 0..=10 {
            session.mutate_with(&mut generator, &mut res)?;
            let wasm = res.to_wasm_binary();
            let mut validator = wasmparser::Validator::new();
            let result = validator.validate_all(&wasm);
            let wat = wasmprinter::print_bytes(&wasm).unwrap();
            println!("{wat}");
            assert!(result.is_ok());
        }
        Ok(())
    }    
}