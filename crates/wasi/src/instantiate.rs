use cranelift_codegen::ir::types;
use cranelift_codegen::{ir, isa};
use cranelift_entity::PrimaryMap;
use cranelift_wasm::DefinedFuncIndex;
use std::fs::File;
use std::rc::Rc;
use target_lexicon::HOST;
use wasi_common::wasi;
use wasi_common::{WasiCtx, WasiCtxBuilder};
use wasmtime_environ::{translate_signature, Export, Module};
use wasmtime_runtime::{Imports, InstanceHandle, InstantiationError, VMContext};

/// Creates `wasmtime::Instance` object implementing the "wasi" interface.
pub fn create_wasi_instance(
    store: &wasmtime::Store,
    preopened_dirs: &[(String, File)],
    argv: &[String],
    environ: &[(String, String)],
) -> Result<wasmtime::Instance, InstantiationError> {
    let wasi = instantiate_wasi(preopened_dirs, argv, environ)?;
    let instance = wasmtime::Instance::from_handle(&store, wasi);
    Ok(instance)
}

/// Return an instance implementing the "wasi" interface.
pub fn instantiate_wasi(
    preopened_dirs: &[(String, File)],
    argv: &[String],
    environ: &[(String, String)],
) -> Result<InstanceHandle, InstantiationError> {
    let mut wasi_ctx_builder = WasiCtxBuilder::new()
        .inherit_stdio()
        .args(argv)
        .envs(environ);

    for (dir, f) in preopened_dirs {
        wasi_ctx_builder = wasi_ctx_builder.preopened_dir(
            f.try_clone().map_err(|err| {
                InstantiationError::Resource(format!(
                    "couldn't clone an instance handle to pre-opened dir: {}",
                    err
                ))
            })?,
            dir,
        );
    }

    let wasi_ctx = wasi_ctx_builder.build().map_err(|err| {
        InstantiationError::Resource(format!("couldn't assemble WASI context object: {}", err))
    })?;
    instantiate_wasi_with_context(wasi_ctx)
}

/// Return an instance implementing the "wasi" interface.
///
/// The wasi context is configured by
pub fn instantiate_wasi_with_context(
    wasi_ctx: WasiCtx,
) -> Result<InstanceHandle, InstantiationError> {
    let pointer_type = types::Type::triple_pointer_type(&HOST);
    let mut module = Module::new();
    let mut finished_functions = PrimaryMap::new();
    let call_conv = isa::CallConv::triple_default(&HOST);

    // This function is defined in the macro invocation of
    // `define_add_wrappers_to_module` below. For more information about how
    // this works it'd recommended to read the source in
    // `crates/wasi-common/wig/src/wasi.rs`.
    add_wrappers_to_module(
        &mut module,
        &mut finished_functions,
        call_conv,
        pointer_type,
    );

    let imports = Imports::none();
    let data_initializers = Vec::new();
    let signatures = PrimaryMap::new();

    InstanceHandle::new(
        Rc::new(module),
        finished_functions.into_boxed_slice(),
        imports,
        &data_initializers,
        signatures.into_boxed_slice(),
        None,
        Box::new(wasi_ctx),
    )
}

wig::define_add_wrappers_to_module!(
    "snapshot" "wasi_snapshot_preview1"
);

// Used by `add_wrappers_to_module` defined in the macro above
fn get_wasi_ctx(vmctx: &mut VMContext) -> Result<&mut WasiCtx, wasi::__wasi_errno_t> {
    unsafe {
        vmctx
            .host_state()
            .downcast_mut::<WasiCtx>()
            .ok_or_else(|| panic!("no host state named WasiCtx available"))
    }
}

// Used by `add_wrappers_to_module` defined in the macro above
fn get_memory(caller_vmctx: &mut VMContext) -> Result<&mut [u8], wasi::__wasi_errno_t> {
    match unsafe { InstanceHandle::from_vmctx(caller_vmctx) }.lookup("memory") {
        Some(wasmtime_runtime::Export::Memory {
            definition,
            vmctx: _,
            memory: _,
        }) => unsafe {
            let definition = &*definition;
            let ptr = definition.base;
            let len = definition.current_length;
            Ok(std::slice::from_raw_parts_mut(ptr, len))
        },
        Some(export) => {
            log::error!("export named \"memory\" isn't a memory: {:?}", export);
            Err(wasi::__WASI_ERRNO_INVAL)
        }
        None => {
            log::error!("no export named \"memory\" available from caller");
            Err(wasi::__WASI_ERRNO_INVAL)
        }
    }
}
