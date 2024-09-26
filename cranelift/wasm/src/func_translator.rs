//! Stand-alone WebAssembly to Cranelift IR translator.
//!
//! This module defines the `FuncTranslator` type which can translate a single WebAssembly
//! function to Cranelift IR guided by a `FuncEnvironment` which provides information about the
//! WebAssembly module and the runtime environment.

use crate::code_translator::{bitcast_wasm_returns, translate_operator};
use crate::environ::FuncEnvironment;
use crate::state::FuncTranslationState;
use crate::translation_utils::get_vmctx_value_label;
use crate::WasmResult;
use cranelift_codegen::entity::EntityRef;
use cranelift_codegen::ir::{self, Block, InstBuilder, ValueLabel};
use cranelift_codegen::timing;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use wasmparser::{BinaryReader, FuncValidator, FunctionBody, WasmModuleResources};

/// WebAssembly to Cranelift IR function translator.
///
/// A `FuncTranslator` is used to translate a binary WebAssembly function into Cranelift IR guided
/// by a `FuncEnvironment` object. A single translator instance can be reused to translate multiple
/// functions which will reduce heap allocation traffic.
pub struct FuncTranslator {
    func_ctx: FunctionBuilderContext,
    state: FuncTranslationState,
}

impl FuncTranslator {
    /// Create a new translator.
    pub fn new() -> Self {
        Self {
            func_ctx: FunctionBuilderContext::new(),
            state: FuncTranslationState::new(),
        }
    }

    /// Returns the underlying `FunctionBuilderContext` that this translator
    /// uses.
    pub fn context(&mut self) -> &mut FunctionBuilderContext {
        &mut self.func_ctx
    }

    /// Translate a binary WebAssembly function from a `FunctionBody`.
    ///
    /// See [the WebAssembly specification][wasm].
    ///
    /// [wasm]: https://webassembly.github.io/spec/core/binary/modules.html#code-section
    ///
    /// The Cranelift IR function `func` should be completely empty except for the `func.signature`
    /// and `func.name` fields. The signature may contain special-purpose arguments which are not
    /// regarded as WebAssembly local variables. Any signature arguments marked as
    /// `ArgumentPurpose::Normal` are made accessible as WebAssembly local variables.
    pub fn translate_body<FE: FuncEnvironment + ?Sized>(
        &mut self,
        validator: &mut FuncValidator<impl WasmModuleResources>,
        body: FunctionBody<'_>,
        func: &mut ir::Function,
        environ: &mut FE,
    ) -> WasmResult<()> {
        let _tt = timing::wasm_translate_function();
        let mut reader = body.get_binary_reader();
        log::trace!(
            "translate({} bytes, {}{})",
            reader.bytes_remaining(),
            func.name,
            func.signature
        );
        debug_assert_eq!(func.dfg.num_blocks(), 0, "Function must be empty");
        debug_assert_eq!(func.dfg.num_insts(), 0, "Function must be empty");

        let mut builder = FunctionBuilder::new(func, &mut self.func_ctx);
        builder.set_srcloc(cur_srcloc(&reader));
        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block); // Declare all predecessors known.

        // Make sure the entry block is inserted in the layout before we make any callbacks to
        // `environ`. The callback functions may need to insert things in the entry block.
        builder.ensure_inserted_block();

        let num_params = declare_wasm_parameters(&mut builder, entry_block, environ);

        // Set up the translation state with a single pushed control block representing the whole
        // function and its return values.
        let exit_block = builder.create_block();
        builder.append_block_params_for_function_returns(exit_block);
        self.state.initialize(&builder.func.signature, exit_block);

        parse_local_decls(&mut reader, &mut builder, num_params, environ, validator)?;
        parse_function_body(validator, reader, &mut builder, &mut self.state, environ)?;

        builder.finalize();
        log::trace!("translated Wasm to CLIF:\n{}", func.display());
        Ok(())
    }
}

/// Declare local variables for the signature parameters that correspond to WebAssembly locals.
///
/// Return the number of local variables declared.
fn declare_wasm_parameters<FE: FuncEnvironment + ?Sized>(
    builder: &mut FunctionBuilder,
    entry_block: Block,
    environ: &FE,
) -> usize {
    let sig_len = builder.func.signature.params.len();
    let mut next_local = 0;
    for i in 0..sig_len {
        let param_type = builder.func.signature.params[i];
        // There may be additional special-purpose parameters in addition to the normal WebAssembly
        // signature parameters. For example, a `vmctx` pointer.
        if environ.is_wasm_parameter(&builder.func.signature, i) {
            // This is a normal WebAssembly signature parameter, so create a local for it.
            let local = Variable::new(next_local);
            builder.declare_var(local, param_type.value_type);
            next_local += 1;

            if environ.param_needs_stack_map(&builder.func.signature, i) {
                builder.declare_var_needs_stack_map(local);
            }

            let param_value = builder.block_params(entry_block)[i];
            builder.def_var(local, param_value);
        }
        if param_type.purpose == ir::ArgumentPurpose::VMContext {
            let param_value = builder.block_params(entry_block)[i];
            builder.set_val_label(param_value, get_vmctx_value_label());
        }
    }

    next_local
}

/// Parse the local variable declarations that precede the function body.
///
/// Declare local variables, starting from `num_params`.
fn parse_local_decls<FE: FuncEnvironment + ?Sized>(
    reader: &mut BinaryReader,
    builder: &mut FunctionBuilder,
    num_params: usize,
    environ: &mut FE,
    validator: &mut FuncValidator<impl WasmModuleResources>,
) -> WasmResult<()> {
    let mut next_local = num_params;
    let local_count = reader.read_var_u32()?;

    for _ in 0..local_count {
        builder.set_srcloc(cur_srcloc(reader));
        let pos = reader.original_position();
        let count = reader.read_var_u32()?;
        let ty = reader.read()?;
        validator.define_locals(pos, count, ty)?;
        declare_locals(builder, count, ty, &mut next_local, environ)?;
    }

    environ.after_locals(next_local);

    Ok(())
}

/// Declare `count` local variables of the same type, starting from `next_local`.
///
/// Fail if too many locals are declared in the function, or if the type is not valid for a local.
fn declare_locals<FE: FuncEnvironment + ?Sized>(
    builder: &mut FunctionBuilder,
    count: u32,
    wasm_type: wasmparser::ValType,
    next_local: &mut usize,
    environ: &mut FE,
) -> WasmResult<()> {
    // All locals are initialized to 0.
    use wasmparser::ValType::*;
    let (ty, init, needs_stack_map) = match wasm_type {
        I32 => (
            ir::types::I32,
            Some(builder.ins().iconst(ir::types::I32, 0)),
            false,
        ),
        I64 => (
            ir::types::I64,
            Some(builder.ins().iconst(ir::types::I64, 0)),
            false,
        ),
        F32 => (
            ir::types::F32,
            Some(builder.ins().f32const(ir::immediates::Ieee32::with_bits(0))),
            false,
        ),
        F64 => (
            ir::types::F64,
            Some(builder.ins().f64const(ir::immediates::Ieee64::with_bits(0))),
            false,
        ),
        V128 => {
            let constant_handle = builder.func.dfg.constants.insert([0; 16].to_vec().into());
            (
                ir::types::I8X16,
                Some(builder.ins().vconst(ir::types::I8X16, constant_handle)),
                false,
            )
        }
        Ref(rt) => {
            let hty = environ.convert_heap_type(rt.heap_type());
            let (ty, needs_stack_map) = environ.reference_type(hty);
            let init = if rt.is_nullable() {
                Some(environ.translate_ref_null(builder.cursor(), hty)?)
            } else {
                None
            };
            (ty, init, needs_stack_map)
        }
    };

    for _ in 0..count {
        let local = Variable::new(*next_local);
        builder.declare_var(local, ty);
        if needs_stack_map {
            builder.declare_var_needs_stack_map(local);
        }
        if let Some(init) = init {
            builder.def_var(local, init);
            builder.set_val_label(init, ValueLabel::new(*next_local));
        }
        *next_local += 1;
    }
    Ok(())
}

/// Parse the function body in `reader`.
///
/// This assumes that the local variable declarations have already been parsed and function
/// arguments and locals are declared in the builder.
fn parse_function_body<FE: FuncEnvironment + ?Sized>(
    validator: &mut FuncValidator<impl WasmModuleResources>,
    mut reader: BinaryReader,
    builder: &mut FunctionBuilder,
    state: &mut FuncTranslationState,
    environ: &mut FE,
) -> WasmResult<()> {
    // The control stack is initialized with a single block representing the whole function.
    debug_assert_eq!(state.control_stack.len(), 1, "State not initialized");

    environ.before_translate_function(builder, state)?;
    while !reader.eof() {
        let pos = reader.original_position();
        builder.set_srcloc(cur_srcloc(&reader));
        let op = reader.read_operator()?;
        validator.op(pos, &op)?;
        environ.before_translate_operator(&op, builder, state)?;
        translate_operator(validator, &op, builder, state, environ)?;
        environ.after_translate_operator(&op, builder, state)?;
    }
    let pos = reader.original_position();
    validator.finish(pos)?;

    // The final `End` operator left us in the exit block where we need to manually add a return
    // instruction.
    //
    // If the exit block is unreachable, it may not have the correct arguments, so we would
    // generate a return instruction that doesn't match the signature.
    if state.reachable {
        if !builder.is_unreachable() {
            environ.handle_before_return(&state.stack, builder);
            bitcast_wasm_returns(environ, &mut state.stack, builder);
            builder.ins().return_(&state.stack);
        }
    }

    // Discard any remaining values on the stack. Either we just returned them,
    // or the end of the function is unreachable.
    state.stack.clear();

    Ok(())
}

/// Get the current source location from a reader.
fn cur_srcloc(reader: &BinaryReader) -> ir::SourceLoc {
    // We record source locations as byte code offsets relative to the beginning of the file.
    // This will wrap around if byte code is larger than 4 GB.
    ir::SourceLoc::new(reader.original_position() as u32)
}
