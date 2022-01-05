//! Dummy implementations of things that a Wasm module can import.
//!
//! Forked from `wasmtime/crates/fuzzing/src/oracles/dummy.rs`.

use anyhow::Result;
use std::fmt::Write;
use wasmtime::*;

/// Create dummy imports for instantiating the module.
pub fn dummy_imports(
    store: &mut crate::Store,
    module: &wasmtime::Module,
    linker: &mut crate::Linker,
) -> Result<()> {
    log::debug!("Creating dummy imports");

    for imp in module.imports() {
        match imp.name() {
            Some(name) => {
                if linker.get(&mut *store, imp.module(), Some(name)).is_some() {
                    // Already defined, must be part of WASI.
                    continue;
                }

                linker
                    .define(
                        imp.module(),
                        name,
                        dummy_extern(
                            &mut *store,
                            imp.ty(),
                            &format!("'{}' '{}'", imp.module(), name),
                        )?,
                    )
                    .unwrap();
            }
            None => match imp.ty() {
                wasmtime::ExternType::Instance(ty) => {
                    for ty in ty.exports() {
                        if linker
                            .get(&mut *store, imp.module(), Some(ty.name()))
                            .is_some()
                        {
                            // Already defined, must be part of WASI.
                            continue;
                        }

                        linker
                            .define(
                                imp.module(),
                                ty.name(),
                                dummy_extern(&mut *store, ty.ty(), &format!("'{}'", imp.module()))?,
                            )
                            .unwrap();
                    }
                }
                other => {
                    if linker.get(&mut *store, imp.module(), None).is_some() {
                        // Already defined, must be part of WASI.
                        continue;
                    }

                    linker
                        .define_name(
                            imp.module(),
                            dummy_extern(&mut *store, other, &format!("'{}'", imp.module()))?,
                        )
                        .unwrap();
                }
            },
        }
    }

    Ok(())
}

/// Construct a dummy `Extern` from its type signature
pub fn dummy_extern(store: &mut crate::Store, ty: ExternType, name: &str) -> Result<Extern> {
    Ok(match ty {
        ExternType::Func(func_ty) => Extern::Func(dummy_func(store, func_ty, name)),
        ExternType::Instance(instance_ty) => {
            Extern::Instance(dummy_instance(store, instance_ty, name)?)
        }
        ExternType::Global(_) => {
            anyhow::bail!("Error: attempted to import unknown global: {}", name)
        }
        ExternType::Table(_) => anyhow::bail!("Error: attempted to import unknown table: {}", name),
        ExternType::Memory(_) => {
            anyhow::bail!("Error: attempted to import unknown memory: {}", name)
        }
        ExternType::Module(_) => {
            anyhow::bail!("Error: attempted to import unknown module: {}", name)
        }
    })
}

/// Construct a dummy function for the given function type
pub fn dummy_func(store: &mut crate::Store, ty: FuncType, name: &str) -> Func {
    let name = name.to_string();
    Func::new(store, ty.clone(), move |_caller, _params, _results| {
        Err(Trap::new(format!(
            "Error: attempted to call an unknown imported function: {}\n\
             \n\
             You cannot call arbitrary imported functions during Wizer initialization.",
            name,
        )))
    })
}

/// Construct a dummy value for the given value type.
#[cfg(fuzzing)]
pub fn dummy_value(val_ty: ValType) -> Val {
    match val_ty {
        ValType::I32 => Val::I32(0),
        ValType::I64 => Val::I64(0),
        ValType::F32 => Val::F32(0),
        ValType::F64 => Val::F64(0),
        ValType::V128 => Val::V128(0),
        ValType::ExternRef => Val::ExternRef(None),
        ValType::FuncRef => Val::FuncRef(None),
    }
}

/// Construct a sequence of dummy values for the given types.
#[cfg(fuzzing)]
pub fn dummy_values(val_tys: impl IntoIterator<Item = ValType>) -> Vec<Val> {
    val_tys.into_iter().map(dummy_value).collect()
}

/// Construct a dummy instance for the given instance type.
///
/// This is done by using the expected type to generate a module on-the-fly
/// which we the instantiate.
pub fn dummy_instance(store: &mut crate::Store, ty: InstanceType, name: &str) -> Result<Instance> {
    let mut wat = WatGenerator::new();
    for ty in ty.exports() {
        wat.export(&ty, name)?;
    }
    let module = Module::new(store.engine(), &wat.finish()).unwrap();
    Ok(Instance::new(store, &module, &[])?)
}

struct WatGenerator {
    tmp: usize,
    dst: String,
}

impl WatGenerator {
    fn new() -> WatGenerator {
        WatGenerator {
            tmp: 0,
            dst: String::from("(module\n"),
        }
    }

    fn finish(mut self) -> String {
        self.dst.push_str(")\n");
        self.dst
    }

    fn export(&mut self, ty: &ExportType<'_>, instance_name: &str) -> Result<()> {
        let wat_name = format!("item{}", self.tmp);
        self.tmp += 1;
        let item_ty = ty.ty();
        self.item(&wat_name, &item_ty, instance_name, ty.name())?;

        write!(self.dst, "(export ").unwrap();
        self.str(ty.name());
        write!(self.dst, " (").unwrap();
        match item_ty {
            ExternType::Func(_) => write!(self.dst, "func").unwrap(),
            ExternType::Instance(_) => write!(self.dst, "instance").unwrap(),
            ExternType::Memory(_) => anyhow::bail!(
                "Error: attempted to import unknown memory: '{}' '{}'",
                instance_name,
                ty.name()
            ),
            ExternType::Global(_) => anyhow::bail!(
                "Error: attempted to import unknown global: '{}' '{}'",
                instance_name,
                ty.name()
            ),
            ExternType::Table(_) => anyhow::bail!(
                "Error: attempted to import unknown table: '{}' '{}'",
                instance_name,
                ty.name()
            ),
            ExternType::Module(_) => anyhow::bail!(
                "Error: attempted to import unknown module: '{}' '{}'",
                instance_name,
                ty.name()
            ),
        }
        writeln!(self.dst, " ${}))", wat_name).unwrap();
        Ok(())
    }

    fn item(
        &mut self,
        name: &str,
        ty: &ExternType,
        instance_name: &str,
        item_name: &str,
    ) -> Result<()> {
        match ty {
            ExternType::Func(ty) => {
                write!(self.dst, "(func ${} ", name).unwrap();
                self.func_sig(ty);
                for ty in ty.results() {
                    writeln!(self.dst, "").unwrap();
                    self.value(&ty);
                }
                writeln!(self.dst, ")").unwrap();
            }
            ExternType::Instance(ty) => {
                writeln!(self.dst, "(module ${}_module", name).unwrap();
                for ty in ty.exports() {
                    self.export(&ty, instance_name)?;
                }
                self.dst.push_str(")\n");
                writeln!(self.dst, "(instance ${} (instantiate ${0}_module))", name).unwrap();
            }
            ExternType::Memory(_) => anyhow::bail!(
                "Error: attempted to import unknown memory: '{}' '{}'",
                instance_name,
                item_name
            ),
            ExternType::Global(_) => anyhow::bail!(
                "Error: attempted to import unknown global: '{}' '{}'",
                instance_name,
                item_name
            ),
            ExternType::Table(_) => anyhow::bail!(
                "Error: attempted to import unknown table: '{}' '{}'",
                instance_name,
                item_name
            ),
            ExternType::Module(_) => anyhow::bail!(
                "Error: attempted to import unknown module: '{}' '{}'",
                instance_name,
                item_name
            ),
        }
        Ok(())
    }

    fn func_sig(&mut self, ty: &FuncType) {
        write!(self.dst, "(param ").unwrap();
        for ty in ty.params() {
            write!(self.dst, "{} ", wat_ty(&ty)).unwrap();
        }
        write!(self.dst, ") (result ").unwrap();
        for ty in ty.results() {
            write!(self.dst, "{} ", wat_ty(&ty)).unwrap();
        }
        write!(self.dst, ")").unwrap();
    }

    fn value(&mut self, ty: &ValType) {
        match ty {
            ValType::I32 => write!(self.dst, "i32.const 0").unwrap(),
            ValType::I64 => write!(self.dst, "i64.const 0").unwrap(),
            ValType::F32 => write!(self.dst, "f32.const 0").unwrap(),
            ValType::F64 => write!(self.dst, "f64.const 0").unwrap(),
            ValType::V128 => write!(self.dst, "v128.const i32x4 0 0 0 0").unwrap(),
            ValType::ExternRef => write!(self.dst, "ref.null extern").unwrap(),
            ValType::FuncRef => write!(self.dst, "ref.null func").unwrap(),
        }
    }

    fn str(&mut self, name: &str) {
        let mut bytes = [0; 4];
        self.dst.push_str("\"");
        for c in name.chars() {
            let v = c as u32;
            if v >= 0x20 && v < 0x7f && c != '"' && c != '\\' && v < 0xff {
                self.dst.push(c);
            } else {
                for byte in c.encode_utf8(&mut bytes).as_bytes() {
                    self.hex_byte(*byte);
                }
            }
        }
        self.dst.push_str("\"");
    }

    fn hex_byte(&mut self, byte: u8) {
        fn to_hex(b: u8) -> char {
            if b < 10 {
                (b'0' + b) as char
            } else {
                (b'a' + b - 10) as char
            }
        }
        self.dst.push('\\');
        self.dst.push(to_hex((byte >> 4) & 0xf));
        self.dst.push(to_hex(byte & 0xf));
    }
}

fn wat_ty(ty: &ValType) -> &'static str {
    match ty {
        ValType::I32 => "i32",
        ValType::I64 => "i64",
        ValType::F32 => "f32",
        ValType::F64 => "f64",
        ValType::V128 => "v128",
        ValType::ExternRef => "externref",
        ValType::FuncRef => "funcref",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn store() -> crate::Store {
        let mut config = Config::default();
        config.wasm_module_linking(true);
        config.wasm_multi_memory(true);
        let engine = wasmtime::Engine::new(&config).unwrap();
        Store::new(&engine, None)
    }

    #[test]
    fn dummy_function_import() {
        let mut store = store();
        let func_ty = FuncType::new(vec![ValType::I32], vec![ValType::I64]);
        let func = dummy_func(&mut store, func_ty.clone(), "f");
        assert_eq!(func.ty(&store), func_ty);
    }

    #[test]
    fn dummy_instance_import() {
        let mut store = store();

        let mut instance_ty = InstanceType::new();

        // Functions.
        instance_ty.add_named_export("func0", FuncType::new(vec![ValType::I32], vec![]).into());
        instance_ty.add_named_export("func1", FuncType::new(vec![], vec![ValType::I64]).into());

        // Instances.
        instance_ty.add_named_export("instance0", InstanceType::new().into());
        instance_ty.add_named_export("instance1", InstanceType::new().into());

        let instance = dummy_instance(&mut store, instance_ty.clone(), "instance").unwrap();

        let mut expected_exports = vec!["func0", "func1", "instance0", "instance1"]
            .into_iter()
            .collect::<HashSet<_>>();
        for exp in instance.ty(&store).exports() {
            let was_expected = expected_exports.remove(exp.name());
            assert!(was_expected);
        }
        assert!(expected_exports.is_empty());
    }
}
