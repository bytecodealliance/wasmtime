use crate::host::{
    argv_environ_init, argv_environ_values, fd_prestats, fd_prestats_init, fd_prestats_insert,
    fd_table, fd_table_init, fd_table_insert_existing,
};
use crate::syscalls;
use cranelift_codegen::ir::types;
use cranelift_codegen::{ir, isa};
use cranelift_entity::PrimaryMap;
use cranelift_wasm::DefinedFuncIndex;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CString;
use std::fs::File;
use std::mem;
use std::os::unix::io::AsRawFd;
use std::rc::Rc;
use target_lexicon::HOST;
use wasmtime_environ::{translate_signature, Export, Module};
use wasmtime_runtime::{Imports, InstanceHandle, InstantiationError, VMFunctionBody};

pub(crate) struct WASIState {
    pub curfds: Box<fd_table>,
    pub prestats: Box<fd_prestats>,
    pub argv_environ: Box<argv_environ_values>,
}

/// Return an instance implementing the "wasi" interface.
pub fn instantiate_wasi_c(
    prefix: &str,
    global_exports: Rc<RefCell<HashMap<String, Option<wasmtime_runtime::Export>>>>,
    preopened_dirs: &[(String, File)],
    argv: &[String],
    environ: &[(String, String)],
) -> Result<InstanceHandle, InstantiationError> {
    let pointer_type = types::Type::triple_pointer_type(&HOST);
    let mut module = Module::new();
    let mut finished_functions: PrimaryMap<DefinedFuncIndex, *const VMFunctionBody> =
        PrimaryMap::new();
    let call_conv = isa::CallConv::triple_default(&HOST);

    macro_rules! signature {
        ($name:ident) => {{
            let sig = module.signatures.push(translate_signature(
                ir::Signature {
                    params: syscalls::$name::params()
                        .into_iter()
                        .map(ir::AbiParam::new)
                        .collect(),
                    returns: syscalls::$name::results()
                        .into_iter()
                        .map(ir::AbiParam::new)
                        .collect(),
                    call_conv,
                },
                pointer_type,
            ));
            let func = module.functions.push(sig);
            module.exports.insert(
                prefix.to_owned() + stringify!($name),
                Export::Function(func),
            );
            finished_functions.push(syscalls::$name::SHIM as *const VMFunctionBody);
        }};
    }

    signature!(args_get);
    signature!(args_sizes_get);
    signature!(clock_res_get);
    signature!(clock_time_get);
    signature!(environ_get);
    signature!(environ_sizes_get);
    signature!(fd_prestat_get);
    signature!(fd_prestat_dir_name);
    signature!(fd_close);
    signature!(fd_datasync);
    signature!(fd_pread);
    signature!(fd_pwrite);
    signature!(fd_read);
    signature!(fd_renumber);
    signature!(fd_seek);
    signature!(fd_tell);
    signature!(fd_fdstat_get);
    signature!(fd_fdstat_set_flags);
    signature!(fd_fdstat_set_rights);
    signature!(fd_sync);
    signature!(fd_write);
    signature!(fd_advise);
    signature!(fd_allocate);
    signature!(path_create_directory);
    signature!(path_link);
    signature!(path_open);
    signature!(fd_readdir);
    signature!(path_readlink);
    signature!(path_rename);
    signature!(fd_filestat_get);
    signature!(fd_filestat_set_times);
    signature!(fd_filestat_set_size);
    signature!(path_filestat_get);
    signature!(path_filestat_set_times);
    signature!(path_symlink);
    signature!(path_unlink_file);
    signature!(path_remove_directory);
    signature!(poll_oneoff);
    signature!(proc_exit);
    signature!(proc_raise);
    signature!(random_get);
    signature!(sched_yield);
    signature!(sock_recv);
    signature!(sock_send);
    signature!(sock_shutdown);

    let imports = Imports::none();
    let data_initializers = Vec::new();
    let signatures = PrimaryMap::new();
    let mut curfds = Box::new(unsafe { mem::zeroed::<fd_table>() });
    let mut prestats = Box::new(unsafe { mem::zeroed::<fd_prestats>() });
    let mut argv_environ = Box::new(unsafe { mem::zeroed::<argv_environ_values>() });

    unsafe {
        let argv_environ: &mut argv_environ_values = &mut *argv_environ;
        let (argv_offsets, argv_buf, environ_offsets, environ_buf) =
            allocate_argv_environ(argv, environ);
        argv_environ_init(
            argv_environ,
            argv_offsets.as_ptr(),
            argv_offsets.len(),
            argv_buf.as_ptr(),
            argv_buf.len(),
            environ_offsets.as_ptr(),
            environ_offsets.len(),
            environ_buf.as_ptr(),
            environ_buf.len(),
        );

        let curfds: *mut fd_table = &mut *curfds;
        fd_table_init(curfds);

        let prestats: *mut fd_prestats = &mut *prestats;
        fd_prestats_init(prestats);

        // Prepopulate curfds with stdin, stdout, and stderr file descriptors.
        assert!(fd_table_insert_existing(curfds, 0, 0));
        assert!(fd_table_insert_existing(curfds, 1, 1));
        assert!(fd_table_insert_existing(curfds, 2, 2));

        let mut wasm_fd = 3;
        for (dir, file) in preopened_dirs {
            assert!(fd_table_insert_existing(curfds, wasm_fd, file.as_raw_fd()));
            let dir_cstr = CString::new(dir.as_str()).unwrap();
            assert!(fd_prestats_insert(prestats, dir_cstr.as_ptr(), wasm_fd));
            wasm_fd += 1;
        }
    }

    let host_state = WASIState {
        curfds,
        prestats,
        argv_environ,
    };

    InstanceHandle::new(
        Rc::new(module),
        global_exports,
        finished_functions.into_boxed_slice(),
        imports,
        &data_initializers,
        signatures.into_boxed_slice(),
        None,
        Box::new(host_state),
    )
}

fn allocate_argv_environ(
    argv: &[String],
    environ: &[(String, String)],
) -> (Vec<usize>, Vec<libc::c_char>, Vec<usize>, Vec<libc::c_char>) {
    let mut argv_offsets = Vec::new();
    let mut argv_buf = Vec::new();
    let mut environ_offsets = Vec::new();
    let mut environ_buf = Vec::new();

    for arg in argv {
        argv_offsets.push(argv_buf.len());
        for c in arg.bytes() {
            argv_buf.push(c as libc::c_char);
        }
        argv_buf.push('\0' as libc::c_char);
    }
    for (key, value) in environ {
        environ_offsets.push(environ_buf.len());
        for c in key.bytes() {
            environ_buf.push(c as libc::c_char);
        }
        environ_buf.push('=' as libc::c_char);
        for c in value.bytes() {
            environ_buf.push(c as libc::c_char);
        }
        environ_buf.push('\0' as libc::c_char);
    }

    (argv_offsets, argv_buf, environ_offsets, environ_buf)
}
