//! Dummy implementations of things that a Wasm module can import.

use anyhow::Result;
use std::fmt::Write;
use wasmtime::*;

/// Create a set of dummy functions/globals/etc for the given imports.
pub fn dummy_linker<'module, T>(store: &mut Store<T>, module: &Module) -> Result<Linker<T>> {
    let mut linker = Linker::new(store.engine());
    linker.allow_shadowing(true);
    for import in module.imports() {
        match import.name() {
            Some(name) => {
                linker
                    .define(import.module(), name, dummy_extern(store, import.ty())?)
                    .unwrap();
            }
            None => match import.ty() {
                ExternType::Instance(ty) => {
                    for ty in ty.exports() {
                        linker
                            .define(import.module(), ty.name(), dummy_extern(store, ty.ty())?)
                            .unwrap();
                    }
                }
                other => {
                    linker
                        .define_name(import.module(), dummy_extern(store, other)?)
                        .unwrap();
                }
            },
        }
    }
    Ok(linker)
}

/// Construct a dummy `Extern` from its type signature
pub fn dummy_extern<T>(store: &mut Store<T>, ty: ExternType) -> Result<Extern> {
    Ok(match ty {
        ExternType::Func(func_ty) => Extern::Func(dummy_func(store, func_ty)),
        ExternType::Global(global_ty) => Extern::Global(dummy_global(store, global_ty)),
        ExternType::Table(table_ty) => Extern::Table(dummy_table(store, table_ty)),
        ExternType::Memory(mem_ty) => Extern::Memory(dummy_memory(store, mem_ty)?),
        ExternType::Instance(instance_ty) => Extern::Instance(dummy_instance(store, instance_ty)),
        ExternType::Module(module_ty) => Extern::Module(dummy_module(store.engine(), module_ty)),
    })
}

/// Construct a dummy function for the given function type
pub fn dummy_func<T>(store: &mut Store<T>, ty: FuncType) -> Func {
    Func::new(store, ty.clone(), move |_, _, results| {
        for (ret_ty, result) in ty.results().zip(results) {
            *result = dummy_value(ret_ty);
        }
        Ok(())
    })
}

/// Construct a dummy value for the given value type.
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
pub fn dummy_values(val_tys: impl IntoIterator<Item = ValType>) -> Vec<Val> {
    val_tys.into_iter().map(dummy_value).collect()
}

/// Construct a dummy global for the given global type.
pub fn dummy_global<T>(store: &mut Store<T>, ty: GlobalType) -> Global {
    let val = dummy_value(ty.content().clone());
    Global::new(store, ty, val).unwrap()
}

/// Construct a dummy table for the given table type.
pub fn dummy_table<T>(store: &mut Store<T>, ty: TableType) -> Table {
    let init_val = dummy_value(ty.element().clone());
    Table::new(store, ty, init_val).unwrap()
}

/// Construct a dummy memory for the given memory type.
pub fn dummy_memory<T>(store: &mut Store<T>, ty: MemoryType) -> Result<Memory> {
    Memory::new(store, ty)
}

/// Construct a dummy instance for the given instance type.
///
/// This is done by using the expected type to generate a module on-the-fly
/// which we the instantiate.
pub fn dummy_instance<T>(store: &mut Store<T>, ty: InstanceType) -> Instance {
    let mut wat = WatGenerator::new();
    for ty in ty.exports() {
        wat.export(&ty);
    }
    let module = Module::new(store.engine(), &wat.finish()).unwrap();
    Instance::new(store, &module, &[]).unwrap()
}

/// Construct a dummy module for the given module type.
///
/// This is done by using the expected type to generate a module on-the-fly.
pub fn dummy_module(engine: &Engine, ty: ModuleType) -> Module {
    let mut wat = WatGenerator::new();
    for ty in ty.imports() {
        wat.import(&ty);
    }
    for ty in ty.exports() {
        wat.export(&ty);
    }
    Module::new(engine, &wat.finish()).unwrap()
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

    fn import(&mut self, ty: &ImportType<'_>) {
        write!(self.dst, "(import ").unwrap();
        self.str(ty.module());
        write!(self.dst, " ").unwrap();
        if let Some(field) = ty.name() {
            self.str(field);
            write!(self.dst, " ").unwrap();
        }
        self.item_ty(&ty.ty());
        writeln!(self.dst, ")").unwrap();
    }

    fn item_ty(&mut self, ty: &ExternType) {
        match ty {
            ExternType::Memory(mem) => {
                write!(
                    self.dst,
                    "(memory {} {})",
                    mem.limits().min(),
                    match mem.limits().max() {
                        Some(max) => max.to_string(),
                        None => String::new(),
                    }
                )
                .unwrap();
            }
            ExternType::Table(table) => {
                write!(
                    self.dst,
                    "(table {} {} {})",
                    table.limits().min(),
                    match table.limits().max() {
                        Some(max) => max.to_string(),
                        None => String::new(),
                    },
                    wat_ty(table.element()),
                )
                .unwrap();
            }
            ExternType::Global(ty) => {
                if ty.mutability() == Mutability::Const {
                    write!(self.dst, "(global {})", wat_ty(ty.content())).unwrap();
                } else {
                    write!(self.dst, "(global (mut {}))", wat_ty(ty.content())).unwrap();
                }
            }
            ExternType::Func(ty) => {
                write!(self.dst, "(func ").unwrap();
                self.func_sig(ty);
                write!(self.dst, ")").unwrap();
            }
            ExternType::Instance(ty) => {
                writeln!(self.dst, "(instance").unwrap();
                for ty in ty.exports() {
                    write!(self.dst, "(export ").unwrap();
                    self.str(ty.name());
                    write!(self.dst, " ").unwrap();
                    self.item_ty(&ty.ty());
                    writeln!(self.dst, ")").unwrap();
                }
                write!(self.dst, ")").unwrap();
            }
            ExternType::Module(ty) => {
                writeln!(self.dst, "(module").unwrap();
                for ty in ty.imports() {
                    self.import(&ty);
                    writeln!(self.dst, "").unwrap();
                }
                for ty in ty.exports() {
                    write!(self.dst, "(export ").unwrap();
                    self.str(ty.name());
                    write!(self.dst, " ").unwrap();
                    self.item_ty(&ty.ty());
                    writeln!(self.dst, ")").unwrap();
                }
                write!(self.dst, ")").unwrap();
            }
        }
    }

    fn export(&mut self, ty: &ExportType<'_>) {
        let wat_name = format!("item{}", self.tmp);
        self.tmp += 1;
        let item_ty = ty.ty();
        self.item(&wat_name, &item_ty);

        write!(self.dst, "(export ").unwrap();
        self.str(ty.name());
        write!(self.dst, " (").unwrap();
        match item_ty {
            ExternType::Memory(_) => write!(self.dst, "memory").unwrap(),
            ExternType::Global(_) => write!(self.dst, "global").unwrap(),
            ExternType::Func(_) => write!(self.dst, "func").unwrap(),
            ExternType::Instance(_) => write!(self.dst, "instance").unwrap(),
            ExternType::Table(_) => write!(self.dst, "table").unwrap(),
            ExternType::Module(_) => write!(self.dst, "module").unwrap(),
        }
        writeln!(self.dst, " ${}))", wat_name).unwrap();
    }

    fn item(&mut self, name: &str, ty: &ExternType) {
        match ty {
            ExternType::Memory(mem) => {
                write!(
                    self.dst,
                    "(memory ${} {} {})\n",
                    name,
                    mem.limits().min(),
                    match mem.limits().max() {
                        Some(max) => max.to_string(),
                        None => String::new(),
                    }
                )
                .unwrap();
            }
            ExternType::Table(table) => {
                write!(
                    self.dst,
                    "(table ${} {} {} {})\n",
                    name,
                    table.limits().min(),
                    match table.limits().max() {
                        Some(max) => max.to_string(),
                        None => String::new(),
                    },
                    wat_ty(table.element()),
                )
                .unwrap();
            }
            ExternType::Global(ty) => {
                write!(self.dst, "(global ${} ", name).unwrap();
                if ty.mutability() == Mutability::Var {
                    write!(self.dst, "(mut ").unwrap();
                }
                write!(self.dst, "{}", wat_ty(ty.content())).unwrap();
                if ty.mutability() == Mutability::Var {
                    write!(self.dst, ")").unwrap();
                }
                write!(self.dst, " (").unwrap();
                self.value(ty.content());
                writeln!(self.dst, "))").unwrap();
            }
            ExternType::Func(ty) => {
                write!(self.dst, "(func ${} ", name).unwrap();
                self.func_sig(ty);
                for ty in ty.results() {
                    writeln!(self.dst, "").unwrap();
                    self.value(&ty);
                }
                writeln!(self.dst, ")").unwrap();
            }
            ExternType::Module(ty) => {
                writeln!(self.dst, "(module ${}", name).unwrap();
                for ty in ty.imports() {
                    self.import(&ty);
                }
                for ty in ty.exports() {
                    self.export(&ty);
                }
                self.dst.push_str(")\n");
            }
            ExternType::Instance(ty) => {
                writeln!(self.dst, "(module ${}_module", name).unwrap();
                for ty in ty.exports() {
                    self.export(&ty);
                }
                self.dst.push_str(")\n");
                writeln!(self.dst, "(instance ${} (instantiate ${0}_module))", name).unwrap();
            }
        }
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

    fn store() -> Store<()> {
        let mut config = Config::default();
        config.wasm_module_linking(true);
        config.wasm_multi_memory(true);
        let engine = wasmtime::Engine::new(&config).unwrap();
        Store::new(&engine, ())
    }

    #[test]
    fn dummy_table_import() {
        let mut store = store();
        let table = dummy_table(
            &mut store,
            TableType::new(ValType::ExternRef, Limits::at_least(10)),
        );
        assert_eq!(table.size(&store), 10);
        for i in 0..10 {
            assert!(table
                .get(&mut store, i)
                .unwrap()
                .unwrap_externref()
                .is_none());
        }
    }

    #[test]
    fn dummy_global_import() {
        let mut store = store();
        let global = dummy_global(&mut store, GlobalType::new(ValType::I32, Mutability::Const));
        assert_eq!(*global.ty(&store).content(), ValType::I32);
        assert_eq!(global.ty(&store).mutability(), Mutability::Const);
    }

    #[test]
    fn dummy_memory_import() {
        let mut store = store();
        let memory = dummy_memory(&mut store, MemoryType::new(Limits::at_least(1))).unwrap();
        assert_eq!(memory.size(&store), 1);
    }

    #[test]
    fn dummy_function_import() {
        let mut store = store();
        let func_ty = FuncType::new(vec![ValType::I32], vec![ValType::I64]);
        let func = dummy_func(&mut store, func_ty.clone());
        assert_eq!(func.ty(&store), func_ty);
    }

    #[test]
    fn dummy_instance_import() {
        let mut store = store();

        let mut instance_ty = InstanceType::new();

        // Functions.
        instance_ty.add_named_export("func0", FuncType::new(vec![ValType::I32], vec![]).into());
        instance_ty.add_named_export("func1", FuncType::new(vec![], vec![ValType::I64]).into());

        // Globals.
        instance_ty.add_named_export(
            "global0",
            GlobalType::new(ValType::I32, Mutability::Const).into(),
        );
        instance_ty.add_named_export(
            "global1",
            GlobalType::new(ValType::I64, Mutability::Var).into(),
        );

        // Tables.
        instance_ty.add_named_export(
            "table0",
            TableType::new(ValType::ExternRef, Limits::at_least(1)).into(),
        );
        instance_ty.add_named_export(
            "table1",
            TableType::new(ValType::ExternRef, Limits::at_least(1)).into(),
        );

        // Memories.
        instance_ty.add_named_export("memory0", MemoryType::new(Limits::at_least(1)).into());
        instance_ty.add_named_export("memory1", MemoryType::new(Limits::at_least(1)).into());

        // Modules.
        instance_ty.add_named_export("module0", ModuleType::new().into());
        instance_ty.add_named_export("module1", ModuleType::new().into());

        // Instances.
        instance_ty.add_named_export("instance0", InstanceType::new().into());
        instance_ty.add_named_export("instance1", InstanceType::new().into());

        let instance = dummy_instance(&mut store, instance_ty.clone());

        let mut expected_exports = vec![
            "func0",
            "func1",
            "global0",
            "global1",
            "table0",
            "table1",
            "memory0",
            "memory1",
            "module0",
            "module1",
            "instance0",
            "instance1",
        ]
        .into_iter()
        .collect::<HashSet<_>>();
        for exp in instance.ty(&store).exports() {
            let was_expected = expected_exports.remove(exp.name());
            assert!(was_expected);
        }
        assert!(expected_exports.is_empty());
    }

    #[test]
    fn dummy_module_import() {
        let store = store();

        let mut module_ty = ModuleType::new();

        // Multiple exported and imported functions.
        module_ty.add_named_export("func0", FuncType::new(vec![ValType::I32], vec![]).into());
        module_ty.add_named_export("func1", FuncType::new(vec![], vec![ValType::I64]).into());
        module_ty.add_named_import(
            "func2",
            None,
            FuncType::new(vec![ValType::I64], vec![]).into(),
        );
        module_ty.add_named_import(
            "func3",
            None,
            FuncType::new(vec![], vec![ValType::I32]).into(),
        );

        // Multiple exported and imported globals.
        module_ty.add_named_export(
            "global0",
            GlobalType::new(ValType::I32, Mutability::Const).into(),
        );
        module_ty.add_named_export(
            "global1",
            GlobalType::new(ValType::I64, Mutability::Var).into(),
        );
        module_ty.add_named_import(
            "global2",
            None,
            GlobalType::new(ValType::I32, Mutability::Var).into(),
        );
        module_ty.add_named_import(
            "global3",
            None,
            GlobalType::new(ValType::I64, Mutability::Const).into(),
        );

        // Multiple exported and imported tables.
        module_ty.add_named_export(
            "table0",
            TableType::new(ValType::ExternRef, Limits::at_least(1)).into(),
        );
        module_ty.add_named_export(
            "table1",
            TableType::new(ValType::ExternRef, Limits::at_least(1)).into(),
        );
        module_ty.add_named_import(
            "table2",
            None,
            TableType::new(ValType::ExternRef, Limits::at_least(1)).into(),
        );
        module_ty.add_named_import(
            "table3",
            None,
            TableType::new(ValType::ExternRef, Limits::at_least(1)).into(),
        );

        // Multiple exported and imported memories.
        module_ty.add_named_export("memory0", MemoryType::new(Limits::at_least(1)).into());
        module_ty.add_named_export("memory1", MemoryType::new(Limits::at_least(1)).into());
        module_ty.add_named_import("memory2", None, MemoryType::new(Limits::at_least(1)).into());
        module_ty.add_named_import("memory3", None, MemoryType::new(Limits::at_least(1)).into());

        // An exported and an imported module.
        module_ty.add_named_export("module0", ModuleType::new().into());
        module_ty.add_named_import("module1", None, ModuleType::new().into());

        // An exported and an imported instance.
        module_ty.add_named_export("instance0", InstanceType::new().into());
        module_ty.add_named_import("instance1", None, InstanceType::new().into());

        // Create the module.
        let module = dummy_module(store.engine(), module_ty);

        // Check that we have the expected exports.
        assert!(module.get_export("func0").is_some());
        assert!(module.get_export("func1").is_some());
        assert!(module.get_export("global0").is_some());
        assert!(module.get_export("global1").is_some());
        assert!(module.get_export("table0").is_some());
        assert!(module.get_export("table1").is_some());
        assert!(module.get_export("memory0").is_some());
        assert!(module.get_export("memory1").is_some());
        assert!(module.get_export("instance0").is_some());
        assert!(module.get_export("module0").is_some());

        // Check that we have the exported imports.
        let mut expected_imports = vec![
            "func2",
            "func3",
            "global2",
            "global3",
            "table2",
            "table3",
            "memory2",
            "memory3",
            "instance1",
            "module1",
        ]
        .into_iter()
        .collect::<HashSet<_>>();
        for imp in module.imports() {
            assert!(imp.name().is_none());
            let was_expected = expected_imports.remove(imp.module());
            assert!(was_expected);
        }
        assert!(expected_imports.is_empty());
    }
}
