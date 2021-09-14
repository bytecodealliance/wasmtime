use super::{first_exported_function, first_exported_memory, log_wasm};
use rusty_v8 as v8;
use std::convert::TryFrom;
use std::sync::Once;
use wasmtime::*;

/// Performs differential execution between Wasmtime and V8.
///
/// This will instantiate the `wasm` provided, which should have no host
/// imports, and then run it in Wasmtime with the `config` specified and V8 with
/// default settings. The first export is executed and if memory is exported
/// it's compared as well.
///
/// Note that it's the caller's responsibility to ensure that the `wasm`
/// doesn't infinitely loop as no protections are done in v8 to prevent this
/// from happening.
pub fn differential_v8_execution(wasm: &[u8], config: &crate::generators::Config) -> Option<()> {
    // Wasmtime setup
    crate::init_fuzzing();
    log_wasm(wasm);
    let (wasmtime_module, mut wasmtime_store) = super::differential_store(wasm, config);
    log::trace!("compiled module with wasmtime");

    // V8 setup
    let mut isolate = isolate();
    let mut scope = v8::HandleScope::new(&mut *isolate);
    let context = v8::Context::new(&mut scope);
    let global = context.global(&mut scope);
    let mut scope = v8::ContextScope::new(&mut scope, context);

    // V8: compile module
    let buf = v8::ArrayBuffer::new_backing_store_from_boxed_slice(wasm.into());
    let buf = v8::SharedRef::from(buf);
    let name = v8::String::new(&mut scope, "WASM_BINARY").unwrap();
    let buf = v8::ArrayBuffer::with_backing_store(&mut scope, &buf);
    global.set(&mut scope, name.into(), buf.into());
    let v8_module = eval(&mut scope, "new WebAssembly.Module(WASM_BINARY)").unwrap();
    let name = v8::String::new(&mut scope, "WASM_MODULE").unwrap();
    global.set(&mut scope, name.into(), v8_module);
    log::trace!("compiled module with v8");

    // Wasmtime: instantiate
    let wasmtime_instance = wasmtime::Instance::new(&mut wasmtime_store, &wasmtime_module, &[]);
    log::trace!("instantiated with wasmtime");

    // V8: instantiate
    let v8_instance = eval(&mut scope, "new WebAssembly.Instance(WASM_MODULE)");
    log::trace!("instantiated with v8");

    // Verify V8 and wasmtime match
    let (wasmtime_instance, v8_instance) = match (wasmtime_instance, v8_instance) {
        (Ok(i1), Ok(i2)) => (i1, i2),
        (Ok(_), Err(msg)) => {
            panic!("wasmtime succeeded at instantiation, v8 failed: {}", msg)
        }
        (Err(err), Ok(_)) => {
            panic!("v8 succeeded at instantiation, wasmtime failed: {:?}", err)
        }
        (Err(err), Err(msg)) => {
            log::trace!("instantiations failed");
            assert_error_matches(&err, &msg);
            return None;
        }
    };
    log::trace!("instantiations were successful");

    let (func, ty) = first_exported_function(&wasmtime_module)?;

    // not supported yet in V8
    if ty.params().chain(ty.results()).any(|t| t == ValType::V128) {
        log::trace!("exported function uses v128, skipping");
        return None;
    }

    let mut wasmtime_params = Vec::new();
    let mut v8_params = Vec::new();
    for param in ty.params() {
        wasmtime_params.push(match param {
            ValType::I32 => Val::I32(0),
            ValType::I64 => Val::I64(0),
            ValType::F32 => Val::F32(0),
            ValType::F64 => Val::F64(0),
            _ => unimplemented!(),
        });
        v8_params.push(match param {
            ValType::I32 | ValType::F32 | ValType::F64 => v8::Number::new(&mut scope, 0.0).into(),
            ValType::I64 => v8::BigInt::new_from_i64(&mut scope, 0).into(),
            _ => unimplemented!(),
        });
    }

    // Wasmtime: call the first exported func
    let wasmtime_main = wasmtime_instance
        .get_func(&mut wasmtime_store, func)
        .expect("function export is present");
    let wasmtime_vals = wasmtime_main.call(&mut wasmtime_store, &wasmtime_params);
    log::trace!("finished wasmtime invocation");

    // V8: call the first exported func
    let name = v8::String::new(&mut scope, "WASM_INSTANCE").unwrap();
    global.set(&mut scope, name.into(), v8_instance);
    let name = v8::String::new(&mut scope, "EXPORT_NAME").unwrap();
    let func_name = v8::String::new(&mut scope, func).unwrap();
    global.set(&mut scope, name.into(), func_name.into());
    let name = v8::String::new(&mut scope, "ARGS").unwrap();
    let v8_params = v8::Array::new_with_elements(&mut scope, &v8_params);
    global.set(&mut scope, name.into(), v8_params.into());
    let v8_vals = eval(
        &mut scope,
        &format!("WASM_INSTANCE.exports[EXPORT_NAME](...ARGS)"),
    );
    log::trace!("finished v8 invocation");

    // Verify V8 and wasmtime match
    match (wasmtime_vals, v8_vals) {
        (Ok(wasmtime), Ok(v8)) => {
            log::trace!("both executed successfully");
            match wasmtime.len() {
                0 => assert!(v8.is_undefined()),
                1 => assert_val_match(&wasmtime[0], &v8, &mut scope),
                _ => {
                    let array = v8::Local::<'_, v8::Array>::try_from(v8).unwrap();
                    for (i, wasmtime) in wasmtime.iter().enumerate() {
                        let v8 = array.get_index(&mut scope, i as u32).unwrap();
                        assert_val_match(wasmtime, &v8, &mut scope);
                        // ..
                    }
                }
            }
        }
        (Ok(_), Err(msg)) => {
            panic!("wasmtime succeeded at invocation, v8 failed: {}", msg)
        }
        (Err(err), Ok(_)) => {
            panic!("v8 succeeded at invocation, wasmtime failed: {:?}", err)
        }
        (Err(err), Err(msg)) => {
            log::trace!("got two traps");
            assert_error_matches(&err, &msg);
            return Some(());
        }
    };

    // Verify V8 and wasmtime match memories
    if let Some(mem) = first_exported_memory(&wasmtime_module) {
        log::trace!("comparing memories");
        let wasmtime = wasmtime_instance
            .get_memory(&mut wasmtime_store, mem)
            .unwrap();

        let name = v8::String::new(&mut scope, "MEMORY_NAME").unwrap();
        let func_name = v8::String::new(&mut scope, mem).unwrap();
        global.set(&mut scope, name.into(), func_name.into());
        let v8 = eval(
            &mut scope,
            &format!("WASM_INSTANCE.exports[MEMORY_NAME].buffer"),
        )
        .unwrap();
        let v8 = v8::Local::<'_, v8::ArrayBuffer>::try_from(v8).unwrap();
        let v8_data = v8.get_backing_store();
        let wasmtime_data = wasmtime.data(&wasmtime_store);
        assert_eq!(wasmtime_data.len(), v8_data.len());
        for i in 0..v8_data.len() {
            if wasmtime_data[i] != v8_data[i].get() {
                panic!("memories differ");
            }
        }
    }

    Some(())
}

/// Manufactures a new V8 Isolate to run within.
fn isolate() -> v8::OwnedIsolate {
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();
    });

    v8::Isolate::new(Default::default())
}

/// Evaluates the JS `code` within `scope`, returning either the result of the
/// computation or the stringified exception if one happened.
fn eval<'s>(
    scope: &mut v8::HandleScope<'s>,
    code: &str,
) -> Result<v8::Local<'s, v8::Value>, String> {
    let mut tc = v8::TryCatch::new(scope);
    let mut scope = v8::EscapableHandleScope::new(&mut tc);
    let source = v8::String::new(&mut scope, code).unwrap();
    let script = v8::Script::compile(&mut scope, source, None).unwrap();
    match script.run(&mut scope) {
        Some(val) => Ok(scope.escape(val)),
        None => {
            drop(scope);
            assert!(tc.has_caught());
            Err(tc
                .message()
                .unwrap()
                .get(&mut tc)
                .to_rust_string_lossy(&mut tc))
        }
    }
}

/// Asserts that the wasmtime value `a` matches the v8 value `b`.
///
/// For NaN values simply just asserts that they're both NaN.
fn assert_val_match(a: &Val, b: &v8::Local<'_, v8::Value>, scope: &mut v8::HandleScope<'_>) {
    match *a {
        Val::I32(wasmtime) => {
            assert_eq!(i64::from(wasmtime), b.to_int32(scope).unwrap().value());
        }
        Val::I64(wasmtime) => {
            assert_eq!((wasmtime, true), b.to_big_int(scope).unwrap().i64_value());
        }
        Val::F32(wasmtime) => {
            same_float(
                f64::from(f32::from_bits(wasmtime)),
                b.to_number(scope).unwrap().value(),
            );
        }
        Val::F64(wasmtime) => {
            same_float(
                f64::from_bits(wasmtime),
                b.to_number(scope).unwrap().value(),
            );
        }
        _ => panic!("unsupported match {:?}", a),
    }

    fn same_float(a: f64, b: f64) {
        assert!(a == b || (a.is_nan() && b.is_nan()), "{} != {}", a, b);
    }
}

/// Attempts to assert that the `wasmtime` error matches the `v8` error string.
///
/// This is not a precise function. This will likely need updates over time as
/// v8 and/or wasmtime changes. The goal here is to generally make sure that
/// both engines fail for basically the same reason.
fn assert_error_matches(wasmtime: &anyhow::Error, v8: &str) {
    let wasmtime_msg = match wasmtime.downcast_ref::<Trap>() {
        Some(trap) => trap.display_reason().to_string(),
        None => format!("{:?}", wasmtime),
    };
    let verify_wasmtime = |msg: &str| {
        assert!(wasmtime_msg.contains(msg), "{}\n!=\n{}", wasmtime_msg, v8);
    };
    let verify_v8 = |msg: &[&str]| {
        assert!(
            msg.iter().any(|msg| v8.contains(msg)),
            "{:?}\n\t!=\n{}",
            wasmtime_msg,
            v8
        );
    };
    if let Some(code) = wasmtime.downcast_ref::<Trap>().and_then(|t| t.trap_code()) {
        match code {
            TrapCode::MemoryOutOfBounds => {
                return verify_v8(&[
                    "memory access out of bounds",
                    "data segment is out of bounds",
                ])
            }
            TrapCode::UnreachableCodeReached => {
                return verify_v8(&[
                    "unreachable",
                    // All the wasms we test use wasm-smith's
                    // `ensure_termination` option which will `unreachable` when
                    // "fuel" runs out within the wasm module itself. This
                    // sometimes manifests as a call stack size exceeded in v8,
                    // however, since v8 sometimes has different limits on the
                    // call-stack especially when it's run multiple times. To
                    // get these error messages to line up allow v8 to say the
                    // call stack size exceeded when wasmtime says we hit
                    // unreachable.
                    "Maximum call stack size exceeded",
                ]);
            }
            TrapCode::IntegerDivisionByZero => {
                return verify_v8(&["divide by zero", "remainder by zero"])
            }
            TrapCode::StackOverflow => return verify_v8(&["call stack size exceeded"]),
            TrapCode::IndirectCallToNull => return verify_v8(&["null function"]),
            TrapCode::TableOutOfBounds => {
                return verify_v8(&[
                    "table initializer is out of bounds",
                    "table index is out of bounds",
                ])
            }
            TrapCode::BadSignature => return verify_v8(&["function signature mismatch"]),
            TrapCode::IntegerOverflow | TrapCode::BadConversionToInteger => {
                return verify_v8(&[
                    "float unrepresentable in integer range",
                    "divide result unrepresentable",
                ])
            }
            other => log::debug!("unknown code {:?}", other),
        }
    }
    verify_wasmtime("not possibly present in an error, just panic please");
}
