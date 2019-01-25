use cranelift_codegen::ir;
use cranelift_codegen::isa::TargetFrontendConfig;
use cranelift_entity::EntityRef;
use cranelift_wasm::FuncIndex;
use std::io::{Result, Write};
use wasmtime_environ::{DataInitializer, Module, VMOffsets};

/// Generates C code based on module metadata.
pub fn generate_c_stubs(
    header: &mut Vec<u8>,
    code: &mut Vec<u8>,
    name: &str,
    module: &Module,
    data_initializers: &[DataInitializer],
    target_config: &TargetFrontendConfig,
) -> Result<()> {
    let ofs = VMOffsets::new(target_config.pointer_bytes(), &module);

    let uppercase_name = String::from(name).to_uppercase();
    writeln!(header, "#ifndef __{}_H", uppercase_name)?;
    writeln!(header, "#define __{}_H", uppercase_name)?;
    writeln!(header, "#include <stddef.h>")?;
    writeln!(header, "#include <stdint.h>")?;
    writeln!(header)?;

    if !module.memory_plans.is_empty() {
        if module.memory_plans.len() > 1 {
            panic!("multiple memories not supported yet");
        }
        let (_, memory_plan) = module
            .memory_plans
            .iter()
            .next()
            .expect("at least one memory");
        let size = memory_plan.memory.minimum << 16;
        writeln!(header, "#define MEMORY_INIT_SIZE {}", size)?;
    }

    writeln!(header)?;
    writeln!(header, "struct VMContext {{")?;
    if ofs.num_signature_ids > 0 {
        writeln!(header, "\tuint32_t\tsignatures[{}];", ofs.num_signature_ids)?;
    }
    if ofs.num_imported_functions > 0 {
        writeln!(header, "\tstruct {{")?;
        writeln!(header, "\t\tvoid\t*body;")?;
        writeln!(header, "\t\tvoid\t*vmctx;")?;
        writeln!(
            header,
            "\t}}\timported_functions[{}];",
            ofs.num_imported_functions
        )?;
    }
    if ofs.num_imported_tables > 0 {
        writeln!(header, "\tstruct {{")?;
        writeln!(header, "\t\tvoid\t*from;")?;
        writeln!(header, "\t\tvoid\t*vmctx;")?;
        writeln!(
            header,
            "\t}}\timported_tables[{}];",
            ofs.num_imported_tables
        )?;
    }
    if ofs.num_imported_memories > 0 {
        writeln!(header, "\tstruct {{")?;
        writeln!(header, "\t\tvoid\t*from;")?;
        writeln!(header, "\t\tvoid\t*vmctx;")?;
        writeln!(
            header,
            "\t}}\timported_memories[{}];",
            ofs.num_imported_memories
        )?;
    }
    if ofs.num_imported_globals > 0 {
        writeln!(
            header,
            "\tvoid\t*imported_globals[{}];",
            ofs.num_imported_globals
        )?;
    }
    if ofs.num_defined_tables > 0 {
        writeln!(header, "\tstruct {{")?;
        writeln!(header, "\t\tvoid\t**base;")?;
        writeln!(header, "\t\tsize_t\tcurrent_elements;")?;
        writeln!(header, "\t}}\tdefined_tables[{}];", ofs.num_defined_tables)?;
    }
    if ofs.num_defined_memories > 0 {
        writeln!(header, "\tstruct {{")?;
        writeln!(header, "\t\tvoid\t*base;")?;
        writeln!(header, "\t\tsize_t\tcurrent_length;")?;
        writeln!(
            header,
            "\t}}\tdefined_memories[{}];",
            ofs.num_defined_memories
        )?;
    }
    if ofs.num_defined_globals > 0 {
        writeln!(header, "\tstruct {{")?;
        writeln!(header, "\t\tunion {{")?;
        writeln!(header, "\t\t\tuint32_t\tu32;")?;
        writeln!(header, "\t\t\tuint64_t\tu64;")?;
        writeln!(header, "\t\t\tfloat\tf32;")?;
        writeln!(header, "\t\t\tdouble\tf64;")?;
        writeln!(header, "\t\t}};")?;
        writeln!(
            header,
            "\t}}\tdefined_globals[{}];",
            ofs.num_defined_globals
        )?;
    }
    writeln!(header, "}};")?;
    writeln!(header)?;
    for i in 0..module.functions.len() {
        if i < ofs.num_imported_functions as usize {
            continue;
        }
        let signature = &module.signatures[module.functions[FuncIndex::from_u32(i as u32)]];
        let get_type = |x: &ir::AbiParam| -> &str {
            match x.value_type {
                ir::types::I32 => "uint32_t",
                ir::types::I64 => "uint64_t",
                _ => panic!("unsupported type"),
            }
        };
        let mut first = true;
        for p in signature.returns.iter() {
            if p.purpose != ir::ArgumentPurpose::Normal {
                continue;
            }
            assert!(first);
            first = false;
            write!(header, "{}", get_type(p))?;
        }
        if first {
            write!(header, "void")?;
        }
        write!(header, " _wasm_function_{}(", i)?;
        let mut first = true;
        for p in signature.params.iter() {
            if first {
                first = false;
            } else {
                write!(header, ", ")?;
            }
            match p.purpose {
                ir::ArgumentPurpose::VMContext => {
                    write!(header, "struct VMContext*")?;
                }
                ir::ArgumentPurpose::Normal => {
                    write!(header, "{}", get_type(p))?;
                }
                _ => panic!("unsupported param type: {:?}", p.purpose),
            }
        }
        writeln!(header, ");")?
    }
    if data_initializers.len() > 0 {
        writeln!(header)?;
        for i in 0..data_initializers.len() {
            writeln!(header, "extern void *_memory_{};", i)?;
        }
    }
    writeln!(header)?;
    writeln!(
        header,
        "extern char _vmcontext_init[{}];",
        ofs.size_of_vmctx()
    )?;
    writeln!(header, "void init_vm(struct VMContext *, void *, size_t);")?;
    writeln!(header, "#endif // __{}_H", uppercase_name)?;

    writeln!(code, "#include \"{}.h\"", name)?;
    writeln!(code)?;
    writeln!(code, "#include <string.h>")?;
    writeln!(code)?;
    writeln!(
        code,
        "void init_vm(struct VMContext *vmctx, void *memory, size_t memory_len) {{"
    )?;
    writeln!(
        code,
        "\tmemcpy(vmctx, _vmcontext_init, sizeof(_vmcontext_init));"
    )?;
    if ofs.num_defined_memories > 0 {
        writeln!(code, "\tvmctx->defined_memories[0].base = memory;")?;
        writeln!(
            code,
            "\tvmctx->defined_memories[0].current_length = memory_len;"
        )?;
    }

    if data_initializers.len() > 0 {
        writeln!(code)?;
        for i in 0..data_initializers.len() {
            let data_initializer = &data_initializers[i];
            let offset = {
                let addend = data_initializer.location.offset;
                if let Some(global_index) = data_initializer.location.base {
                    format!(
                        "vmctx->defined_globals[{}].u32 + {}",
                        global_index.index(),
                        addend
                    )
                } else {
                    format!("{}", addend)
                }
            };
            let len = data_initializer.data.len();
            writeln!(
                code,
                "\tmemcpy(memory + {}, _memory_{}, {});",
                offset, i, len
            )?;
        }
    }

    writeln!(code, "}};")?;
    Ok(())
}
