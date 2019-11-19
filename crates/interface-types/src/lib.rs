//! A small crate to handle WebAssembly interface types in wasmtime.
//!
//! Note that this is intended to follow the [official proposal][proposal] and
//! is highly susceptible to change/breakage/etc.
//!
//! [proposal]: https://github.com/webassembly/webidl-bindings

#![deny(missing_docs)]

use anyhow::{bail, format_err, Result};
use cranelift_codegen::ir;
use std::convert::TryFrom;
use std::str;
use wasm_webidl_bindings::ast;
use wasmtime_jit::RuntimeValue;
use wasmtime_runtime::{Export, InstanceHandle};

mod value;
pub use value::Value;

/// A data structure intended to hold a parsed representation of the wasm
/// interface types of a module.
///
/// The expected usage pattern is to create this next to wasmtime data
/// structures and then use this to process arguments into wasm arguments as
/// appropriate for bound functions.
pub struct ModuleData {
    inner: Option<Inner>,
    wasi_module_name: Option<String>,
}

struct Inner {
    module: walrus::Module,
}

/// Representation of a binding of an exported function.
///
/// Can be used to learn about binding expressions and/or binding types.
pub struct ExportBinding<'a> {
    kind: ExportBindingKind<'a>,
}

enum ExportBindingKind<'a> {
    Rich {
        section: &'a ast::WebidlBindings,
        binding: &'a ast::ExportBinding,
    },
    Raw(ir::Signature),
}

impl ModuleData {
    /// Parses a raw binary wasm file, extracting information about wasm
    /// interface types.
    ///
    /// Returns an error if the wasm file is malformed.
    pub fn new(wasm: &[u8]) -> Result<ModuleData> {
        // Perform a fast search through the module for the right custom
        // section. Actually parsing out the interface types data is currently a
        // pretty expensive operation so we want to only do that if we actually
        // find the right section.
        let mut reader = wasmparser::ModuleReader::new(wasm)?;
        let mut found = false;
        let mut wasi_module_name = None;
        while !reader.eof() {
            let section = reader.read()?;

            match section.code {
                wasmparser::SectionCode::Custom { name, .. } => {
                    if name == "webidl-bindings" {
                        found = true;
                        break;
                    }
                }

                // If we see the import section then see if we can find a wasi
                // module import which we can later use to register the wasi
                // implementation automatically.
                wasmparser::SectionCode::Import => {
                    let section = section.get_import_section_reader()?;
                    for import in section {
                        let import = import?;
                        if wasmtime_wasi::is_wasi_module(import.module) {
                            wasi_module_name = Some(import.module.to_string());
                        }
                    }
                }
                _ => {}
            }
        }
        if !found {
            return Ok(ModuleData {
                inner: None,
                wasi_module_name,
            });
        }

        // Ok, perform the more expensive parsing. WebAssembly interface types
        // are super experimental and under development. To get something
        // quickly up and running we're using the same crate as `wasm-bindgen`,
        // a producer of wasm interface types, the `wasm-webidl-bindings` crate.
        // This crate relies on `walrus` which has its own IR for a wasm module.
        // Ideally we'd do all this during cranelift's own parsing of the wasm
        // module and we wouldn't have to reparse here purely for this one use
        // case.
        //
        // For now though this is "fast enough" and good enough for some demos,
        // but for full-on production quality engines we'll want to integrate
        // this much more tightly with the rest of wasmtime.
        let module = walrus::ModuleConfig::new()
            .on_parse(wasm_webidl_bindings::binary::on_parse)
            .parse(wasm)?;

        Ok(ModuleData {
            inner: Some(Inner { module }),
            wasi_module_name,
        })
    }

    /// Detects if WASI support is needed: returns module name that is requested.
    pub fn find_wasi_module_name(&self) -> Option<String> {
        self.wasi_module_name.clone()
    }

    /// Invokes wasmtime function with a `&[Value]` list. `Value` the set of
    /// wasm interface types.
    pub fn invoke_export(
        &self,
        instance: &wasmtime::HostRef<wasmtime::Instance>,
        export: &str,
        args: &[Value],
    ) -> Result<Vec<Value>> {
        let mut handle = instance.borrow().handle().clone();

        let binding = self.binding_for_export(&mut handle, export)?;
        let incoming = binding.param_bindings()?;
        let outgoing = binding.result_bindings()?;

        let f = instance
            .borrow()
            .find_export_by_name(export)
            .ok_or_else(|| format_err!("failed to find export `{}`", export))?
            .func()
            .ok_or_else(|| format_err!("`{}` is not a function", export))?
            .clone();

        let mut cx = InstanceTranslateContext(instance.clone());
        let wasm_args = translate_incoming(&mut cx, &incoming, args)?
            .into_iter()
            .map(|rv| rv.into())
            .collect::<Vec<_>>();
        let wasm_results = match f.borrow().call(&wasm_args) {
            Ok(values) => values
                .to_vec()
                .into_iter()
                .map(|v: wasmtime::Val| v.into())
                .collect::<Vec<RuntimeValue>>(),
            Err(trap) => bail!("trapped: {:?}", trap),
        };
        translate_outgoing(&mut cx, &outgoing, &wasm_results)
    }

    /// Returns an appropriate binding for the `name` export in this module
    /// which has also been instantiated as `instance` provided here.
    ///
    /// Returns an error if `name` is not present in the module.
    pub fn binding_for_export(
        &self,
        instance: &mut InstanceHandle,
        name: &str,
    ) -> Result<ExportBinding<'_>> {
        if let Some(binding) = self.interface_binding_for_export(name) {
            return Ok(binding);
        }
        let signature = match instance.lookup(name) {
            Some(Export::Function { signature, .. }) => signature,
            Some(_) => bail!("`{}` is not a function", name),
            None => bail!("failed to find export `{}`", name),
        };
        Ok(ExportBinding {
            kind: ExportBindingKind::Raw(signature),
        })
    }

    fn interface_binding_for_export(&self, name: &str) -> Option<ExportBinding<'_>> {
        let inner = self.inner.as_ref()?;
        let bindings = inner.module.customs.get_typed::<ast::WebidlBindings>()?;
        let export = inner.module.exports.iter().find(|e| e.name == name)?;
        let id = match export.item {
            walrus::ExportItem::Function(f) => f,
            _ => panic!(),
        };
        let (_, bind) = bindings.binds.iter().find(|(_, b)| b.func == id)?;
        let binding = bindings.bindings.get(bind.binding)?;
        let binding = match binding {
            ast::FunctionBinding::Export(export) => export,
            ast::FunctionBinding::Import(_) => return None,
        };
        Some(ExportBinding {
            kind: ExportBindingKind::Rich {
                binding,
                section: bindings,
            },
        })
    }
}

impl ExportBinding<'_> {
    /// Returns the list of binding expressions used to create the parameters
    /// for this binding.
    pub fn param_bindings(&self) -> Result<Vec<ast::IncomingBindingExpression>> {
        match &self.kind {
            ExportBindingKind::Rich { binding, .. } => Ok(binding.params.bindings.clone()),
            ExportBindingKind::Raw(sig) => sig
                .params
                .iter()
                .skip(1) // skip the VMContext argument
                .enumerate()
                .map(|(i, param)| default_incoming(i, param))
                .collect(),
        }
    }

    /// Returns the list of scalar types used for this binding
    pub fn param_types(&self) -> Result<Vec<ast::WebidlScalarType>> {
        match &self.kind {
            ExportBindingKind::Rich {
                binding, section, ..
            } => {
                let id = match binding.webidl_ty {
                    ast::WebidlTypeRef::Id(id) => id,
                    ast::WebidlTypeRef::Scalar(_) => {
                        bail!("webidl types for functions cannot be scalar")
                    }
                };
                let ty = section
                    .types
                    .get::<ast::WebidlCompoundType>(id)
                    .ok_or_else(|| format_err!("invalid webidl custom section"))?;
                let func = match ty {
                    ast::WebidlCompoundType::Function(f) => f,
                    _ => bail!("webidl type for function must be of function type"),
                };
                func.params
                    .iter()
                    .map(|param| match param {
                        ast::WebidlTypeRef::Id(_) => bail!("function arguments cannot be compound"),
                        ast::WebidlTypeRef::Scalar(s) => Ok(*s),
                    })
                    .collect()
            }
            ExportBindingKind::Raw(sig) => sig.params.iter().skip(1).map(abi2ast).collect(),
        }
    }

    /// Returns the list of binding expressions used to extract the return
    /// values of this binding.
    pub fn result_bindings(&self) -> Result<Vec<ast::OutgoingBindingExpression>> {
        match &self.kind {
            ExportBindingKind::Rich { binding, .. } => Ok(binding.result.bindings.clone()),
            ExportBindingKind::Raw(sig) => sig
                .returns
                .iter()
                .enumerate()
                .map(|(i, param)| default_outgoing(i, param))
                .collect(),
        }
    }
}

fn default_incoming(idx: usize, param: &ir::AbiParam) -> Result<ast::IncomingBindingExpression> {
    let get = ast::IncomingBindingExpressionGet { idx: idx as u32 };
    let ty = if param.value_type == ir::types::I32 {
        walrus::ValType::I32
    } else if param.value_type == ir::types::I64 {
        walrus::ValType::I64
    } else if param.value_type == ir::types::F32 {
        walrus::ValType::F32
    } else if param.value_type == ir::types::F64 {
        walrus::ValType::F64
    } else {
        bail!("unsupported type {:?}", param.value_type)
    };
    Ok(ast::IncomingBindingExpressionAs {
        ty,
        expr: Box::new(get.into()),
    }
    .into())
}

fn default_outgoing(idx: usize, param: &ir::AbiParam) -> Result<ast::OutgoingBindingExpression> {
    let ty = abi2ast(param)?;
    Ok(ast::OutgoingBindingExpressionAs {
        ty: ty.into(),
        idx: idx as u32,
    }
    .into())
}

fn abi2ast(param: &ir::AbiParam) -> Result<ast::WebidlScalarType> {
    Ok(if param.value_type == ir::types::I32 {
        ast::WebidlScalarType::Long
    } else if param.value_type == ir::types::I64 {
        ast::WebidlScalarType::LongLong
    } else if param.value_type == ir::types::F32 {
        ast::WebidlScalarType::UnrestrictedFloat
    } else if param.value_type == ir::types::F64 {
        ast::WebidlScalarType::UnrestrictedDouble
    } else {
        bail!("unsupported type {:?}", param.value_type)
    })
}

trait TranslateContext {
    fn invoke_alloc(&mut self, alloc_func_name: &str, len: i32) -> Result<i32>;
    unsafe fn get_memory(&mut self) -> Result<&mut [u8]>;
}

struct InstanceTranslateContext(pub wasmtime::HostRef<wasmtime::Instance>);

impl TranslateContext for InstanceTranslateContext {
    fn invoke_alloc(&mut self, alloc_func_name: &str, len: i32) -> Result<i32> {
        let alloc = self
            .0
            .borrow()
            .find_export_by_name(alloc_func_name)
            .ok_or_else(|| format_err!("failed to find alloc function `{}`", alloc_func_name))?
            .func()
            .ok_or_else(|| format_err!("`{}` is not a (alloc) function", alloc_func_name))?
            .clone();
        let alloc_args = vec![wasmtime::Val::I32(len)];
        let results = match alloc.borrow().call(&alloc_args) {
            Ok(values) => values,
            Err(trap) => bail!("trapped: {:?}", trap),
        };
        if results.len() != 1 {
            bail!("allocator function wrong number of results");
        }
        Ok(match results[0] {
            wasmtime::Val::I32(i) => i,
            _ => bail!("allocator function bad return type"),
        })
    }
    unsafe fn get_memory(&mut self) -> Result<&mut [u8]> {
        let memory = self
            .0
            .borrow()
            .find_export_by_name("memory")
            .ok_or_else(|| format_err!("failed to find `memory` export"))?
            .memory()
            .ok_or_else(|| format_err!("`memory` is not a memory"))?
            .clone();
        let ptr = memory.borrow().data_ptr();
        let len = memory.borrow().data_size();
        Ok(std::slice::from_raw_parts_mut(ptr, len))
    }
}

fn translate_incoming(
    cx: &mut dyn TranslateContext,
    bindings: &[ast::IncomingBindingExpression],
    args: &[Value],
) -> Result<Vec<RuntimeValue>> {
    let get = |expr: &ast::IncomingBindingExpression| match expr {
        ast::IncomingBindingExpression::Get(g) => args
            .get(g.idx as usize)
            .ok_or_else(|| format_err!("argument index out of bounds: {}", g.idx)),
        _ => bail!("unsupported incoming binding expr {:?}", expr),
    };

    let mut copy = |alloc_func_name: &str, bytes: &[u8]| -> Result<(i32, i32)> {
        let len = i32::try_from(bytes.len()).map_err(|_| format_err!("length overflow"))?;
        let ptr = cx.invoke_alloc(alloc_func_name, len)?;
        unsafe {
            let raw = cx.get_memory()?;
            raw[ptr as usize..][..bytes.len()].copy_from_slice(bytes)
        }

        Ok((ptr, len))
    };

    let mut wasm = Vec::new();

    for expr in bindings {
        match expr {
            ast::IncomingBindingExpression::AllocUtf8Str(g) => {
                let val = match get(&g.expr)? {
                    Value::String(s) => s,
                    _ => bail!("expected a string"),
                };
                let (ptr, len) = copy(&g.alloc_func_name, val.as_bytes())?;
                wasm.push(RuntimeValue::I32(ptr));
                wasm.push(RuntimeValue::I32(len));
            }
            ast::IncomingBindingExpression::As(g) => {
                let val = get(&g.expr)?;
                match g.ty {
                    walrus::ValType::I32 => match val {
                        Value::I32(i) => wasm.push(RuntimeValue::I32(*i)),
                        Value::U32(i) => wasm.push(RuntimeValue::I32(*i as i32)),
                        _ => bail!("cannot convert {:?} to `i32`", val),
                    },
                    walrus::ValType::I64 => match val {
                        Value::I32(i) => wasm.push(RuntimeValue::I64((*i).into())),
                        Value::U32(i) => wasm.push(RuntimeValue::I64((*i).into())),
                        Value::I64(i) => wasm.push(RuntimeValue::I64(*i)),
                        Value::U64(i) => wasm.push(RuntimeValue::I64(*i as i64)),
                        _ => bail!("cannot convert {:?} to `i64`", val),
                    },
                    walrus::ValType::F32 => match val {
                        Value::F32(i) => wasm.push(RuntimeValue::F32(i.to_bits())),
                        _ => bail!("cannot convert {:?} to `f32`", val),
                    },
                    walrus::ValType::F64 => match val {
                        Value::F32(i) => wasm.push(RuntimeValue::F64((*i as f64).to_bits())),
                        Value::F64(i) => wasm.push(RuntimeValue::F64(i.to_bits())),
                        _ => bail!("cannot convert {:?} to `f64`", val),
                    },
                    walrus::ValType::V128 | walrus::ValType::Anyref => {
                        bail!("unsupported `as` type {:?}", g.ty);
                    }
                }
            }
            _ => bail!("unsupported incoming binding expr {:?}", expr),
        }
    }

    Ok(wasm)
}

fn translate_outgoing(
    cx: &mut dyn TranslateContext,
    bindings: &[ast::OutgoingBindingExpression],
    args: &[RuntimeValue],
) -> Result<Vec<Value>> {
    let mut values = Vec::new();

    let get = |idx: u32| {
        args.get(idx as usize)
            .cloned()
            .ok_or_else(|| format_err!("argument index out of bounds: {}", idx))
    };

    for expr in bindings {
        match expr {
            ast::OutgoingBindingExpression::As(a) => {
                let arg = get(a.idx)?;
                match a.ty {
                    ast::WebidlTypeRef::Scalar(ast::WebidlScalarType::UnsignedLong) => match arg {
                        RuntimeValue::I32(a) => values.push(Value::U32(a as u32)),
                        _ => bail!("can't convert {:?} to unsigned long", arg),
                    },
                    ast::WebidlTypeRef::Scalar(ast::WebidlScalarType::Long) => match arg {
                        RuntimeValue::I32(a) => values.push(Value::I32(a)),
                        _ => bail!("can't convert {:?} to long", arg),
                    },
                    ast::WebidlTypeRef::Scalar(ast::WebidlScalarType::LongLong) => match arg {
                        RuntimeValue::I32(a) => values.push(Value::I64(a as i64)),
                        RuntimeValue::I64(a) => values.push(Value::I64(a)),
                        _ => bail!("can't convert {:?} to long long", arg),
                    },
                    ast::WebidlTypeRef::Scalar(ast::WebidlScalarType::UnsignedLongLong) => {
                        match arg {
                            RuntimeValue::I32(a) => values.push(Value::U64(a as u64)),
                            RuntimeValue::I64(a) => values.push(Value::U64(a as u64)),
                            _ => bail!("can't convert {:?} to unsigned long long", arg),
                        }
                    }
                    ast::WebidlTypeRef::Scalar(ast::WebidlScalarType::Float) => match arg {
                        RuntimeValue::F32(a) => values.push(Value::F32(f32::from_bits(a))),
                        _ => bail!("can't convert {:?} to float", arg),
                    },
                    ast::WebidlTypeRef::Scalar(ast::WebidlScalarType::Double) => match arg {
                        RuntimeValue::F32(a) => values.push(Value::F64(f32::from_bits(a) as f64)),
                        RuntimeValue::F64(a) => values.push(Value::F64(f64::from_bits(a))),
                        _ => bail!("can't convert {:?} to double", arg),
                    },
                    _ => bail!("unsupported outgoing binding expr {:?}", expr),
                }
            }
            ast::OutgoingBindingExpression::Utf8Str(e) => {
                if e.ty != ast::WebidlScalarType::DomString.into() {
                    bail!("utf-8 strings must go into dom-string")
                }
                let offset = match get(e.offset)? {
                    RuntimeValue::I32(a) => a,
                    _ => bail!("offset must be an i32"),
                };
                let length = match get(e.length)? {
                    RuntimeValue::I32(a) => a,
                    _ => bail!("length must be an i32"),
                };
                let bytes = unsafe { &cx.get_memory()?[offset as usize..][..length as usize] };
                values.push(Value::String(str::from_utf8(bytes).unwrap().to_string()));
            }
            _ => {
                drop(cx);
                bail!("unsupported outgoing binding expr {:?}", expr);
            }
        }
    }

    Ok(values)
}
