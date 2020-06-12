use anyhow::{anyhow, bail, Context as _, Result};
use object::write::Object;
use target_lexicon::Triple;
use wasmtime::Strategy;
use wasmtime_debug::{emit_dwarf, read_debuginfo, write_debugsections};
#[cfg(feature = "lightbeam")]
use wasmtime_environ::Lightbeam;
use wasmtime_environ::{
    entity::EntityRef, settings, settings::Configurable, wasm::DefinedMemoryIndex,
    wasm::MemoryIndex, CacheConfig, Compiler, Cranelift, ModuleEnvironment, ModuleMemoryOffset,
    ModuleVmctxInfo, Tunables, VMOffsets,
};
use wasmtime_jit::native;
use wasmtime_obj::emit_module;

fn to_obj_format(
    triple: &Triple,
) -> Result<(
    object::BinaryFormat,
    object::Architecture,
    object::Endianness,
)> {
    let binary_format = match triple.binary_format {
        target_lexicon::BinaryFormat::Elf => object::BinaryFormat::Elf,
        target_lexicon::BinaryFormat::Coff => object::BinaryFormat::Coff,
        target_lexicon::BinaryFormat::Macho => object::BinaryFormat::MachO,
        target_lexicon::BinaryFormat::Wasm => {
            bail!("binary format wasm is unsupported");
        }
        target_lexicon::BinaryFormat::Unknown => {
            bail!("binary format is unknown");
        }
    };
    let architecture = match triple.architecture {
        target_lexicon::Architecture::I386
        | target_lexicon::Architecture::I586
        | target_lexicon::Architecture::I686 => object::Architecture::I386,
        target_lexicon::Architecture::X86_64 => object::Architecture::X86_64,
        target_lexicon::Architecture::Arm(_) => object::Architecture::Arm,
        target_lexicon::Architecture::Aarch64(_) => object::Architecture::Aarch64,
        architecture => {
            bail!("target architecture {:?} is unsupported", architecture,);
        }
    };
    let endian = match triple.endianness().unwrap() {
        target_lexicon::Endianness::Little => object::Endianness::Little,
        target_lexicon::Endianness::Big => object::Endianness::Big,
    };
    Ok((binary_format, architecture, endian))
}

/// Creates object file from binary wasm data.
pub fn compile_to_obj(
    wasm: &[u8],
    target: Option<&Triple>,
    strategy: Strategy,
    enable_simd: bool,
    opt_level: wasmtime::OptLevel,
    debug_info: bool,
    cache_config: &CacheConfig,
) -> Result<Object> {
    let isa_builder = match target {
        Some(target) => native::lookup(target.clone())?,
        None => native::builder(),
    };
    let mut flag_builder = settings::builder();

    // There are two possible traps for division, and this way
    // we get the proper one if code traps.
    flag_builder.enable("avoid_div_traps").unwrap();

    if enable_simd {
        flag_builder.enable("enable_simd").unwrap();
    }

    match opt_level {
        wasmtime::OptLevel::None => {}
        wasmtime::OptLevel::Speed => {
            flag_builder.set("opt_level", "speed").unwrap();
        }
        wasmtime::OptLevel::SpeedAndSize => {
            flag_builder.set("opt_level", "speed_and_size").unwrap();
        }
        other => bail!("unknown optimization level {:?}", other),
    }

    let isa = isa_builder.finish(settings::Flags::new(flag_builder));

    let (obj_format, obj_arch, obj_endian) = to_obj_format(isa.triple())?;
    let mut obj = Object::new(obj_format, obj_arch, obj_endian);

    // TODO: Expose the tunables as command-line flags.
    let mut tunables = Tunables::default();
    tunables.debug_info = debug_info;

    let environ = ModuleEnvironment::new(isa.frontend_config(), &tunables);

    let translation = environ
        .translate(wasm)
        .context("failed to translate module")?;

    // TODO: use the traps information
    let (compilation, relocations, address_transform, value_ranges, stack_slots, _traps) =
        match strategy {
            Strategy::Auto | Strategy::Cranelift => {
                Cranelift::compile_module(&translation, &*isa, cache_config)
            }
            #[cfg(feature = "lightbeam")]
            Strategy::Lightbeam => Lightbeam::compile_module(&translation, &*isa, cache_config),
            #[cfg(not(feature = "lightbeam"))]
            Strategy::Lightbeam => bail!("lightbeam support not enabled"),
            other => bail!("unsupported compilation strategy {:?}", other),
        }
        .context("failed to compile module")?;

    if compilation.is_empty() {
        bail!("no functions were found/compiled");
    }

    let module_vmctx_info = {
        let ofs = VMOffsets::new(
            translation.target_config.pointer_bytes(),
            &translation.module.local,
        );
        ModuleVmctxInfo {
            memory_offset: if ofs.num_imported_memories > 0 {
                ModuleMemoryOffset::Imported(ofs.vmctx_vmmemory_import(MemoryIndex::new(0)))
            } else if ofs.num_defined_memories > 0 {
                ModuleMemoryOffset::Defined(
                    ofs.vmctx_vmmemory_definition_base(DefinedMemoryIndex::new(0)),
                )
            } else {
                ModuleMemoryOffset::None
            },
            stack_slots,
        }
    };

    emit_module(
        &mut obj,
        &translation.module,
        &compilation,
        &relocations,
        &translation.data_initializers,
        &translation.target_config,
    )
    .map_err(|e| anyhow!(e))
    .context("failed to emit module")?;

    if debug_info {
        let debug_data = read_debuginfo(wasm).context("failed to emit DWARF")?;
        let sections = emit_dwarf(
            &*isa,
            &debug_data,
            &address_transform,
            &module_vmctx_info,
            &value_ranges,
            &compilation,
        )
        .context("failed to emit debug sections")?;
        write_debugsections(&mut obj, sections).context("failed to emit debug sections")?;
    }
    Ok(obj)
}
