//! Implementation of the `wasmtime hot-blocks` subcommand.

use crate::common::{RunCommon, RunTarget};
use capstone::InsnGroupType::{CS_GRP_JUMP, CS_GRP_RET};
use capstone::arch::BuildsCapstone;
use clap::Parser;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use tempfile::tempdir;
use wasmtime::{
    CodeBuilder, CodeHint, Engine, FuncIndex, ModuleFunction, Result, StaticModuleIndex, bail,
    error::Context as _, format_err,
};

/// Profile a WebAssembly module or component's execution and print the hottest
/// basic blocks.
///
/// This command compiles the given Wasm module/component, runs it under `perf
/// record`, and then analyzes the resulting profile to find the hottest basic
/// blocks in the compiled code. Each basic block is printed with its assembly,
/// CLIF IR, and original Wasm instructions.
///
/// This subcommand is only available on Linux.
#[derive(Parser)]
#[command(name = "hot-blocks")]
pub struct HotBlocksCommand {
    #[command(flatten)]
    run: RunCommon,

    /// Print the hottest basic blocks that cover at least this percent of
    /// total execution samples.
    ///
    /// Must be a number between 0 and 100 inclusive.
    #[clap(short, long, default_value = "50")]
    percent: f64,

    /// The kind of perf event to record.
    #[clap(short, long, value_enum, default_value = "cpu-cycles")]
    event: Event,

    /// The sampling frequency to use with `perf record -F`.
    ///
    /// Higher values give more samples but may slow execution.
    #[clap(short = 'F', long)]
    frequency: Option<u64>,

    /// The file to write the output to. When omitted, output goes to stdout.
    #[clap(short, long)]
    output: Option<PathBuf>,

    /// The WebAssembly module or component to profile.
    #[arg(required = true, value_name = "MODULE")]
    module: PathBuf,

    /// Arguments to pass to the WebAssembly module.
    #[arg(trailing_var_arg = true)]
    module_args: Vec<String>,
}

/// The kind of perf event to record.
#[derive(Clone, Debug, clap::ValueEnum)]
pub enum Event {
    /// Record instructions retired.
    ///
    /// Corresponds to `perf record -e instructions`.
    Instructions,
    /// Record CPU cycles.
    ///
    /// Corresponds to `perf record -e cpu-cycles`.
    CpuCycles,
}

impl Event {
    fn perf_event(&self) -> &str {
        match self {
            Event::Instructions => "instructions",
            Event::CpuCycles => "cpu-cycles",
        }
    }
}

/// A zero-based index into the list of basic blocks for a function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct BlockIndex(usize);

/// A byte offset into a function's compiled code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct FunctionOffset(usize);

impl HotBlocksCommand {
    /// Executes the command.
    pub fn execute(mut self) -> Result<()> {
        self.run.common.init_logging()?;

        if !(0.0..=100.0).contains(&self.percent) {
            bail!("--percent must be between 0 and 100 inclusive");
        }

        // Ensure address maps are enabled (error if explicitly disabled).
        if self.run.common.debug.address_map == Some(false) {
            bail!(
                "address maps must be enabled for hot-blocks profiling; do not pass -Daddress-map=n"
            );
        }
        self.run.common.debug.address_map = Some(true);

        let tmp_dir = tempdir().context("failed to create temp directory")?;

        // Compile the input Wasm to a .cwasm, emitting CLIF to the temp dir.
        let clif_dir = tmp_dir.path().join("clif");
        std::fs::create_dir(&clif_dir)?;
        let cwasm_path = tmp_dir.path().join("module.cwasm");

        let wasm_bytes =
            Cow::Owned(std::fs::read(&self.module).with_context(|| {
                format!("failed to read Wasm module: {}", self.module.display())
            })?);
        #[cfg(feature = "wat")]
        let wasm_bytes = wat::parse_bytes(&wasm_bytes).map_err(|mut e| {
            e.set_path(&self.module);
            e
        })?;

        let engine = self.compile_to_cwasm(&clif_dir, &cwasm_path, &wasm_bytes)?;

        // Run perf record.
        let perf_data_path = tmp_dir.path().join("perf.data");
        self.run_perf_record(&cwasm_path, &perf_data_path)?;

        // Run perf script and parse samples.
        let samples = self.run_perf_script(&perf_data_path)?;

        let target = match self.run.common.target.as_deref() {
            None => target_lexicon::Triple::host(),
            Some(t) => target_lexicon::Triple::from_str(t)?,
        };

        // Build the WAT offset map using wasmprinter.
        let wat_map = build_wat_offset_map(&wasm_bytes);

        // Deserialize the cwasm to extract functions, text, and address map.
        self.run.allow_precompiled = true;
        let run_target = self.run.load_module(&engine, &cwasm_path, None)?;
        let (functions, text, address_map) = match &run_target {
            RunTarget::Core(module) => (
                module.functions().collect::<Vec<_>>(),
                module.text(),
                module
                    .address_map()
                    .ok_or_else(|| {
                        format_err!("address maps are not available in the compiled module")
                    })?
                    .collect::<Vec<_>>(),
            ),
            #[cfg(feature = "component-model")]
            RunTarget::Component(component) => (
                component.functions().collect::<Vec<_>>(),
                component.text(),
                component
                    .address_map()
                    .ok_or_else(|| {
                        format_err!("address maps are not available in the compiled component")
                    })?
                    .collect::<Vec<_>>(),
            ),
        };

        let mut output: Box<dyn Write> = match &self.output {
            Some(path) => {
                let file = std::fs::File::create(path)
                    .with_context(|| format!("failed to create output file: {}", path.display()))?;
                Box::new(BufWriter::new(file))
            }
            None => Box::new(io::stdout()),
        };

        self.format_hot_blocks(
            &samples,
            &functions,
            &text,
            &address_map,
            &clif_dir,
            &wat_map,
            &target,
            &mut *output,
        )?;

        Ok(())
    }

    /// Compile the input Wasm bytes to a `.cwasm` file, emitting CLIF to `clif_dir`.
    ///
    /// Returns the engine used for compilation.
    fn compile_to_cwasm(
        &mut self,
        clif_dir: &Path,
        cwasm_path: &Path,
        wasm_bytes: &[u8],
    ) -> Result<Engine> {
        let mut config = self.run.common.config(None)?;
        config.emit_clif(clif_dir);

        let engine = Engine::new(&config)?;

        let mut code = CodeBuilder::new(&engine);
        code.wasm_binary_or_text(wasm_bytes, Some(&self.module))?;

        let serialized = match code.hint() {
            #[cfg(feature = "component-model")]
            Some(CodeHint::Component) => code.compile_component_serialized()?,
            #[cfg(not(feature = "component-model"))]
            Some(CodeHint::Component) => {
                bail!("component model support was disabled at compile time")
            }
            Some(CodeHint::Module) | None => code.compile_module_serialized()?,
        };
        std::fs::write(cwasm_path, &serialized)
            .with_context(|| format!("failed to write cwasm: {}", cwasm_path.display()))?;

        Ok(engine)
    }

    /// Run `perf record` on the compiled `.cwasm` file.
    fn run_perf_record(&self, cwasm_path: &Path, perf_data_path: &Path) -> Result<()> {
        let current_exe =
            std::env::current_exe().context("failed to determine current executable")?;

        let mut perf_cmd = Command::new("perf");
        perf_cmd
            .arg("record")
            .arg("-e")
            .arg(self.event.perf_event())
            .arg("--no-buildid")
            .arg("-o")
            .arg(perf_data_path);

        if let Some(freq) = self.frequency {
            perf_cmd.arg("-F").arg(freq.to_string());
        }

        perf_cmd
            .arg("--")
            .arg(&current_exe)
            .arg("run")
            .arg("--allow-precompiled")
            .arg("--profile=perfmap");

        // Forward run flags to the nested `wasmtime run` subprocess.
        for (host, guest) in &self.run.dirs {
            perf_cmd.arg("--dir").arg(format!("{host}::{guest}"));
        }
        for (key, value) in &self.run.vars {
            match value {
                Some(val) => perf_cmd.arg("--env").arg(format!("{key}={val}")),
                None => perf_cmd.arg("--env").arg(key),
            };
        }
        if self.run.common.wasm.unknown_imports_trap == Some(true) {
            perf_cmd.arg("-Wunknown-imports-trap");
        }
        if self.run.common.wasm.unknown_imports_default == Some(true) {
            perf_cmd.arg("-Wunknown-imports-default");
        }

        perf_cmd.arg(cwasm_path.as_os_str());
        for arg in &self.module_args {
            perf_cmd.arg(arg);
        }

        let perf_output = perf_cmd
            .output()
            .context("failed to run `perf record`; is `perf` installed?")?;
        if !perf_output.status.success() {
            let stderr = String::from_utf8_lossy(&perf_output.stderr);
            bail!("perf record failed:\n{stderr}");
        }

        Ok(())
    }

    /// Run `perf script` and parse the output into samples.
    fn run_perf_script(&self, perf_data_path: &Path) -> Result<Vec<PerfSample>> {
        let perf_script_output = Command::new("perf")
            .arg("script")
            .arg("-i")
            .arg(perf_data_path)
            .arg("-F")
            .arg("ip,sym,symoff,dso")
            .output()
            .context("failed to run `perf script`")?;
        if !perf_script_output.status.success() {
            let stderr = String::from_utf8_lossy(&perf_script_output.stderr);
            bail!("perf script failed:\n{stderr}");
        }

        let script_text = String::from_utf8_lossy(&perf_script_output.stdout);
        Ok(parse_perf_script(&script_text))
    }

    /// Format hot blocks output.
    fn format_hot_blocks(
        &self,
        samples: &[PerfSample],
        functions: &[ModuleFunction],
        text: &[u8],
        address_map: &[(usize, Option<u32>)],
        clif_dir: &Path,
        wat_map: &BTreeMap<u32, String>,
        target: &target_lexicon::Triple,
        output: &mut dyn Write,
    ) -> Result<()> {
        let total_samples = samples.len();
        if total_samples == 0 {
            writeln!(output, "No samples collected within WebAssembly code.")?;
            return Ok(());
        }

        // Build a map from (module, func_index) to &ModuleFunction for fast lookups.
        let func_map: BTreeMap<(StaticModuleIndex, FuncIndex), &ModuleFunction> =
            functions.iter().map(|f| ((f.module, f.index), f)).collect();

        // For each function that has samples, build basic blocks lazily.
        let mut func_blocks: BTreeMap<(StaticModuleIndex, FuncIndex), Vec<BasicBlock>> =
            BTreeMap::new();

        // Count samples per (module, func_index, block_index).
        let mut block_samples: BTreeMap<(StaticModuleIndex, FuncIndex, BlockIndex), u64> =
            BTreeMap::new();
        // Also count samples per (module, func_index, block_index, offset_in_func).
        let mut inst_samples: BTreeMap<
            (StaticModuleIndex, FuncIndex, BlockIndex, FunctionOffset),
            u64,
        > = BTreeMap::new();

        for sample in samples {
            let Some(func) = find_function_for_sample(sample, functions) else {
                continue;
            };
            let key = (func.module, func.index);

            // Lazily build basic blocks for this function.
            let blocks = func_blocks.entry(key).or_insert_with(|| {
                let body = &text[func.offset..][..func.len];
                let clif_lines =
                    read_clif_file(clif_dir, func.module, func.index, func.name.as_deref());
                build_basic_blocks(body, func.offset, address_map, &clif_lines, wat_map, target)
                    .unwrap_or_default()
            });

            let offset_in_func = FunctionOffset(usize::try_from(sample.offset).unwrap());
            if let Some(block_idx) = find_block_for_offset(blocks, offset_in_func) {
                *block_samples.entry((key.0, key.1, block_idx)).or_default() += 1;
                *inst_samples
                    .entry((key.0, key.1, block_idx, offset_in_func))
                    .or_default() += 1;
            }
        }

        // Sort by most samples to least.
        let mut sorted_blocks: Vec<_> = block_samples.into_iter().collect();
        sorted_blocks.sort_by(|a, b| b.1.cmp(&a.1));

        let total_f64 = total_samples as f64;

        // Print hot blocks until we reach the percent threshold.
        let mut samples_printed: u64 = 0;
        for ((mod_idx, func_idx, block_idx), block_sample_count) in &sorted_blocks {
            let percent_printed = samples_printed as f64 / total_f64 * 100.0;
            if percent_printed >= self.percent {
                break;
            }

            let block_percent = *block_sample_count as f64 / total_f64 * 100.0;

            // Look up the function name from the map.
            let func_name = func_map
                .get(&(*mod_idx, *func_idx))
                .and_then(|f| f.name.clone())
                .unwrap_or_else(|| {
                    format!(
                        "wasm[{}]::function[{}]",
                        mod_idx.as_u32(),
                        func_idx.as_u32()
                    )
                });

            let blocks = func_blocks.get(&(*mod_idx, *func_idx)).unwrap();
            let block = &blocks[block_idx.0];

            // Trim leading instructions that have no samples.
            let first_sampled = block
                .instructions
                .iter()
                .position(|inst| {
                    inst_samples
                        .get(&(
                            *mod_idx,
                            *func_idx,
                            *block_idx,
                            FunctionOffset(inst.offset_in_func),
                        ))
                        .copied()
                        .unwrap_or(0)
                        > 0
                })
                .unwrap_or(0);
            let visible_instructions = &block.instructions[first_sampled..];

            writeln!(
                output,
                "`{func_name}` :: block {:#x} :: {block_percent:.2}% total samples",
                block.instructions[first_sampled].offset_in_func,
            )?;
            writeln!(output)?;

            // Calculate column widths.
            let max_asm_len = visible_instructions
                .iter()
                .map(|i| i.assembly.len())
                .max()
                .unwrap_or(10);
            let max_clif_len = visible_instructions
                .iter()
                .map(|i| i.clif.as_ref().map_or(1, |c| c.len()))
                .max()
                .unwrap_or(6);

            let asm_width = max_asm_len.max(10);
            let clif_width = max_clif_len.max(6);

            writeln!(
                output,
                "{:>10}   {:<asm_width$}   {:<clif_width$}   {}",
                "[Samples]", "[Assembly]", "[CLIF]", "[Wasm]"
            )?;

            let mut prev_clif: Option<(&str, Option<u32>)> = None;
            let mut prev_wasm: Option<(&str, Option<u32>)> = None;

            for inst in visible_instructions {
                let sample_count = inst_samples
                    .get(&(
                        *mod_idx,
                        *func_idx,
                        *block_idx,
                        FunctionOffset(inst.offset_in_func),
                    ))
                    .copied()
                    .unwrap_or(0);

                let sample_str = if sample_count > 0 {
                    format!("{:.2}%", sample_count as f64 / total_f64 * 100.0)
                } else {
                    String::new()
                };

                // Determine CLIF display, using ditto marks for repeated same-offset instructions.
                let clif_display = if let Some(ref clif_text) = inst.clif {
                    let current = (clif_text.as_str(), inst.wasm_offset);
                    if prev_clif == Some(current) {
                        "\"".to_string()
                    } else {
                        prev_clif = Some((clif_text.as_str(), inst.wasm_offset));
                        clif_text.clone()
                    }
                } else {
                    prev_clif = None;
                    "-".to_string()
                };

                // Determine Wasm display, using ditto marks for repeated same-offset instructions.
                let wasm_display = if let Some(ref wasm_text) = inst.wasm {
                    let current = (wasm_text.as_str(), inst.wasm_offset);
                    if prev_wasm == Some(current) {
                        "\"".to_string()
                    } else {
                        prev_wasm = Some((wasm_text.as_str(), inst.wasm_offset));
                        wasm_text.clone()
                    }
                } else {
                    prev_wasm = None;
                    "-".to_string()
                };

                writeln!(
                    output,
                    "{:>10}   {:<asm_width$}   {:<clif_width$}   {}",
                    sample_str, inst.assembly, clif_display, wasm_display
                )?;
            }
            writeln!(output)?;

            samples_printed += block_sample_count;
        }

        Ok(())
    }
}

/// A parsed sample from `perf script` output.
#[derive(Debug, Clone)]
struct PerfSample {
    /// The symbol name from perf (e.g., "wasm[0]::function[3]").
    symbol: String,
    /// The offset within the symbol.
    offset: u64,
}

/// Parse `perf script -F ip,sym,symoff,dso` output to extract samples that
/// come from a perf map (i.e. compiled WebAssembly code and trampolines).
fn parse_perf_script(output: &str) -> Vec<PerfSample> {
    let mut samples = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(sample) = parse_perf_script_line(line) {
            samples.push(sample);
        }
    }
    samples
}

fn parse_perf_script_line(line: &str) -> Option<PerfSample> {
    // perf script -F ip,sym,symoff,dso gives lines like:
    //   7f1234567890 wasm[0]::function[3]+0x10 (/tmp/perf-1234.map)
    //   7f1234567890 wasm[0]::function[3]+0x10 (/path/to/module.cwasm)
    // Filter by whether the DSO is a perf map or a cwasm file.
    let line = line.trim();

    // Check for a `.map)` or `.cwasm)` suffix.
    if !line.ends_with(".map)") && !line.ends_with(".cwasm)") {
        return None;
    }

    // Skip the instruction pointer prefix.
    let rest = line.trim_start_matches(|c: char| c.is_ascii_hexdigit() || c == ' ');

    // Find "symbol+0xoffset"
    let (sym_with_offset, _dso) = rest.split_once(" (").unwrap_or((rest, ""));
    let sym_with_offset = sym_with_offset.trim();

    let (symbol, offset_str) = sym_with_offset
        .rsplit_once('+')
        .unwrap_or((sym_with_offset, "0x0"));
    let offset_str = offset_str.trim_start_matches("0x").trim_start_matches("0X");
    let offset = u64::from_str_radix(offset_str, 16).unwrap_or(0);

    Some(PerfSample {
        symbol: symbol.to_string(),
        offset,
    })
}

/// An instruction within a basic block.
#[derive(Debug, Clone)]
struct BlockInstruction {
    /// Offset within the function.
    offset_in_func: usize,
    /// Assembly text (e.g., "movq [rbx], rcx").
    assembly: String,
    /// Associated CLIF text, if any.
    clif: Option<String>,
    /// The wasm bytecode offset for this instruction, if known.
    wasm_offset: Option<u32>,
    /// Associated Wasm text (WAT disassembly), if any.
    wasm: Option<String>,
}

/// A basic block in a compiled function.
#[derive(Debug, Clone)]
struct BasicBlock {
    /// Instructions in this block.
    instructions: Vec<BlockInstruction>,
}

/// Build a capstone disassembler for the given target architecture.
fn build_capstone(target: &target_lexicon::Triple) -> Result<capstone::Capstone> {
    let mut cs = match target.architecture {
        target_lexicon::Architecture::Aarch64(_) => capstone::Capstone::new()
            .arm64()
            .mode(capstone::arch::arm64::ArchMode::Arm)
            .detail(true)
            .build()
            .map_err(|e| format_err!("{e}"))?,
        target_lexicon::Architecture::Riscv64(_) => capstone::Capstone::new()
            .riscv()
            .mode(capstone::arch::riscv::ArchMode::RiscV64)
            .detail(true)
            .build()
            .map_err(|e| format_err!("{e}"))?,
        target_lexicon::Architecture::S390x => capstone::Capstone::new()
            .sysz()
            .mode(capstone::arch::sysz::ArchMode::Default)
            .detail(true)
            .build()
            .map_err(|e| format_err!("{e}"))?,
        target_lexicon::Architecture::X86_64 => capstone::Capstone::new()
            .x86()
            .mode(capstone::arch::x86::ArchMode::Mode64)
            .detail(true)
            .build()
            .map_err(|e| format_err!("{e}"))?,
        _ => bail!("unsupported target architecture: {target}"),
    };
    // Skip over anything that looks like data (inline constant pools, etc.).
    cs.set_skipdata(true).unwrap();
    Ok(cs)
}

/// Build basic blocks for a function by disassembling its code and splitting
/// at control flow boundaries.
fn build_basic_blocks(
    func_body: &[u8],
    func_offset: usize,
    address_map: &[(usize, Option<u32>)],
    clif_lines: &[(Option<u32>, String)],
    wat_map: &BTreeMap<u32, String>,
    target: &target_lexicon::Triple,
) -> Result<Vec<BasicBlock>> {
    let cs = build_capstone(target)?;

    let instructions = cs
        .disasm_all(func_body, u64::try_from(func_offset).unwrap())
        .map_err(|e| format_err!("{e}"))?;

    // Build a map from code offset -> wasm offset for instructions in this function.
    let mut offset_to_wasm: BTreeMap<usize, Option<u32>> = BTreeMap::new();
    for &(code_offset, wasm_offset) in address_map {
        if code_offset >= func_offset && code_offset < func_offset + func_body.len() {
            offset_to_wasm.insert(code_offset, wasm_offset);
        }
    }

    // Build a map from wasm offset -> CLIF text.
    let mut wasm_to_clif: BTreeMap<u32, Vec<&str>> = BTreeMap::new();
    for (wasm_off, clif_text) in clif_lines {
        if let Some(off) = wasm_off {
            wasm_to_clif.entry(*off).or_default().push(clif_text);
        }
    }

    // Build annotated instructions and identify block boundaries.
    let mut annotated = Vec::new();
    let mut is_block_end = Vec::new();

    for inst in instructions.iter() {
        let addr = usize::try_from(inst.address()).unwrap();
        let offset_in_func = addr - func_offset;

        let disassembly = match (inst.mnemonic(), inst.op_str()) {
            (Some(m), Some(o)) if !o.is_empty() => format!("{m:7} {o}"),
            (Some(m), _) => m.to_string(),
            _ => "<unknown>".to_string(),
        };

        // Find wasm offset for this instruction.
        let wasm_offset = find_wasm_offset_for_address(&offset_to_wasm, addr);

        // Find CLIF text for this wasm offset.
        let clif = wasm_offset
            .and_then(|wo| wasm_to_clif.get(&wo))
            .map(|lines| lines.join("; "));

        // Find Wasm text for this wasm offset from the WAT map.
        let wasm = wasm_offset.and_then(|wo| wat_map.get(&wo).cloned());

        annotated.push(BlockInstruction {
            offset_in_func,
            assembly: disassembly,
            clif,
            wasm_offset,
            wasm,
        });

        // Check if this instruction ends a basic block.
        let detail = cs.insn_detail(&inst).ok();
        let ends_block = detail
            .as_ref()
            .map(|d| {
                d.groups()
                    .iter()
                    .any(|g| u32::from(g.0) == CS_GRP_JUMP || u32::from(g.0) == CS_GRP_RET)
            })
            .unwrap_or(false);
        is_block_end.push(ends_block);
    }

    // Split into basic blocks.
    let mut blocks = Vec::new();
    let mut current_block = Vec::new();

    for (i, inst) in annotated.into_iter().enumerate() {
        current_block.push(inst);
        if is_block_end[i] {
            blocks.push(BasicBlock {
                instructions: std::mem::take(&mut current_block),
            });
        }
    }
    // Don't forget the last block if it didn't end with a branch.
    if !current_block.is_empty() {
        blocks.push(BasicBlock {
            instructions: current_block,
        });
    }

    Ok(blocks)
}

/// Find the wasm offset for a given code address by looking up the nearest
/// entry in the address map that is <= the address.
fn find_wasm_offset_for_address(
    offset_to_wasm: &BTreeMap<usize, Option<u32>>,
    addr: usize,
) -> Option<u32> {
    offset_to_wasm
        .range(..=addr)
        .next_back()
        .and_then(|(_, wasm_off)| *wasm_off)
}

/// Build a map from wasm bytecode offset to WAT disassembly text using wasmprinter.
fn build_wat_offset_map(wasm_bytes: &[u8]) -> BTreeMap<u32, String> {
    let mut map = BTreeMap::new();
    let printer = wasmprinter::Config::new();
    let mut storage = String::new();
    let Ok(chunks) = printer.offsets_and_lines(wasm_bytes, &mut storage) else {
        return map;
    };
    for (offset, wat_line) in chunks {
        if let Some(offset) = offset {
            let trimmed = wat_line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('(') && !trimmed.starts_with(')') {
                map.insert(u32::try_from(offset).unwrap(), trimmed.to_string());
            }
        }
    }
    map
}

/// Read CLIF file for a given function, returning pairs of
/// (wasm_offset, clif_line).
fn read_clif_file(
    clif_dir: &Path,
    mod_idx: StaticModuleIndex,
    func_index: FuncIndex,
    func_name: Option<&str>,
) -> Vec<(Option<u32>, String)> {
    let contents = find_and_read_clif(clif_dir, mod_idx, func_index, func_name);
    let Some(contents) = contents else {
        return Vec::new();
    };

    let mut result = Vec::new();
    for line in contents.lines() {
        if line.is_empty() {
            continue;
        }
        // CLIF lines come in these formats:
        //   "@0042                           v12 = load.i64 v10+8"  (with wasm offset)
        //   "                                v12 = ..."            (no wasm offset, 32-char indent)
        //   "block0(v0: i64, ...):"                                (block headers, etc.)
        let trimmed = line.trim_start();
        if trimmed.starts_with('@') {
            let offset = u32::from_str_radix(&trimmed[1..5], 16).ok();
            // Find the instruction text after the padding.
            let text = trimmed[5..].trim().to_string();
            result.push((offset, text));
        } else if line.starts_with(' ') {
            let text = trimmed.to_string();
            result.push((None, text));
        } else {
            result.push((None, trimmed.to_string()));
        }
    }
    result
}

/// Find and read a CLIF file for a function, using the naming convention from
/// `finish_with_info` in `crates/cranelift/src/compiler.rs`.
fn find_and_read_clif(
    clif_dir: &Path,
    mod_idx: StaticModuleIndex,
    func_index: FuncIndex,
    func_name: Option<&str>,
) -> Option<String> {
    let mod_idx = mod_idx.as_u32();
    let func_idx = func_index.as_u32();

    // Try with name: "wasm[N]--function[M]--name.clif"
    if let Some(name) = func_name {
        let short_name = name.rsplit("::").next().unwrap_or(name);
        let path = clif_dir.join(format!(
            "wasm[{mod_idx}]--function[{func_idx}]--{short_name}.clif"
        ));
        if let Ok(contents) = std::fs::read_to_string(&path) {
            return Some(contents);
        }
    }

    // Try without name: "wasm[N]--function[M].clif"
    let path = clif_dir.join(format!("wasm[{mod_idx}]--function[{func_idx}].clif"));
    if let Ok(contents) = std::fs::read_to_string(&path) {
        return Some(contents);
    }

    None
}

/// Parse a perfmap-style function name like "wasm[0]::function[3]" to extract
/// the module and function indices.
fn parse_wasm_func_name(name: &str) -> Option<(StaticModuleIndex, FuncIndex)> {
    // Pattern: "wasm[<module>]::function[<func>]"
    let rest = name.strip_prefix("wasm[")?;
    let (mod_idx_str, rest) = rest.split_once(']')?;
    let rest = rest.strip_prefix("::function[")?;
    let (func_idx_str, _) = rest.split_once(']')?;
    let mod_idx: u32 = mod_idx_str.parse().ok()?;
    let func_idx: u32 = func_idx_str.parse().ok()?;
    Some((
        StaticModuleIndex::from_u32(mod_idx),
        FuncIndex::from_u32(func_idx),
    ))
}

/// Match a perf sample's symbol to a ModuleFunction using binary search by
/// (module, func_index).
///
/// `functions` must be sorted by `(module, index)` (ascending), which is the
/// natural order since module and function indices increase monotonically.
fn find_function_for_sample<'a>(
    sample: &PerfSample,
    functions: &'a [ModuleFunction],
) -> Option<&'a ModuleFunction> {
    let (mod_idx, func_idx) = parse_wasm_func_name(&sample.symbol)?;
    functions
        .binary_search_by_key(&(mod_idx, func_idx), |f| (f.module, f.index))
        .ok()
        .map(|i| &functions[i])
}

/// Find which basic block an offset falls into, using binary search.
fn find_block_for_offset(
    blocks: &[BasicBlock],
    offset_in_func: FunctionOffset,
) -> Option<BlockIndex> {
    let idx = blocks
        .binary_search_by_key(&offset_in_func.0, |b| b.instructions[0].offset_in_func)
        .unwrap_or_else(|i| i.saturating_sub(1));
    let block = blocks.get(idx)?;
    let last_offset = block.instructions.last()?.offset_in_func;
    if offset_in_func.0 >= block.instructions[0].offset_in_func && offset_in_func.0 <= last_offset {
        Some(BlockIndex(idx))
    } else {
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_perf_script_line_map_dso() {
        let line = " 7f1234567890 wasm[0]::function[3]+0x10 (/tmp/perf-1234.map)";
        let sample = parse_perf_script_line(line).unwrap();
        assert_eq!(sample.symbol, "wasm[0]::function[3]");
        assert_eq!(sample.offset, 0x10);
    }

    #[test]
    fn test_parse_perf_script_line_no_offset() {
        let line = "7f1234567890 wasm[0]::function[0]+0x0 (/tmp/perf-123.map)";
        let sample = parse_perf_script_line(line).unwrap();
        assert_eq!(sample.symbol, "wasm[0]::function[0]");
        assert_eq!(sample.offset, 0);
    }

    #[test]
    fn test_parse_perf_script_line_non_map_dso() {
        // Non-.map / non-.cwasm DSO should be filtered out.
        let line = "7f1234567890 main+0x10 (/usr/bin/wasmtime)";
        assert!(parse_perf_script_line(line).is_none());
    }

    #[test]
    fn test_parse_perf_script_line_cwasm_dso() {
        let line = " 7f1234567890 wasm[0]::function[1]+0x20 (/tmp/.tmpABC123/module.cwasm)";
        let sample = parse_perf_script_line(line).unwrap();
        assert_eq!(sample.symbol, "wasm[0]::function[1]");
        assert_eq!(sample.offset, 0x20);
    }

    #[test]
    fn test_parse_perf_script_line_trampoline() {
        // Trampolines in perf maps should be captured.
        let line = "7f1234567890 trampoline+0x5 (/tmp/perf-1234.map)";
        let sample = parse_perf_script_line(line).unwrap();
        assert_eq!(sample.symbol, "trampoline");
        assert_eq!(sample.offset, 0x5);
    }

    #[test]
    fn test_parse_wasm_func_name() {
        assert_eq!(
            parse_wasm_func_name("wasm[0]::function[3]"),
            Some((StaticModuleIndex::from_u32(0), FuncIndex::from_u32(3)))
        );
        assert_eq!(
            parse_wasm_func_name("wasm[1]::function[42]"),
            Some((StaticModuleIndex::from_u32(1), FuncIndex::from_u32(42)))
        );
        assert_eq!(parse_wasm_func_name("main"), None);
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_mocked_hot_blocks() {
        // Build a tiny x86_64 function: two blocks
        //   Block 0: nop; nop; jmp +0 (falls through)
        //   Block 1: nop; ret
        let func_body: &[u8] = &[
            0x90, // nop          (offset 0)
            0x90, // nop          (offset 1)
            0xeb, 0x00, // jmp +0 (offset 2, 2 bytes) -> ends block 0
            0x90, // nop          (offset 4)
            0xc3, // ret          (offset 5) -> ends block 1
        ];

        let func_offset = 0x1000usize;

        let address_map = vec![
            (func_offset, Some(0x0010u32)),
            (func_offset + 1, Some(0x0011)),
            (func_offset + 2, Some(0x0012)),
            (func_offset + 4, Some(0x0013)),
            (func_offset + 5, Some(0x0014)),
        ];

        let clif_lines = vec![
            (Some(0x0010u32), "v1 = iconst.i32 0".to_string()),
            (Some(0x0011u32), "v2 = iconst.i32 1".to_string()),
            (Some(0x0012u32), "jump block1".to_string()),
            (Some(0x0013u32), "v3 = iadd v1, v2".to_string()),
            (Some(0x0014u32), "return v3".to_string()),
        ];

        let mut wat_map = BTreeMap::new();
        wat_map.insert(0x0010, "i32.const 0".to_string());
        wat_map.insert(0x0011, "i32.const 1".to_string());
        wat_map.insert(0x0012, "br 0".to_string());
        wat_map.insert(0x0013, "i32.add".to_string());
        wat_map.insert(0x0014, "return".to_string());

        let target = target_lexicon::Triple::host();
        let blocks = build_basic_blocks(
            func_body,
            func_offset,
            &address_map,
            &clif_lines,
            &wat_map,
            &target,
        )
        .unwrap();

        assert_eq!(blocks.len(), 2, "expected 2 basic blocks");
        assert_eq!(blocks[0].instructions.len(), 3, "block 0: nop, nop, jmp");
        assert_eq!(blocks[1].instructions.len(), 2, "block 1: nop, ret");

        // Verify CLIF annotations.
        assert_eq!(
            blocks[0].instructions[0].clif.as_deref(),
            Some("v1 = iconst.i32 0")
        );
        assert_eq!(blocks[1].instructions[1].clif.as_deref(), Some("return v3"));

        // Verify Wasm annotations come from WAT map.
        assert_eq!(
            blocks[0].instructions[0].wasm.as_deref(),
            Some("i32.const 0")
        );
        assert_eq!(blocks[1].instructions[1].wasm.as_deref(), Some("return"));

        // Test find_block_for_offset.
        assert_eq!(
            find_block_for_offset(&blocks, FunctionOffset(0)),
            Some(BlockIndex(0))
        );
        assert_eq!(
            find_block_for_offset(&blocks, FunctionOffset(1)),
            Some(BlockIndex(0))
        );
        assert_eq!(
            find_block_for_offset(&blocks, FunctionOffset(2)),
            Some(BlockIndex(0))
        );
        assert_eq!(
            find_block_for_offset(&blocks, FunctionOffset(4)),
            Some(BlockIndex(1))
        );
        assert_eq!(
            find_block_for_offset(&blocks, FunctionOffset(5)),
            Some(BlockIndex(1))
        );

        // Verify block instructions have assembly text.
        assert!(blocks[0].instructions[0].assembly.contains("nop"));
        assert!(blocks[0].instructions[2].assembly.contains("jmp"));
    }

    #[test]
    fn test_parse_perf_script() {
        let input = "\
 7f0001001000 wasm[0]::function[3]+0x0 (/tmp/perf-1234.map)
 7f0001001005 wasm[0]::function[3]+0x5 (/tmp/perf-1234.map)
 7f0001001000 wasm[0]::function[3]+0x0 (/tmp/perf-1234.map)
 7f0001002000 some_native_func+0x10 (/usr/bin/wasmtime)
 7f0001001010 wasm[0]::function[5]+0x10 (/tmp/perf-1234.map)
";
        let samples = parse_perf_script(input);
        // The native func line is filtered out (not a .map DSO).
        assert_eq!(samples.len(), 4);
        assert_eq!(samples[0].symbol, "wasm[0]::function[3]");
        assert_eq!(samples[0].offset, 0);
        assert_eq!(samples[1].symbol, "wasm[0]::function[3]");
        assert_eq!(samples[1].offset, 5);
        assert_eq!(samples[2].symbol, "wasm[0]::function[3]");
        assert_eq!(samples[2].offset, 0);
        assert_eq!(samples[3].symbol, "wasm[0]::function[5]");
        assert_eq!(samples[3].offset, 0x10);
    }

    #[test]
    fn test_read_clif_file() {
        let tmp = tempdir().unwrap();
        let clif_content = "\
@0010                           v1 = iconst.i32 0
@0011                           v2 = iconst.i32 1
                                v3 = iadd v1, v2
@0012                           return v3
";
        std::fs::write(tmp.path().join("wasm[0]--function[0].clif"), clif_content).unwrap();

        let lines = read_clif_file(
            tmp.path(),
            StaticModuleIndex::from_u32(0),
            FuncIndex::from_u32(0),
            None,
        );
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0].0, Some(0x0010));
        assert!(lines[0].1.contains("iconst.i32 0"));
        assert_eq!(lines[2].0, None);
        assert!(lines[2].1.contains("iadd"));
    }

    #[test]
    fn test_wat_offset_map() {
        // A minimal valid Wasm module with one function containing i32.add.
        let wat = r#"(module (func (param i32 i32) (result i32) local.get 0 local.get 1 i32.add))"#;
        let wasm = wat::parse_str(wat).unwrap();
        let map = build_wat_offset_map(&wasm);

        // The map should contain entries for the Wasm instructions.
        let has_i32_add = map.values().any(|v| v.contains("i32.add"));
        assert!(
            has_i32_add,
            "expected wat offset map to contain i32.add, got: {map:?}"
        );
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_ditto_marks() {
        // Test that repeated CLIF/Wasm annotations use ditto marks.
        // Build a function where multiple assembly instructions map to the
        // same wasm offset.
        let func_body: &[u8] = &[
            0x90, // nop  (offset 0) -> wasm @0010
            0x90, // nop  (offset 1) -> wasm @0010 (same)
            0x90, // nop  (offset 2) -> wasm @0011 (different)
            0xc3, // ret  (offset 3) -> wasm @0011 (same)
        ];

        let func_offset = 0usize;
        let address_map = vec![
            (func_offset, Some(0x0010u32)),
            (func_offset + 1, Some(0x0010)),
            (func_offset + 2, Some(0x0011)),
            (func_offset + 3, Some(0x0011)),
        ];

        let clif_lines = vec![
            (Some(0x0010u32), "v1 = iconst.i32 42".to_string()),
            (Some(0x0011u32), "return v1".to_string()),
        ];

        let mut wat_map = BTreeMap::new();
        wat_map.insert(0x0010, "i32.const 42".to_string());
        wat_map.insert(0x0011, "return".to_string());

        let target = target_lexicon::Triple::host();
        let _blocks = build_basic_blocks(
            func_body,
            func_offset,
            &address_map,
            &clif_lines,
            &wat_map,
            &target,
        )
        .unwrap();

        // Create samples for all instructions.
        let samples = vec![
            PerfSample {
                symbol: "wasm[0]::function[0]".to_string(),
                offset: 0,
            },
            PerfSample {
                symbol: "wasm[0]::function[0]".to_string(),
                offset: 1,
            },
            PerfSample {
                symbol: "wasm[0]::function[0]".to_string(),
                offset: 2,
            },
            PerfSample {
                symbol: "wasm[0]::function[0]".to_string(),
                offset: 3,
            },
        ];

        let functions = vec![ModuleFunction {
            module: StaticModuleIndex::from_u32(0),
            index: FuncIndex::from_u32(0),
            name: Some("wasm[0]::function[0]::test".to_string()),
            offset: 0,
            len: func_body.len(),
        }];

        let cmd = HotBlocksCommand {
            run: RunCommon {
                common: wasmtime_cli_flags::CommonOptions::default(),
                allow_precompiled: false,
                profile: None,
                dirs: Vec::new(),
                vars: Vec::new(),
            },
            percent: 100.0,
            event: Event::CpuCycles,
            frequency: None,
            output: None,
            module: PathBuf::from("dummy.wasm"),
            module_args: Vec::new(),
        };

        let mut output = Vec::new();
        cmd.format_hot_blocks(
            &samples,
            &functions,
            func_body,
            &address_map,
            Path::new("/nonexistent"),
            &wat_map,
            &target,
            &mut output,
        )
        .unwrap();

        let output_str = String::from_utf8(output).unwrap();
        // The second nop at offset 1 should show ditto marks for both CLIF and Wasm
        // since it has the same wasm offset (0x0010) as the first nop.
        assert!(
            output_str.contains('"'),
            "expected ditto marks in output, got:\n{output_str}"
        );
    }

    #[test]
    fn test_find_function_binary_search() {
        let functions = vec![
            ModuleFunction {
                module: StaticModuleIndex::from_u32(0),
                index: FuncIndex::from_u32(0),
                name: None,
                offset: 0x100,
                len: 0x50,
            },
            ModuleFunction {
                module: StaticModuleIndex::from_u32(0),
                index: FuncIndex::from_u32(1),
                name: None,
                offset: 0x150,
                len: 0x30,
            },
            ModuleFunction {
                module: StaticModuleIndex::from_u32(0),
                index: FuncIndex::from_u32(3),
                name: None,
                offset: 0x200,
                len: 0x40,
            },
        ];

        let sample = PerfSample {
            symbol: "wasm[0]::function[1]".to_string(),
            offset: 0x10,
        };
        let func = find_function_for_sample(&sample, &functions).unwrap();
        assert_eq!(func.index, FuncIndex::from_u32(1));

        let sample = PerfSample {
            symbol: "wasm[0]::function[3]".to_string(),
            offset: 0x5,
        };
        let func = find_function_for_sample(&sample, &functions).unwrap();
        assert_eq!(func.index, FuncIndex::from_u32(3));

        // Non-existent function.
        let sample = PerfSample {
            symbol: "wasm[0]::function[99]".to_string(),
            offset: 0,
        };
        assert!(find_function_for_sample(&sample, &functions).is_none());
    }
}
