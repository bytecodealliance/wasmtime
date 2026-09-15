use anyhow::Result;
use json_from_wast::FloatConst;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::{
    cases::{Case, CaseValue},
    compile,
};

pub struct BatchRunResult {
    pub output: String,
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
}

#[derive(Clone)]
struct PreparedCase {
    index: usize,
    symbol_name: String,
    runner_name: String,
    bytes: Vec<u8>,
    alignment: u32,
    args: Vec<CaseValue>,
    expected: CaseValue,
    store_ctx_off: u32,
}

pub fn run_cases_batch(cases: &[Case], workdir: &Path, verbose: bool) -> Result<BatchRunResult> {
    let (compiler, tunables) = compile::build_arm32_compiler()?;

    let mut compiled_cases = Vec::new();
    let mut compiled_cache: HashMap<String, PreparedCase> = HashMap::new();
    let mut output_lines = Vec::new();
    let mut passed = 0u32;
    let mut failed = 0u32;
    let skipped = 0u32;

    for (idx, case) in cases.iter().enumerate() {
        let runner_name = make_case_runner_name(idx, &case.export);
        let cache_key = make_compiled_case_key(case);

        let prepared = match compiled_cache.get(&cache_key) {
            Some(compiled) => PreparedCase {
                index: idx,
                symbol_name: compiled.symbol_name.clone(),
                runner_name: runner_name.clone(),
                bytes: compiled.bytes.clone(),
                alignment: compiled.alignment,
                args: case.args.clone(),
                expected: case.expected,
                store_ctx_off: compiled.store_ctx_off,
            },
            None => match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                prepare_case(
                    &*compiler,
                    &tunables,
                    idx,
                    case,
                    &make_compiled_symbol_name(case),
                    &runner_name,
                )
            })) {
                Ok(Ok(result)) => {
                    compiled_cache.insert(cache_key.clone(), result.clone());
                    result
                }
                Ok(Err(e)) => {
                    output_lines.push(format!("FAIL case {idx}: {e}"));
                    failed += 1;
                    continue;
                }
                Err(payload) => {
                    output_lines.push(format!("FAIL case {idx}: {payload:?}"));
                    failed += 1;
                    continue;
                }
            },
        };

        compiled_cases.push(prepared);
    }

    if compiled_cases.is_empty() {
        return Ok(BatchRunResult {
            output: output_lines.join("\n"),
            passed,
            failed,
            skipped,
        });
    }

    let obj_bytes = emit_object_file_with_trampoline(&compiled_cases)?;
    let obj_path = workdir.join("module.o");
    std::fs::write(&obj_path, &obj_bytes)?;

    let c_source = generate_c_driver(&compiled_cases, verbose);
    let driver_path = workdir.join("driver.c");
    std::fs::write(&driver_path, &c_source)?;

    let elf_path = workdir.join("program");
    let toolchain = locate_toolchain()?;
    link_with_gcc(
        &toolchain.compiler,
        &toolchain.sysroot,
        &obj_path,
        &driver_path,
        &elf_path,
    )?;

    let (harness_output, exit_code) =
        run_under_qemu(&toolchain.qemu, &toolchain.sysroot, &elf_path)?;
    if let Some((summary_passed, summary_failed)) = parse_result_summary(&harness_output) {
        passed += summary_passed;
        failed += summary_failed;
    } else {
        let harness_failed = if exit_code == 0 {
            0u32
        } else {
            exit_code as u32
        };
        let harness_passed = compiled_cases.len().saturating_sub(harness_failed as usize) as u32;
        passed += harness_passed;
        failed += harness_failed;
    }

    output_lines.push(harness_output.trim().to_string());
    let output = output_lines
        .into_iter()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    Ok(BatchRunResult {
        output,
        passed,
        failed,
        skipped,
    })
}

fn prepare_case(
    compiler: &dyn wasmtime_environ::Compiler,
    tunables: &wasmtime_environ::Tunables,
    index: usize,
    case: &Case,
    symbol_name: &str,
    runner_name: &str,
) -> Result<PreparedCase> {
    let (bytes, alignment, translation) =
        compile_wasm_function(compiler, tunables, &case.module, &case.export, symbol_name)?;
    let offsets = get_vmctx_offsets(&translation);
    Ok(PreparedCase {
        index,
        symbol_name: symbol_name.to_string(),
        runner_name: runner_name.to_string(),
        bytes,
        alignment,
        args: case.args.clone(),
        expected: case.expected,
        store_ctx_off: offsets.store_ctx_off,
    })
}

fn make_case_symbol_name(index: usize, export_name: &str) -> String {
    let sanitized = export_name
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>();
    let sanitized = sanitized.trim_matches('_');
    if sanitized.is_empty() {
        format!("case_{index}")
    } else {
        format!("case_{index}_{sanitized}")
    }
}

fn make_case_runner_name(index: usize, export_name: &str) -> String {
    format!(
        "run_case_{index}_{}",
        make_case_symbol_name(index, export_name)
    )
}

fn make_compiled_case_key(case: &Case) -> String {
    let mut key = String::new();
    key.push_str(&case.export);
    key.push('|');
    for arg in &case.args {
        key.push_str(arg.type_key());
        key.push(',');
    }
    key.push_str(case.expected.type_key());
    key.push('|');
    key.push_str(&format!("{:016x}", hash_bytes(&case.module)));
    key
}

fn make_compiled_symbol_name(case: &Case) -> String {
    let export_name = make_case_symbol_name(0, &case.export);
    let key = make_compiled_case_key(case);
    let hash = hash_bytes(key.as_bytes());
    format!("wast_arm_runtest_module_{export_name}_{hash:016x}")
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3_u64);
    }
    hash
}

fn compile_wasm_function<'data>(
    compiler: &dyn wasmtime_environ::Compiler,
    tunables: &wasmtime_environ::Tunables,
    wasm_bytes: &'data [u8],
    export_name: &str,
    symbol_name: &str,
) -> Result<(Vec<u8>, u32, wasmtime_environ::ModuleTranslation<'data>)> {
    use wasmparser::{Parser, Validator};
    use wasmtime_environ::{FuncKey, ModuleEnvironment, ModuleTypesBuilder, StaticModuleIndex};

    let mut validator = Validator::new();
    let mut types = ModuleTypesBuilder::new(&validator);
    let env = ModuleEnvironment::new(
        tunables,
        &mut validator,
        &mut types,
        StaticModuleIndex::from_u32(0),
    );

    let mut translation = env
        .translate(Parser::new(0), wasm_bytes)
        .map_err(|e| anyhow::anyhow!("failed to translate module: {}", e))?;

    // Find the function index for the export
    let func_index = find_export_func_index(&translation.module, export_name)
        .ok_or_else(|| anyhow::anyhow!("export '{}' not found", export_name))?;

    // Move function bodies out - we need to clone the map but FunctionBodyData doesn't Clone
    // So we use a workaround: iterate and collect the specific body we need
    let bodies = std::mem::take(&mut translation.function_body_inputs);

    // Find the body for our function index by iterating
    let body_data = bodies
        .into_iter()
        .find(|(idx, _)| *idx == func_index)
        .map(|(_, data)| data)
        .ok_or_else(|| anyhow::anyhow!("function body not found for index {:?}", func_index))?;

    let key = FuncKey::DefinedWasmFunction(StaticModuleIndex::from_u32(0), func_index);

    // Compile the function
    let mut cfb = compiler.compile_function(&translation, key, body_data, &types, symbol_name)?;

    // Finish compiling
    compiler
        .inlining_compiler()
        .ok_or_else(|| anyhow::anyhow!("compiler does not support inlining"))?
        .finish_compiling(&mut cfb, None, symbol_name)
        .map_err(|e| anyhow::anyhow!("failed to finish compiling: {}", e))?;

    // Extract machine code bytes
    let cf = cfb
        .code
        .downcast_ref::<wasmtime_cranelift::CompiledFunction>()
        .ok_or_else(|| anyhow::anyhow!("expected CompiledFunction"))?;

    let bytes = cf.buffer.data().to_vec();
    let alignment = cf.alignment;

    Ok((bytes, alignment, translation))
}

fn find_export_func_index(
    module: &wasmtime_environ::Module,
    name: &str,
) -> Option<wasmtime_environ::DefinedFuncIndex> {
    use wasmtime_environ::EntityIndex;

    for (atom, entity_idx) in &module.exports {
        let atom_str = module.strings.get(*atom);
        if atom_str == Some(name) {
            match entity_idx {
                EntityIndex::Function(idx) => {
                    // FuncIndex is a u32 wrapper, we need to convert to DefinedFuncIndex
                    // For now just use the raw value - this works because both are u32-based
                    return Some(wasmtime_environ::DefinedFuncIndex::from_u32(idx.as_u32()));
                }
                _ => continue,
            }
        }
    }
    None
}

fn exported_symbol_name(symbol_name: &str) -> String {
    if symbol_name.starts_with("wast_arm_runtest_module_") {
        symbol_name.to_string()
    } else {
        format!("wast_arm_runtest_module_{symbol_name}")
    }
}

fn emit_object_file_with_trampoline(cases: &[PreparedCase]) -> Result<Vec<u8>> {
    use cranelift_codegen::ir::{AbiParam, Signature, types};
    use cranelift_codegen::isa::{CallConv, lookup};
    use cranelift_codegen::settings;
    use cranelift_module::{Linkage, Module, default_libcall_names};
    use cranelift_object::{ObjectBuilder, ObjectModule};

    let triple_str = "armv7-unknown-linux-gnueabihf";
    let triple = target_lexicon::Triple::from_str(triple_str)
        .map_err(|e| anyhow::anyhow!("failed to parse target triple: {}", e))?;
    let isa = lookup(triple.clone())?.finish(settings::Flags::new(settings::builder()))?;
    let builder = ObjectBuilder::new(isa, "module", default_libcall_names())?;
    let mut module = ObjectModule::new(builder);

    let ptr = types::I32;
    let mut emitted_symbols = HashSet::new();

    for case in cases {
        // Export the compiled wasm function directly so C can call it with the
        // expected AAPCS-like register layout for (vmctx, caller_vmctx, args...).
        let mut wasm_sig = Signature::new(CallConv::triple_default(&triple));
        wasm_sig.params.push(AbiParam::new(ptr)); // vmctx
        wasm_sig.params.push(AbiParam::new(ptr)); // caller_vmctx
        for arg in &case.args {
            wasm_sig.params.push(AbiParam::new(arg.cranelift_type()));
        }
        wasm_sig
            .returns
            .push(AbiParam::new(case.expected.cranelift_type()));

        let symbol_name = exported_symbol_name(&case.symbol_name);
        if emitted_symbols.contains(&symbol_name) {
            continue;
        }
        emitted_symbols.insert(symbol_name.clone());

        let wasm_id = module.declare_function(&symbol_name, Linkage::Export, &wasm_sig)?;
        module.define_function_bytes(wasm_id, case.alignment as u64, &case.bytes, &[])?;
    }

    Ok(module.finish().emit()?)
}

fn get_vmctx_offsets(translation: &wasmtime_environ::ModuleTranslation<'_>) -> VmctxOffsets {
    use wasmtime_environ::{PtrSize, VMOffsets};

    let offsets = VMOffsets::new(4u8, &translation.module);
    VmctxOffsets {
        store_ctx_off: offsets.ptr.vmcontext_store_context() as u32,
        stack_limit_off: offsets.ptr.vmstore_context_stack_limit() as u32,
    }
}

struct VmctxOffsets {
    store_ctx_off: u32,
    #[allow(dead_code)]
    stack_limit_off: u32,
}

impl CaseValue {
    fn type_key(self) -> &'static str {
        self.dispatch(|_| "i32", |_| "i64", |_| "f32", |_| "f64")
    }

    fn dispatch<T>(
        self,
        on_i32: impl FnOnce(i32) -> T,
        on_i64: impl FnOnce(i64) -> T,
        on_f32: impl FnOnce(FloatConst<f32>) -> T,
        on_f64: impl FnOnce(FloatConst<f64>) -> T,
    ) -> T {
        match self {
            Self::I32(value) => on_i32(value),
            Self::I64(value) => on_i64(value),
            Self::F32(value) => on_f32(value),
            Self::F64(value) => on_f64(value),
        }
    }

    fn cranelift_type(self) -> cranelift_codegen::ir::Type {
        self.dispatch(
            |_| cranelift_codegen::ir::types::I32,
            |_| cranelift_codegen::ir::types::I64,
            |_| cranelift_codegen::ir::types::F32,
            |_| cranelift_codegen::ir::types::F64,
        )
    }

    fn c_type_name(self) -> &'static str {
        self.dispatch(|_| "int", |_| "long long", |_| "float", |_| "double")
    }

    fn c_assignment(self, index: usize) -> String {
        self.dispatch(
            |value| format!("    int a{index} = {value};"),
            |value| format!("    long long a{index} = {value}LL;"),
            |bits| {
                format!(
                    "    float a{index} = bits_to_f32(0x{:08x}u);",
                    bits.to_bits()
                )
            },
            |bits| {
                format!(
                    "    double a{index} = bits_to_f64(0x{:016x}ULL);",
                    bits.to_bits()
                )
            },
        )
    }

    fn c_compare_stmt(self, got_name: &str) -> String {
        self.dispatch(
            |value| format!("    int passed = ({got_name} == {value});"),
            |value| format!("    int passed = ({got_name} == {value}LL);"),
            |bits| {
                format!(
                    "    int passed = (f32_to_bits({got_name}) == 0x{:08x}u);",
                    bits.to_bits()
                )
            },
            |bits| {
                format!(
                    "    int passed = (f64_to_bits({got_name}) == 0x{:016x}ULL);",
                    bits.to_bits()
                )
            },
        )
    }

    fn c_expected_literal(self) -> String {
        let literal = self.to_wast_literal();
        format!("\"{}\"", literal.replace('\\', "\\\\").replace('"', "\\\""))
    }

    fn c_got_literal_stmt(self) -> String {
        self.dispatch(
            |_| "    char got_literal_buf[64];\n    snprintf(got_literal_buf, sizeof(got_literal_buf), \"%d\", got);\n    const char *got_literal = got_literal_buf;".to_string(),
            |_| "    char got_literal_buf[64];\n    snprintf(got_literal_buf, sizeof(got_literal_buf), \"%lld\", got);\n    const char *got_literal = got_literal_buf;".to_string(),
            |_| "    char got_literal_buf[64];\n    snprintf(got_literal_buf, sizeof(got_literal_buf), \"%f\", got);\n    const char *got_literal = got_literal_buf;".to_string(),
            |_| "    char got_literal_buf[64];\n    snprintf(got_literal_buf, sizeof(got_literal_buf), \"%f\", got);\n    const char *got_literal = got_literal_buf;".to_string(),
        )
    }
}

fn format_c_arg_params(args: &[CaseValue]) -> String {
    if args.is_empty() {
        String::new()
    } else {
        let params = args
            .iter()
            .enumerate()
            .map(|(i, arg)| format!(" {} a{}", arg.c_type_name(), i))
            .collect::<Vec<_>>()
            .join(",");
        format!(", {}", params)
    }
}

fn format_c_arg_calls(args: &[CaseValue]) -> String {
    if args.is_empty() {
        String::new()
    } else {
        let args_call = args
            .iter()
            .enumerate()
            .map(|(i, _)| format!("a{i}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(", {}", args_call)
    }
}

fn c_driver_helpers(verbose: bool) -> String {
    let verbose_define = if verbose { "1" } else { "0" };
    let mut helper = String::from(
        "#include <stdint.h>\n#include <stdio.h>\n#include <string.h>\n\n#define WAST_ARM_RUNTTEST_VERBOSE ",
    );
    helper.push_str(verbose_define);
    helper.push_str(
        "\n\nstatic float bits_to_f32(uint32_t bits) {\n    float out;\n    memcpy(&out, &bits, sizeof(out));\n    return out;\n}\n\nstatic double bits_to_f64(uint64_t bits) {\n    double out;\n    memcpy(&out, &bits, sizeof(out));\n    return out;\n}\n\nstatic uint32_t f32_to_bits(float value) {\n    uint32_t bits;\n    memcpy(&bits, &value, sizeof(bits));\n    return bits;\n}\n\nstatic uint64_t f64_to_bits(double value) {\n    uint64_t bits;\n    memcpy(&bits, &value, sizeof(bits));\n    return bits;\n}\n\nstatic const char *f32_to_literal(uint32_t bits) {\n    static char buf[64];\n    float value = bits_to_f32(bits);\n    if (bits == 0x00000000u) return \"0x0p+0\";\n    if (bits == 0x80000000u) return \"-0x0p+0\";\n    if ((bits & 0x7f800000u) == 0x7f800000u) {\n        if ((bits & 0x00400000u) == 0) return \"nan:canonical\";\n        return \"nan:arithmetic\";\n    }\n    snprintf(buf, sizeof(buf), \"%a\", (double)value);\n    return buf;\n}\n\nstatic const char *f64_to_literal(uint64_t bits) {\n    static char buf[64];\n    double value = bits_to_f64(bits);\n    if (bits == 0x0000000000000000ULL) return \"0x0p+0\";\n    if (bits == 0x8000000000000000ULL) return \"-0x0p+0\";\n    if ((bits & 0x7ff0000000000000ULL) == 0x7ff0000000000000ULL) {\n        if ((bits & 0x0008000000000000ULL) == 0) return \"nan:canonical\";\n        return \"nan:arithmetic\";\n    }\n    snprintf(buf, sizeof(buf), \"%a\", value);\n    return buf;\n}\n",
    );
    helper
}

fn c_driver_prototype(ret_ty: &str, export_name: &str, args_params: &str) -> String {
    format!("extern {ret_ty} {export_name}(void *vmctx, void *caller_vmctx{args_params});")
}

struct CDriverMainContext<'a> {
    store_ctx_off: u32,
    sc_base: u32,
    export_name: &'a str,
    runner_name: &'a str,
    case_label: &'a str,
    arg_assignments: &'a str,
    got_decl: &'a str,
    args_call: &'a str,
    compare_stmt: &'a str,
    got_literal_stmt: &'a str,
    expected_literal: &'a str,
}

fn c_driver_main(context: CDriverMainContext<'_>) -> String {
    let CDriverMainContext {
        store_ctx_off,
        sc_base,
        export_name,
        runner_name,
        case_label,
        arg_assignments,
        got_decl,
        args_call,
        compare_stmt,
        got_literal_stmt,
        expected_literal,
    } = context;

    format!(
        "int {runner_name}(void) {{\n\
            unsigned char buf[256] = {{0}};\n\
            // VMContext.store_context (at STORE_CTX_OFF) -> points at buf+SC_BASE\n\
            *(uintptr_t*)(buf + {store_ctx_off}) = (uintptr_t)(buf + {sc_base});\n\
            // VMStoreContext.stack_limit at SC_BASE + STACK_LIMIT_OFF is 0 (buf is zero-initialized)\n\
            {arg_assignments}\n\
            {got_decl} = {export_name}((void*)buf, (void*)buf{args_call});\n\
            {got_literal_stmt}\n\
            {compare_stmt}\n\
            if (passed) {{\n\
                return 0;\n\
            }} else {{\n\
                printf(\"FAIL case {case_label}: expected %s, got %s\\n\", {expected_literal}, got_literal);\n\
                return 1;\n\
            }}\n\
        }}"
    )
}

fn generate_c_driver(cases: &[PreparedCase], verbose: bool) -> String {
    let sc_base = 64u32;
    let mut driver = String::new();
    driver.push_str(&c_driver_helpers(verbose));
    driver.push_str("\n\n");

    for case in cases {
        let args_params_str = format_c_arg_params(&case.args);
        let args_call_str = format_c_arg_calls(&case.args);

        let arg_assignments = case
            .args
            .iter()
            .enumerate()
            .map(|(i, arg)| arg.c_assignment(i))
            .collect::<Vec<_>>()
            .join("\n");

        let ret_ty = case.expected.c_type_name();
        let got_decl = format!("{} got", ret_ty);
        let compare_stmt = case.expected.c_compare_stmt("got");
        let got_literal_stmt = case.expected.c_got_literal_stmt();
        let expected_literal = case.expected.c_expected_literal();
        let case_label = format!("{}", case.index);

        let exported_name = exported_symbol_name(&case.symbol_name);

        driver.push_str(&c_driver_prototype(
            ret_ty,
            &exported_name,
            &args_params_str,
        ));
        driver.push_str("\n\n");
        driver.push_str(&c_driver_main(CDriverMainContext {
            store_ctx_off: case.store_ctx_off,
            sc_base,
            export_name: &exported_name,
            runner_name: &case.runner_name,
            case_label: &case_label,
            arg_assignments: &arg_assignments,
            got_decl: &got_decl,
            args_call: &args_call_str,
            compare_stmt: &compare_stmt,
            got_literal_stmt: &got_literal_stmt,
            expected_literal: &expected_literal,
        }));
        driver.push_str("\n\n");
    }

    driver.push_str("int main(void) {\n");
    driver.push_str("    int passed = 0;\n");
    driver.push_str("    int failed = 0;\n");
    for (idx, case) in cases.iter().enumerate() {
        driver.push_str(&format!("    int result_{idx} = {}();\n", case.runner_name));
        driver.push_str(&format!("    if (result_{idx} == 0) {{\n"));
        driver.push_str("        passed += 1;\n");
        driver.push_str(
            "    } else {
",
        );
        driver.push_str("        failed += 1;\n");
        driver.push_str(
            "    }
",
        );
    }
    if verbose {
        driver.push_str("    printf(\"RESULT passed=%d failed=%d\\n\", passed, failed);\n");
    }
    driver.push_str("    return failed;\n}\n");
    driver
}

fn parse_result_summary(output: &str) -> Option<(u32, u32)> {
    output.lines().find_map(|line| {
        let line = line.trim();
        let mut words = line.split_whitespace();
        if words.next()? != "RESULT" {
            return None;
        }

        let passed = words.next()?.strip_prefix("passed=")?.parse::<u32>().ok()?;
        let failed = words.next()?.strip_prefix("failed=")?.parse::<u32>().ok()?;
        Some((passed, failed))
    })
}

struct Toolchain {
    compiler: std::path::PathBuf,
    sysroot: Option<std::path::PathBuf>,
    qemu: std::path::PathBuf,
}

fn locate_toolchain() -> Result<Toolchain> {
    let compiler_candidates = [
        "arm-linux-gnueabihf-gcc",
        "arm-linux-gnueabihf-gcc-13",
        "arm-linux-gnueabihf-gcc-12",
        "arm-linux-gnueabihf-gcc-11",
        "arm-none-eabi-gcc",
    ];
    let qemu_candidates = ["qemu-arm-static", "qemu-arm"];

    let compiler = compiler_candidates
        .iter()
        .find_map(|name| find_executable(name))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "could not find an ARM cross-compiler; tried {:?}",
                compiler_candidates
            )
        })?;
    let qemu = qemu_candidates
        .iter()
        .find_map(|name| find_executable(name))
        .ok_or_else(|| {
            anyhow::anyhow!("could not find QEMU for ARM; tried {:?}", qemu_candidates)
        })?;

    let sysroot = detect_sysroot(&compiler).or_else(detect_sysroot_from_env);
    Ok(Toolchain {
        compiler,
        sysroot,
        qemu,
    })
}

fn find_executable(name: &str) -> Option<std::path::PathBuf> {
    std::env::var_os("PATH")?;
    std::env::split_paths(&std::env::var_os("PATH")?).find_map(|dir| {
        let path = dir.join(name);
        path.is_file().then_some(path)
    })
}

fn detect_sysroot(compiler: &std::path::Path) -> Option<std::path::PathBuf> {
    let output = std::process::Command::new(compiler)
        .arg("-print-sysroot")
        .output()
        .ok()?;
    let sysroot = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let sysroot = if sysroot.is_empty() || sysroot == "/" {
        None
    } else {
        Some(std::path::PathBuf::from(sysroot))
    };

    sysroot.or_else(|| {
        [
            std::path::PathBuf::from("/usr/arm-linux-gnueabihf"),
            std::path::PathBuf::from("/usr/arm-linux-gnueabihf/lib"),
        ]
        .into_iter()
        .find(|path| path.join("lib/ld-linux-armhf.so.3").is_file())
    })
}

fn detect_sysroot_from_env() -> Option<std::path::PathBuf> {
    std::env::var_os("ARM_LINUX_GNUEABIHF_SYSROOT")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("CROSS_SYSROOT").map(std::path::PathBuf::from))
}

fn link_with_gcc(
    gcc_path: &Path,
    sysroot: &Option<PathBuf>,
    obj_path: &Path,
    driver_path: &Path,
    elf_path: &Path,
) -> Result<()> {
    let mut command = std::process::Command::new(gcc_path);
    command
        .arg("-marm")
        .arg(driver_path)
        .arg(obj_path)
        .arg("-o")
        .arg(elf_path);
    if let Some(sysroot) = sysroot {
        command.arg("-L").arg(sysroot.join("lib"));
    }
    let status = command.status()?;

    if !status.success() {
        anyhow::bail!("linking failed with status: {}", status);
    }
    Ok(())
}

fn run_under_qemu(
    qemu: &Path,
    sysroot: &Option<PathBuf>,
    elf_path: &Path,
) -> Result<(String, i32)> {
    let mut command = std::process::Command::new(qemu);
    if let Some(sysroot) = sysroot {
        command.arg("-L").arg(sysroot);
    }
    command.arg(elf_path);
    let output = command.output()?;

    let result = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let exit_code = output.status.code().unwrap_or(-1);
    if exit_code < 0 {
        anyhow::bail!(
            "QEMU execution failed with status: {}, stderr: {}",
            output.status,
            stderr.trim()
        );
    }

    Ok((result.trim().to_string(), exit_code))
}

#[cfg(test)]
mod tests {
    use super::*;
    use json_from_wast::FloatConst;

    #[test]
    fn driver_uses_wast_style_float_literals() {
        let prepared = PreparedCase {
            index: 0,
            symbol_name: "f".to_string(),
            runner_name: "run_f".to_string(),
            bytes: vec![],
            alignment: 0,
            args: vec![],
            expected: CaseValue::F32(FloatConst::Value(1.0)),
            store_ctx_off: 0,
        };
        let driver = generate_c_driver(&[prepared], false);
        assert!(driver.contains("#define WAST_ARM_RUNTTEST_VERBOSE 0"));
        assert!(!driver.contains("RESULT"));
        assert!(!driver.contains("PASS case"));
    }

    #[test]
    fn non_verbose_mode_suppresses_result_summary() {
        let prepared = PreparedCase {
            index: 0,
            symbol_name: "f".to_string(),
            runner_name: "run_f".to_string(),
            bytes: vec![],
            alignment: 0,
            args: vec![],
            expected: CaseValue::F32(FloatConst::Value(1.0)),
            store_ctx_off: 0,
        };
        let driver = generate_c_driver(&[prepared], false);
        assert!(driver.contains("#define WAST_ARM_RUNTTEST_VERBOSE 0"));
        assert!(!driver.contains("RESULT"));
    }

    #[test]
    fn verbose_mode_emits_result_summary() {
        let prepared = PreparedCase {
            index: 0,
            symbol_name: "f".to_string(),
            runner_name: "run_f".to_string(),
            bytes: vec![],
            alignment: 0,
            args: vec![],
            expected: CaseValue::F32(FloatConst::Value(1.0)),
            store_ctx_off: 0,
        };
        let driver = generate_c_driver(&[prepared], true);
        assert!(driver.contains("#define WAST_ARM_RUNTTEST_VERBOSE 1"));
        assert!(driver.contains("RESULT passed=%d failed=%d"));
    }

    #[test]
    fn generated_driver_uses_unique_result_names_per_case() {
        let prepared_a = PreparedCase {
            index: 0,
            symbol_name: "f".to_string(),
            runner_name: "run_f".to_string(),
            bytes: vec![],
            alignment: 0,
            args: vec![],
            expected: CaseValue::F32(FloatConst::Value(1.0)),
            store_ctx_off: 0,
        };
        let prepared_b = PreparedCase {
            index: 1,
            symbol_name: "g".to_string(),
            runner_name: "run_g".to_string(),
            bytes: vec![],
            alignment: 0,
            args: vec![],
            expected: CaseValue::F32(FloatConst::Value(1.0)),
            store_ctx_off: 0,
        };
        let driver = generate_c_driver(&[prepared_a, prepared_b], false);
        assert!(driver.contains("int result_0 = run_f();"));
        assert!(driver.contains("int result_1 = run_g();"));
    }

    #[test]
    fn case_symbol_names_are_unique_and_c_safe() {
        assert_eq!(make_case_symbol_name(0, "foo"), "case_0_foo");
        assert_eq!(make_case_symbol_name(1, "foo-bar"), "case_1_foo_bar");
        assert_eq!(make_case_symbol_name(2, "foo.bar"), "case_2_foo_bar");
    }

    #[test]
    fn exported_symbols_are_prefixed_for_the_c_driver() {
        assert_eq!(
            exported_symbol_name("case_0_foo"),
            "wast_arm_runtest_module_case_0_foo"
        );
        assert_eq!(
            exported_symbol_name("wast_arm_runtest_module_case_0_foo"),
            "wast_arm_runtest_module_case_0_foo"
        );
    }

    #[test]
    fn unsupported_backend_errors_are_failed() {
        let outcome = classify_case_error(
            "Unsupported feature: should be implemented in ISLE: inst = `v4 = rotl.i32`",
        );
        assert!(matches!(outcome, CaseOutcome::Fail(_)));
    }

    #[test]
    fn panic_errors_are_failed() {
        let outcome = classify_case_error("panic while compiling wasm function");
        assert!(matches!(outcome, CaseOutcome::Fail(_)));
    }
}
