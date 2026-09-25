//! > **⚠️ Warning ⚠️**: this crate is an internal-only crate for the Wasmtime
//! > project and is not intended for general use. APIs are not strictly
//! > reviewed for safety and usage outside of Wasmtime may have bugs. If
//! > you're interested in using this feel free to file an issue on the
//! > Wasmtime repository to start a discussion about doing so, but otherwise
//! > be aware that your usage of this crate is not supported.

use capstone::arch::BuildsCapstone;
use serde_derive::Serialize;
use std::{
    collections::HashMap,
    fs::File,
    io::{Write, read_to_string},
    path::{Path, PathBuf},
    str::FromStr,
};
use wasmtime::{ModuleFunction, Result, ToWasmtimeResult as _};
use wasmtime_environ::{demangle_function_name, wasmparser};

pub fn generate(
    config: &wasmtime::Config,
    target: Option<&str>,
    clif_dir: Option<&Path>,
    wasm: &[u8],
    dest: &mut dyn Write,
) -> Result<()> {
    let target = match target {
        None => target_lexicon::Triple::host(),
        Some(target) => target_lexicon::Triple::from_str(target)?,
    };

    let wat = annotate_wat(wasm)?;
    let wat_json = serde_json::to_string(&wat)?;
    let asm = annotate_asm(config, &target, wasm)?;
    let asm_json = serde_json::to_string(&asm)?;
    let clif_json = clif_dir
        .map::<wasmtime::Result<String>, _>(|clif_dir| {
            let clif = annotate_clif(clif_dir, &asm)?;
            Ok(serde_json::to_string(&clif)?)
        })
        .transpose()?;

    let index_css = include_str!("./index.css");
    let index_js = include_str!("./index.js");

    write!(
        dest,
        r#"
<!DOCTYPE html>
<html>
  <head>
    <title>Wasmtime Compiler Explorer</title>
    <style>
      {index_css}
    </style>
  </head>
  <body class="hbox">
    <pre id="wat"></pre>
        "#
    )?;
    if clif_json.is_some() {
        write!(dest, r#"<div id="clif"></div>"#)?;
    }
    write!(
        dest,
        r#"
    <div id="asm"></div>
    <script>
      window.WAT = {wat_json};
        "#
    )?;
    if let Some(clif_json) = clif_json {
        write!(
            dest,
            r#"
          window.CLIF = {clif_json};
            "#
        )?;
    }
    write!(
        dest,
        r#"
      window.ASM = {asm_json};
    </script>
    <script>
      {index_js}
    </script>
  </body>
</html>
        "#
    )?;
    Ok(())
}

#[derive(Serialize, Clone, Copy, Debug)]
struct WasmOffset(u32);

#[derive(Serialize, Debug)]
struct AnnotatedWat {
    chunks: Vec<AnnotatedWatChunk>,
}

#[derive(Serialize, Debug)]
struct AnnotatedWatChunk {
    wasm_offset: Option<WasmOffset>,
    wat: String,
}

fn annotate_wat(wasm: &[u8]) -> Result<AnnotatedWat> {
    let printer = wasmprinter::Config::new();
    let mut storage = String::new();
    let chunks = printer
        .offsets_and_lines(wasm, &mut storage)
        .to_wasmtime_result()?
        .map(|(offset, wat)| AnnotatedWatChunk {
            wasm_offset: offset.map(|o| WasmOffset(u32::try_from(o).unwrap())),
            wat: wat.to_string(),
        })
        .collect();
    Ok(AnnotatedWat { chunks })
}

#[derive(Serialize, Debug)]
struct AnnotatedAsm {
    functions: Vec<AnnotatedFunction>,
}

#[derive(Serialize, Debug)]
struct AnnotatedFunction {
    module_index: u32,
    func_index: u32,
    name: Option<String>,
    demangled_name: Option<String>,
    instructions: Vec<AnnotatedInstruction>,
}

#[derive(Serialize, Debug)]
struct AnnotatedInstruction {
    wasm_offset: Option<WasmOffset>,
    address: u32,
    bytes: Vec<u8>,
    mnemonic: Option<String>,
    operands: Option<String>,
}

enum CompilationArtifact {
    Module(wasmtime::Module),
    Component(wasmtime::component::Component),
}

impl CompilationArtifact {
    fn text(&self) -> &[u8] {
        match self {
            CompilationArtifact::Module(module) => module.text(),
            CompilationArtifact::Component(component) => component.text(),
        }
    }
    fn address_map(&self) -> Option<Box<dyn Iterator<Item = (usize, Option<u32>)> + '_>> {
        match self {
            CompilationArtifact::Module(module) => module
                .address_map()
                .map(|i| Box::new(i) as Box<dyn Iterator<Item = _>>),
            CompilationArtifact::Component(component) => component
                .address_map()
                .map(|i| Box::new(i) as Box<dyn Iterator<Item = _>>),
        }
    }
    fn functions(&self) -> Box<dyn Iterator<Item = ModuleFunction> + '_> {
        match self {
            CompilationArtifact::Module(module) => Box::new(module.functions()),
            CompilationArtifact::Component(component) => Box::new(component.functions()),
        }
    }
}
fn annotate_asm(
    config: &wasmtime::Config,
    target: &target_lexicon::Triple,
    wasm: &[u8],
) -> Result<AnnotatedAsm> {
    let engine = wasmtime::Engine::new(config)?;
    let artifact = if wasmparser::Parser::is_component(wasm) {
        CompilationArtifact::Component(wasmtime::component::Component::new(&engine, wasm)?)
    } else {
        CompilationArtifact::Module(wasmtime::Module::new(&engine, wasm)?)
    };

    let text = artifact.text();
    let address_map: Vec<_> = artifact
        .address_map()
        .ok_or_else(|| wasmtime::format_err!("address maps must be enabled in the config"))?
        .collect();

    let mut address_map_iter = address_map.into_iter().peekable();
    let mut current_entry = address_map_iter.next();
    let mut wasm_offset_for_address = |start: usize, address: u32| -> Option<WasmOffset> {
        // Consume any entries that happened before the current function for the
        // first instruction.
        while current_entry.map_or(false, |cur| cur.0 < start) {
            current_entry = address_map_iter.next();
        }

        // Next advance the address map up to the current `address` specified,
        // including it.
        while address_map_iter.peek().map_or(false, |next_entry| {
            u32::try_from(next_entry.0).unwrap() <= address
        }) {
            current_entry = address_map_iter.next();
        }
        current_entry.and_then(|entry| entry.1.map(WasmOffset))
    };

    let functions = artifact
        .functions()
        .map(|function| {
            let body = &text[function.offset..][..function.len];

            let mut cs = match target.architecture {
                target_lexicon::Architecture::Aarch64(_) => capstone::Capstone::new()
                    .arm64()
                    .mode(capstone::arch::arm64::ArchMode::Arm)
                    .build()
                    .map_err(|e| wasmtime::format_err!("{e}"))?,
                target_lexicon::Architecture::Riscv64(_) => capstone::Capstone::new()
                    .riscv()
                    .mode(capstone::arch::riscv::ArchMode::RiscV64)
                    .build()
                    .map_err(|e| wasmtime::format_err!("{e}"))?,
                target_lexicon::Architecture::S390x => capstone::Capstone::new()
                    .sysz()
                    .mode(capstone::arch::sysz::ArchMode::Default)
                    .build()
                    .map_err(|e| wasmtime::format_err!("{e}"))?,
                target_lexicon::Architecture::X86_64 => capstone::Capstone::new()
                    .x86()
                    .mode(capstone::arch::x86::ArchMode::Mode64)
                    .build()
                    .map_err(|e| wasmtime::format_err!("{e}"))?,
                _ => wasmtime::bail!("Unsupported target: {target}"),
            };

            // This tells capstone to skip over anything that looks like data,
            // such as inline constant pools and things like that. This also
            // additionally is required to skip over trapping instructions on
            // AArch64.
            cs.set_skipdata(true).unwrap();

            let instructions = cs
                .disasm_all(body, function.offset as u64)
                .map_err(|e| wasmtime::format_err!("{e}"))?;
            let instructions = instructions
                .iter()
                .map(|inst| {
                    let address = u32::try_from(inst.address()).unwrap();
                    let wasm_offset = wasm_offset_for_address(function.offset, address);
                    Ok(AnnotatedInstruction {
                        wasm_offset,
                        address,
                        bytes: inst.bytes().to_vec(),
                        mnemonic: inst.mnemonic().map(ToString::to_string),
                        operands: inst.op_str().map(ToString::to_string),
                    })
                })
                .collect::<Result<Vec<_>>>()?;

            let demangled_name = if let Some(name) = &function.name {
                let mut demangled = String::new();
                if demangle_function_name(&mut demangled, &name).is_ok() {
                    Some(demangled)
                } else {
                    None
                }
            } else {
                None
            };

            Ok(AnnotatedFunction {
                module_index: function.module.as_u32(),
                func_index: function.index.as_u32(),
                name: function.name,
                demangled_name,
                instructions,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(AnnotatedAsm { functions })
}

#[derive(Serialize, Debug)]
struct AnnotatedClif {
    functions: Vec<AnnotatedClifFunction>,
}

#[derive(Serialize, Debug)]
struct AnnotatedClifFunction {
    module_index: u32,
    func_index: u32,
    name: Option<String>,
    demangled_name: Option<String>,
    instructions: Vec<AnnotatedClifInstruction>,
}

#[derive(Serialize, Debug)]
struct AnnotatedClifInstruction {
    wasm_offset: Option<WasmOffset>,
    clif: String,
}

fn annotate_clif(clif_dir: &Path, asm: &AnnotatedAsm) -> Result<AnnotatedClif> {
    let clif_files = ClifFiles::new(clif_dir)?;
    let mut clif = AnnotatedClif {
        functions: Vec::new(),
    };
    for function in &asm.functions {
        let Some(function_path) = clif_files.get(function.module_index, function.func_index) else {
            continue;
        };
        let mut clif_function = AnnotatedClifFunction {
            module_index: function.module_index,
            func_index: function.func_index,
            name: function.name.clone(),
            demangled_name: function.demangled_name.clone(),
            instructions: Vec::new(),
        };
        let file = File::open(&function_path)?;
        for mut line in read_to_string(file)?.lines() {
            if line.is_empty() {
                continue;
            }
            let mut wasm_offset = None;
            if line.starts_with('@') {
                wasm_offset = Some(WasmOffset(u32::from_str_radix(&line[1..5], 16)?));
                line = &line[28..];
            } else if line.starts_with("     ") {
                line = &line[28..];
            }
            clif_function.instructions.push(AnnotatedClifInstruction {
                wasm_offset,
                clif: line.to_string(),
            });
        }
        clif.functions.push(clif_function);
    }
    Ok(clif)
}

/// Cache the mapping from module + function indices to clif file path.
struct ClifFiles {
    paths: HashMap<(u32, u32), PathBuf>,
}

impl ClifFiles {
    fn new(clif_dir: &Path) -> Result<Self> {
        let mut paths = HashMap::new();
        for entry in clif_dir.read_dir()? {
            let path = entry?.path();
            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let Some(key) = parse_clif_filename(file_name) else {
                continue;
            };
            if let Some(previous) = paths.insert(key, path.clone()) {
                panic!(
                    "multiple CLIF files for wasm[{}]::function[{}]: {:?} and {:?}",
                    key.0, key.1, previous, path,
                );
            }
        }
        Ok(Self { paths })
    }

    fn get(&self, module_index: u32, func_index: u32) -> Option<&Path> {
        self.paths
            .get(&(module_index, func_index))
            .map(PathBuf::as_path)
    }
}

/// Parse the expected filename `wasm[<mod>]--function[<fn>]--<name>.clif` to
/// extract the indices <mod> and <fn>.
fn parse_clif_filename(file_name: &str) -> Option<(u32, u32)> {
    let file_name = file_name.strip_suffix(".clif")?;
    let file_name = file_name.strip_prefix("wasm[")?;
    let (module_index, file_name) = file_name.split_once("]--function[")?;
    let (func_index, name) = file_name.split_once(']')?;
    if !name.is_empty() && !name.starts_with("--") {
        return None;
    }
    Some((module_index.parse().ok()?, func_index.parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::{ClifFiles, parse_clif_filename};
    use tempfile::tempdir;

    #[test]
    fn parses_clif_filenames_for_modules_and_components() {
        assert_eq!(
            parse_clif_filename("wasm[0]--function[3].clif"),
            Some((0, 3))
        );
        assert_eq!(
            parse_clif_filename("wasm[4]--function[27]--some-name.clif"),
            Some((4, 27))
        );
        assert_eq!(
            parse_clif_filename("array_to_wasm_trampoline[0].clif"),
            None
        );
    }

    #[test]
    fn finds_emitted_module_clif() {
        let clif_dir = tempdir().unwrap();
        let mut config = wasmtime::Config::new();
        config.emit_clif(clif_dir.path());
        let engine = wasmtime::Engine::new(&config).unwrap();
        let wasm = wat::parse_str("(module (func $f) (func $g))").unwrap();
        wasmtime::Module::new(&engine, wasm).unwrap();

        let files = ClifFiles::new(clif_dir.path()).unwrap();
        assert!(files.get(0, 0).is_some());
        assert!(files.get(0, 1).is_some());
        assert!(files.get(1, 0).is_none());
        assert!(files.get(0, 2).is_none());
    }

    #[test]
    fn finds_emitted_component_clif() {
        let clif_dir = tempdir().unwrap();
        let mut config = wasmtime::Config::new();
        config.emit_clif(clif_dir.path());
        let engine = wasmtime::Engine::new(&config).unwrap();
        let wasm = wat::parse_str(
            "(component
                (core module (func $first))
                (core module (func $second) (func $third))
            )",
        )
        .unwrap();
        wasmtime::component::Component::new(&engine, wasm).unwrap();

        let files = ClifFiles::new(clif_dir.path()).unwrap();
        assert!(files.get(0, 0).is_some());
        assert!(files.get(1, 0).is_some());
        assert!(files.get(1, 1).is_some());
        assert!(files.get(0, 1).is_none());
    }
}
