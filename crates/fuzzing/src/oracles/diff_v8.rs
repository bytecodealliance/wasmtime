use crate::generators::{Config, DiffValue, DiffValueType};
use crate::oracles::engine::{DiffEngine, DiffInstance};
use anyhow::{bail, Error, Result};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Once;
use wasmtime::Trap;

pub struct V8Engine {
    isolate: Rc<RefCell<v8::OwnedIsolate>>,
}

impl V8Engine {
    pub fn new(config: &mut Config) -> V8Engine {
        static INIT: Once = Once::new();

        INIT.call_once(|| {
            let platform = v8::new_default_platform(0, false).make_shared();
            v8::V8::initialize_platform(platform);
            v8::V8::initialize();
        });

        let config = &mut config.module_config.config;
        // FIXME: reference types are disabled for now as we seemingly keep finding
        // a segfault in v8. This is found relatively quickly locally and keeps
        // getting found by oss-fuzz and currently we don't think that there's
        // really much we can do about it. For the time being disable reference
        // types entirely. An example bug is
        // https://bugs.chromium.org/p/oss-fuzz/issues/detail?id=45662
        config.reference_types_enabled = false;

        config.min_memories = config.min_memories.min(1);
        config.max_memories = config.max_memories.min(1);
        config.memory64_enabled = false;
        config.custom_page_sizes_enabled = false;
        config.wide_arithmetic_enabled = false;

        Self {
            isolate: Rc::new(RefCell::new(v8::Isolate::new(Default::default()))),
        }
    }
}

impl DiffEngine for V8Engine {
    fn name(&self) -> &'static str {
        "v8"
    }

    fn instantiate(&mut self, wasm: &[u8]) -> Result<Box<dyn DiffInstance>> {
        // Setup a new `Context` in which we'll be creating this instance and
        // executing code.
        let mut isolate = self.isolate.borrow_mut();
        let isolate = &mut **isolate;
        let mut scope = v8::HandleScope::new(isolate);
        let context = v8::Context::new(&mut scope, Default::default());
        let global = context.global(&mut scope);
        let mut scope = v8::ContextScope::new(&mut scope, context);

        // Move the `wasm` into JS and then invoke `new WebAssembly.Module`.
        let buf = v8::ArrayBuffer::new_backing_store_from_boxed_slice(wasm.into());
        let buf = v8::SharedRef::from(buf);
        let name = v8::String::new(&mut scope, "WASM_BINARY").unwrap();
        let buf = v8::ArrayBuffer::with_backing_store(&mut scope, &buf);
        global.set(&mut scope, name.into(), buf.into());
        let module = eval(&mut scope, "new WebAssembly.Module(WASM_BINARY)").unwrap();
        let name = v8::String::new(&mut scope, "WASM_MODULE").unwrap();
        global.set(&mut scope, name.into(), module);

        // Using our `WASM_MODULE` run instantiation. Note that it's guaranteed
        // that nothing is imported into differentially-executed modules so
        // this is expected to only take the module argument.
        let instance = eval(&mut scope, "new WebAssembly.Instance(WASM_MODULE)")?;

        Ok(Box::new(V8Instance {
            isolate: self.isolate.clone(),
            context: v8::Global::new(&mut scope, context),
            instance: v8::Global::new(&mut scope, instance),
        }))
    }

    fn assert_error_match(&self, wasmtime: &Trap, err: &Error) {
        let v8 = err.to_string();
        let wasmtime_msg = wasmtime.to_string();
        let verify_wasmtime = |msg: &str| {
            assert!(wasmtime_msg.contains(msg), "{wasmtime_msg}\n!=\n{v8}");
        };
        let verify_v8 = |msg: &[&str]| {
            assert!(
                msg.iter().any(|msg| v8.contains(msg)),
                "{wasmtime_msg:?}\n\t!=\n{v8}"
            );
        };
        match wasmtime {
            Trap::MemoryOutOfBounds => {
                return verify_v8(&["memory access out of bounds", "is out of bounds"])
            }
            Trap::UnreachableCodeReached => {
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
            Trap::IntegerDivisionByZero => {
                return verify_v8(&["divide by zero", "remainder by zero"])
            }
            Trap::StackOverflow => {
                return verify_v8(&[
                    "call stack size exceeded",
                    // Similar to the above comment in `UnreachableCodeReached`
                    // if wasmtime hits a stack overflow but v8 ran all the way
                    // to when the `unreachable` instruction was hit then that's
                    // ok. This just means that wasmtime either has less optimal
                    // codegen or different limits on the stack than v8 does,
                    // which isn't an issue per-se.
                    "unreachable",
                ]);
            }
            Trap::IndirectCallToNull => return verify_v8(&["null function"]),
            Trap::TableOutOfBounds => {
                return verify_v8(&[
                    "table initializer is out of bounds",
                    "table index is out of bounds",
                    "element segment out of bounds",
                ])
            }
            Trap::BadSignature => return verify_v8(&["function signature mismatch"]),
            Trap::IntegerOverflow | Trap::BadConversionToInteger => {
                return verify_v8(&[
                    "float unrepresentable in integer range",
                    "divide result unrepresentable",
                ])
            }
            other => log::debug!("unknown code {:?}", other),
        }

        verify_wasmtime("not possibly present in an error, just panic please");
    }

    fn is_stack_overflow(&self, err: &Error) -> bool {
        err.to_string().contains("Maximum call stack size exceeded")
    }
}

struct V8Instance {
    isolate: Rc<RefCell<v8::OwnedIsolate>>,
    context: v8::Global<v8::Context>,
    instance: v8::Global<v8::Value>,
}

impl DiffInstance for V8Instance {
    fn name(&self) -> &'static str {
        "v8"
    }

    fn evaluate(
        &mut self,
        function_name: &str,
        arguments: &[DiffValue],
        result_tys: &[DiffValueType],
    ) -> Result<Option<Vec<DiffValue>>> {
        let mut isolate = self.isolate.borrow_mut();
        let isolate = &mut **isolate;
        let mut scope = v8::HandleScope::new(isolate);
        let context = v8::Local::new(&mut scope, &self.context);
        let global = context.global(&mut scope);
        let mut scope = v8::ContextScope::new(&mut scope, context);

        // See https://webassembly.github.io/spec/js-api/index.html#tojsvalue
        // for how the Wasm-to-JS conversions are done.
        let mut params = Vec::new();
        for arg in arguments {
            params.push(match *arg {
                DiffValue::I32(n) => v8::Number::new(&mut scope, n.into()).into(),
                DiffValue::F32(n) => v8::Number::new(&mut scope, f32::from_bits(n).into()).into(),
                DiffValue::F64(n) => v8::Number::new(&mut scope, f64::from_bits(n)).into(),
                DiffValue::I64(n) => v8::BigInt::new_from_i64(&mut scope, n).into(),
                DiffValue::FuncRef { null } | DiffValue::ExternRef { null } => {
                    assert!(null);
                    v8::null(&mut scope).into()
                }
                // JS doesn't support v128 parameters
                DiffValue::V128(_) => return Ok(None),
                DiffValue::AnyRef { .. } => unimplemented!(),
            });
        }
        // JS doesn't support v128 return values
        for ty in result_tys {
            if let DiffValueType::V128 = ty {
                return Ok(None);
            }
        }

        let name = v8::String::new(&mut scope, "WASM_INSTANCE").unwrap();
        let instance = v8::Local::new(&mut scope, &self.instance);
        global.set(&mut scope, name.into(), instance);
        let name = v8::String::new(&mut scope, "EXPORT_NAME").unwrap();
        let func_name = v8::String::new(&mut scope, function_name).unwrap();
        global.set(&mut scope, name.into(), func_name.into());
        let name = v8::String::new(&mut scope, "ARGS").unwrap();
        let params = v8::Array::new_with_elements(&mut scope, &params);
        global.set(&mut scope, name.into(), params.into());
        let v8_vals = eval(&mut scope, "WASM_INSTANCE.exports[EXPORT_NAME](...ARGS)")?;

        let mut results = Vec::new();
        match result_tys.len() {
            0 => assert!(v8_vals.is_undefined()),
            1 => results.push(get_diff_value(&v8_vals, result_tys[0], &mut scope)),
            _ => {
                let array = v8::Local::<'_, v8::Array>::try_from(v8_vals).unwrap();
                for (i, ty) in result_tys.iter().enumerate() {
                    let v8 = array.get_index(&mut scope, i as u32).unwrap();
                    results.push(get_diff_value(&v8, *ty, &mut scope));
                }
            }
        }
        Ok(Some(results))
    }

    fn get_global(&mut self, global_name: &str, ty: DiffValueType) -> Option<DiffValue> {
        if let DiffValueType::V128 = ty {
            return None;
        }
        let mut isolate = self.isolate.borrow_mut();
        let mut scope = v8::HandleScope::new(&mut *isolate);
        let context = v8::Local::new(&mut scope, &self.context);
        let global = context.global(&mut scope);
        let mut scope = v8::ContextScope::new(&mut scope, context);

        let name = v8::String::new(&mut scope, "GLOBAL_NAME").unwrap();
        let memory_name = v8::String::new(&mut scope, global_name).unwrap();
        global.set(&mut scope, name.into(), memory_name.into());
        let val = eval(&mut scope, "WASM_INSTANCE.exports[GLOBAL_NAME].value").unwrap();
        Some(get_diff_value(&val, ty, &mut scope))
    }

    fn get_memory(&mut self, memory_name: &str, shared: bool) -> Option<Vec<u8>> {
        let mut isolate = self.isolate.borrow_mut();
        let mut scope = v8::HandleScope::new(&mut *isolate);
        let context = v8::Local::new(&mut scope, &self.context);
        let global = context.global(&mut scope);
        let mut scope = v8::ContextScope::new(&mut scope, context);

        let name = v8::String::new(&mut scope, "MEMORY_NAME").unwrap();
        let memory_name = v8::String::new(&mut scope, memory_name).unwrap();
        global.set(&mut scope, name.into(), memory_name.into());
        let v8 = eval(&mut scope, "WASM_INSTANCE.exports[MEMORY_NAME].buffer").unwrap();
        let v8_data = if shared {
            v8::Local::<'_, v8::SharedArrayBuffer>::try_from(v8)
                .unwrap()
                .get_backing_store()
        } else {
            v8::Local::<'_, v8::ArrayBuffer>::try_from(v8)
                .unwrap()
                .get_backing_store()
        };

        Some(v8_data.iter().map(|i| i.get()).collect())
    }
}

/// Evaluates the JS `code` within `scope`, returning either the result of the
/// computation or the stringified exception if one happened.
fn eval<'s>(scope: &mut v8::HandleScope<'s>, code: &str) -> Result<v8::Local<'s, v8::Value>> {
    let mut tc = v8::TryCatch::new(scope);
    let mut scope = v8::EscapableHandleScope::new(&mut tc);
    let source = v8::String::new(&mut scope, code).unwrap();
    let script = v8::Script::compile(&mut scope, source, None).unwrap();
    match script.run(&mut scope) {
        Some(val) => Ok(scope.escape(val)),
        None => {
            drop(scope);
            assert!(tc.has_caught());
            bail!(
                "{}",
                tc.message()
                    .unwrap()
                    .get(&mut tc)
                    .to_rust_string_lossy(&mut tc)
            )
        }
    }
}

fn get_diff_value(
    val: &v8::Local<'_, v8::Value>,
    ty: DiffValueType,
    scope: &mut v8::HandleScope<'_>,
) -> DiffValue {
    match ty {
        DiffValueType::I32 => DiffValue::I32(val.to_int32(scope).unwrap().value()),
        DiffValueType::I64 => {
            let (val, todo) = val.to_big_int(scope).unwrap().i64_value();
            assert!(todo);
            DiffValue::I64(val)
        }
        DiffValueType::F32 => {
            DiffValue::F32((val.to_number(scope).unwrap().value() as f32).to_bits())
        }
        DiffValueType::F64 => DiffValue::F64(val.to_number(scope).unwrap().value().to_bits()),
        DiffValueType::FuncRef => DiffValue::FuncRef {
            null: val.is_null(),
        },
        DiffValueType::ExternRef => DiffValue::ExternRef {
            null: val.is_null(),
        },
        DiffValueType::AnyRef => unimplemented!(),
        DiffValueType::V128 => unreachable!(),
    }
}

#[test]
fn smoke() {
    crate::oracles::engine::smoke_test_engine(|_, config| Ok(V8Engine::new(config)))
}
