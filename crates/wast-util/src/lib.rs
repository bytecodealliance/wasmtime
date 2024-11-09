use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde_derive::Deserialize;
use std::fmt;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

/// Limits for running wast tests.
///
/// This is useful for sharing between `tests/wast.rs` and fuzzing, for
/// example, and is used as the minimum threshold for configuration when
/// fuzzing.
///
/// Note that it's ok to increase these numbers if a test comes along and needs
/// it, they're just here as empirically found minimum thresholds so far and
/// they're not too scientific.
pub mod limits {
    pub const MEMORY_SIZE: usize = 805 << 16;
    pub const MEMORIES: u32 = 450;
    pub const TABLES: u32 = 200;
    pub const MEMORIES_PER_MODULE: u32 = 9;
    pub const TABLES_PER_MODULE: u32 = 5;
    pub const COMPONENT_INSTANCES: u32 = 50;
    pub const CORE_INSTANCES: u32 = 900;
    pub const TABLE_ELEMENTS: usize = 1000;
    pub const CORE_INSTANCE_SIZE: usize = 64 * 1024;
}

/// Local all `*.wast` tests under `root` which should be the path to the root
/// of the wasmtime repository.
pub fn find_tests(root: &Path) -> Result<Vec<WastTest>> {
    let mut tests = Vec::new();
    add_tests(&mut tests, &root.join("tests/spec_testsuite"), false)?;
    add_tests(&mut tests, &root.join("tests/misc_testsuite"), true)?;
    Ok(tests)
}

fn add_tests(tests: &mut Vec<WastTest>, path: &Path, has_config: bool) -> Result<()> {
    for entry in path.read_dir().context("failed to read directory")? {
        let entry = entry.context("failed to read directory entry")?;
        let path = entry.path();
        if entry
            .file_type()
            .context("failed to get file type")?
            .is_dir()
        {
            add_tests(tests, &path, has_config).context("failed to read sub-directory")?;
            continue;
        }

        if path.extension().and_then(|s| s.to_str()) != Some("wast") {
            continue;
        }

        let contents =
            fs::read_to_string(&path).with_context(|| format!("failed to read test: {path:?}"))?;
        let config = if has_config {
            parse_test_config(&contents)
                .with_context(|| format!("failed to parse test configuration: {path:?}"))?
        } else {
            spec_test_config(&path)
        };
        tests.push(WastTest {
            path,
            contents,
            config,
        })
    }
    Ok(())
}

fn spec_test_config(test: &Path) -> TestConfig {
    let mut ret = TestConfig::default();
    match spec_proposal_from_path(test) {
        Some("multi-memory") => {
            ret.multi_memory = Some(true);
            ret.reference_types = Some(true);
        }
        Some("wide-arithmetic") => {
            ret.wide_arithmetic = Some(true);
        }
        Some("threads") => {
            ret.threads = Some(true);
            ret.reference_types = Some(false);
        }
        Some("tail-call") => {
            ret.tail_call = Some(true);
            ret.reference_types = Some(true);
        }
        Some("relaxed-simd") => {
            ret.relaxed_simd = Some(true);
        }
        Some("memory64") => {
            ret.memory64 = Some(true);
            ret.tail_call = Some(true);
            ret.gc = Some(true);
            ret.extended_const = Some(true);
            ret.multi_memory = Some(true);
            ret.relaxed_simd = Some(true);
        }
        Some("extended-const") => {
            ret.extended_const = Some(true);
            ret.reference_types = Some(true);
        }
        Some("custom-page-sizes") => {
            ret.custom_page_sizes = Some(true);
            ret.multi_memory = Some(true);
        }
        Some("exception-handling") => {
            ret.reference_types = Some(true);
        }
        Some("gc") => {
            ret.gc = Some(true);
            ret.tail_call = Some(true);
        }
        Some("function-references") => {
            ret.function_references = Some(true);
            ret.tail_call = Some(true);
        }
        Some("annotations") => {}
        Some(proposal) => panic!("unsuported proposal {proposal:?}"),
        None => ret.reference_types = Some(true),
    }

    ret
}

/// Parse test configuration from the specified test, comments starting with
/// `;;!`.
pub fn parse_test_config<T>(wat: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    // The test config source is the leading lines of the WAT file that are
    // prefixed with `;;!`.
    let config_lines: Vec<_> = wat
        .lines()
        .take_while(|l| l.starts_with(";;!"))
        .map(|l| &l[3..])
        .collect();
    let config_text = config_lines.join("\n");

    toml::from_str(&config_text).context("failed to parse the test configuration")
}

/// A `*.wast` test with its path, contents, and configuration.
#[derive(Clone)]
pub struct WastTest {
    pub path: PathBuf,
    pub contents: String,
    pub config: TestConfig,
}

impl fmt::Debug for WastTest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WastTest")
            .field("path", &self.path)
            .field("contents", &"...")
            .field("config", &self.config)
            .finish()
    }
}

/// Per-test configuration which is written down in the test file itself for
/// `misc_testsuite/**/*.wast` or in `spec_test_config` above for spec tests.
#[derive(Debug, PartialEq, Default, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct TestConfig {
    pub memory64: Option<bool>,
    pub custom_page_sizes: Option<bool>,
    pub multi_memory: Option<bool>,
    pub threads: Option<bool>,
    pub gc: Option<bool>,
    pub function_references: Option<bool>,
    pub relaxed_simd: Option<bool>,
    pub reference_types: Option<bool>,
    pub tail_call: Option<bool>,
    pub extended_const: Option<bool>,
    pub wide_arithmetic: Option<bool>,
    pub hogs_memory: Option<bool>,
    pub nan_canonicalization: Option<bool>,
    pub component_model_more_flags: Option<bool>,
}

/// Configuration that spec tests can run under.
pub struct WastConfig {
    /// Compiler chosen to run this test.
    pub compiler: Compiler,
    /// Whether or not the pooling allocator is enabled.
    pub pooling: bool,
    /// What garbage collector is being used.
    pub collector: Collector,
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum Compiler {
    Cranelift,
    Winch,
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum Collector {
    Auto,
    Null,
    DeferredReferenceCounting,
}

impl WastTest {
    /// Returns whether this test exercises the GC types and might want to use
    /// multiple different garbage collectors.
    pub fn test_uses_gc_types(&self) -> bool {
        self.config
            .gc
            .or(self.config.function_references)
            .unwrap_or(false)
    }

    /// Returns the optional spec proposal that this test is associated with.
    pub fn spec_proposal(&self) -> Option<&str> {
        spec_proposal_from_path(&self.path)
    }

    /// Returns whether this test should fail under the specified extra
    /// configuration.
    pub fn should_fail(&self, config: &WastConfig) -> bool {
        // Winch only supports x86_64 at this time.
        if config.compiler == Compiler::Winch && !cfg!(target_arch = "x86_64") {
            return true;
        }

        // Disable spec tests for proposals that Winch does not implement yet.
        if config.compiler == Compiler::Winch {
            // A few proposals that winch has no support for.
            if self.config.gc == Some(true)
                || self.config.threads == Some(true)
                || self.config.tail_call == Some(true)
                || self.config.function_references == Some(true)
                || self.config.gc == Some(true)
                || self.config.relaxed_simd == Some(true)
            {
                return true;
            }

            let unsupported = [
                // externref/reference-types related
                "component-model/modules.wast",
                "extended-const/elem.wast",
                "extended-const/global.wast",
                "memory64/threads.wast",
                "misc_testsuite/externref-id-function.wast",
                "misc_testsuite/externref-segment.wast",
                "misc_testsuite/externref-segments.wast",
                "misc_testsuite/externref-table-dropped-segment-issue-8281.wast",
                "misc_testsuite/linking-errors.wast",
                "misc_testsuite/many_table_gets_lead_to_gc.wast",
                "misc_testsuite/mutable_externref_globals.wast",
                "misc_testsuite/no-mixup-stack-maps.wast",
                "misc_testsuite/no-panic.wast",
                "misc_testsuite/simple_ref_is_null.wast",
                "misc_testsuite/table_grow_with_funcref.wast",
                "spec_testsuite/br_table.wast",
                "spec_testsuite/data-invalid.wast",
                "spec_testsuite/elem.wast",
                "spec_testsuite/global.wast",
                "spec_testsuite/linking.wast",
                "spec_testsuite/ref_func.wast",
                "spec_testsuite/ref_is_null.wast",
                "spec_testsuite/ref_null.wast",
                "spec_testsuite/select.wast",
                "spec_testsuite/table-sub.wast",
                "spec_testsuite/table_fill.wast",
                "spec_testsuite/table_get.wast",
                "spec_testsuite/table_grow.wast",
                "spec_testsuite/table_set.wast",
                "spec_testsuite/table_size.wast",
                "spec_testsuite/unreached-invalid.wast",
                "spec_testsuite/call_indirect.wast",
                // simd-related failures
                "annotations/simd_lane.wast",
                "memory64/simd.wast",
                "misc_testsuite/int-to-float-splat.wast",
                "misc_testsuite/issue6562.wast",
                "misc_testsuite/simd/almost-extmul.wast",
                "misc_testsuite/simd/canonicalize-nan.wast",
                "misc_testsuite/simd/cvt-from-uint.wast",
                "misc_testsuite/simd/issue4807.wast",
                "misc_testsuite/simd/issue6725-no-egraph-panic.wast",
                "misc_testsuite/simd/issue_3327_bnot_lowering.wast",
                "misc_testsuite/simd/load_splat_out_of_bounds.wast",
                "misc_testsuite/simd/replace-lane-preserve.wast",
                "misc_testsuite/simd/spillslot-size-fuzzbug.wast",
                "misc_testsuite/simd/unaligned-load.wast",
                "multi-memory/simd_memory-multi.wast",
                "spec_testsuite/simd_align.wast",
                "spec_testsuite/simd_bit_shift.wast",
                "spec_testsuite/simd_bitwise.wast",
                "spec_testsuite/simd_boolean.wast",
                "spec_testsuite/simd_const.wast",
                "spec_testsuite/simd_conversions.wast",
                "spec_testsuite/simd_f32x4.wast",
                "spec_testsuite/simd_f32x4_arith.wast",
                "spec_testsuite/simd_f32x4_cmp.wast",
                "spec_testsuite/simd_f32x4_pmin_pmax.wast",
                "spec_testsuite/simd_f32x4_rounding.wast",
                "spec_testsuite/simd_f64x2.wast",
                "spec_testsuite/simd_f64x2_arith.wast",
                "spec_testsuite/simd_f64x2_cmp.wast",
                "spec_testsuite/simd_f64x2_pmin_pmax.wast",
                "spec_testsuite/simd_f64x2_rounding.wast",
                "spec_testsuite/simd_i16x8_arith.wast",
                "spec_testsuite/simd_i16x8_arith2.wast",
                "spec_testsuite/simd_i16x8_cmp.wast",
                "spec_testsuite/simd_i16x8_extadd_pairwise_i8x16.wast",
                "spec_testsuite/simd_i16x8_extmul_i8x16.wast",
                "spec_testsuite/simd_i16x8_q15mulr_sat_s.wast",
                "spec_testsuite/simd_i16x8_sat_arith.wast",
                "spec_testsuite/simd_i32x4_arith.wast",
                "spec_testsuite/simd_i32x4_arith2.wast",
                "spec_testsuite/simd_i32x4_cmp.wast",
                "spec_testsuite/simd_i32x4_dot_i16x8.wast",
                "spec_testsuite/simd_i32x4_extadd_pairwise_i16x8.wast",
                "spec_testsuite/simd_i32x4_extmul_i16x8.wast",
                "spec_testsuite/simd_i32x4_trunc_sat_f32x4.wast",
                "spec_testsuite/simd_i32x4_trunc_sat_f64x2.wast",
                "spec_testsuite/simd_i64x2_arith.wast",
                "spec_testsuite/simd_i64x2_arith2.wast",
                "spec_testsuite/simd_i64x2_cmp.wast",
                "spec_testsuite/simd_i64x2_extmul_i32x4.wast",
                "spec_testsuite/simd_i8x16_arith.wast",
                "spec_testsuite/simd_i8x16_arith2.wast",
                "spec_testsuite/simd_i8x16_cmp.wast",
                "spec_testsuite/simd_i8x16_sat_arith.wast",
                "spec_testsuite/simd_int_to_int_extend.wast",
                "spec_testsuite/simd_lane.wast",
                "spec_testsuite/simd_load.wast",
                "spec_testsuite/simd_load16_lane.wast",
                "spec_testsuite/simd_load32_lane.wast",
                "spec_testsuite/simd_load64_lane.wast",
                "spec_testsuite/simd_load8_lane.wast",
                "spec_testsuite/simd_load_extend.wast",
                "spec_testsuite/simd_load_splat.wast",
                "spec_testsuite/simd_load_zero.wast",
                "spec_testsuite/simd_splat.wast",
                "spec_testsuite/simd_store16_lane.wast",
                "spec_testsuite/simd_store32_lane.wast",
                "spec_testsuite/simd_store64_lane.wast",
                "spec_testsuite/simd_store8_lane.wast",
            ];

            if unsupported.iter().any(|part| self.path.ends_with(part)) {
                return true;
            }
        }

        for part in self.path.iter() {
            // Not implemented in Wasmtime yet
            if part == "exception-handling" {
                return !self.path.ends_with("binary.wast");
            }

            if part == "memory64" {
                if [
                    // wasmtime doesn't implement exceptions yet
                    "imports.wast",
                    "ref_null.wast",
                    "exports.wast",
                    "throw.wast",
                    "throw_ref.wast",
                    "try_table.wast",
                    "tag.wast",
                    "instance.wast",
                ]
                .iter()
                .any(|i| self.path.ends_with(i))
                {
                    return true;
                }
            }
        }

        // Some tests are known to fail with the pooling allocator
        if config.pooling {
            let unsupported = [
                // allocates too much memory for the pooling configuration here
                "misc_testsuite/memory64/more-than-4gb.wast",
                // shared memories + pooling allocator aren't supported yet
                "misc_testsuite/memory-combos.wast",
                "misc_testsuite/threads/LB.wast",
                "misc_testsuite/threads/LB_atomic.wast",
                "misc_testsuite/threads/MP.wast",
                "misc_testsuite/threads/MP_atomic.wast",
                "misc_testsuite/threads/MP_wait.wast",
                "misc_testsuite/threads/SB.wast",
                "misc_testsuite/threads/SB_atomic.wast",
                "misc_testsuite/threads/atomics_notify.wast",
                "misc_testsuite/threads/atomics_wait_address.wast",
                "misc_testsuite/threads/wait_notify.wast",
                "spec_testsuite/proposals/threads/atomic.wast",
                "spec_testsuite/proposals/threads/exports.wast",
                "spec_testsuite/proposals/threads/memory.wast",
            ];

            if unsupported.iter().any(|part| self.path.ends_with(part)) {
                return true;
            }
        }

        false
    }
}

fn spec_proposal_from_path(path: &Path) -> Option<&str> {
    let mut iter = path.iter();
    loop {
        match iter.next()?.to_str()? {
            "proposals" => break,
            _ => {}
        }
    }
    Some(iter.next()?.to_str()?)
}
