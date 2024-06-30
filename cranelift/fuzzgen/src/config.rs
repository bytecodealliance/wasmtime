use std::collections::HashMap;
use std::ops::RangeInclusive;

/// Holds the range of acceptable values to use during the generation of testcases
pub struct Config {
    /// Maximum allowed test case inputs.
    /// We build test case inputs from the rest of the bytes that the fuzzer provides us
    /// so we allow the fuzzer to control this by feeding us more or less bytes.
    /// The upper bound here is to prevent too many inputs that cause long test times
    pub max_test_case_inputs: usize,
    // Number of functions that we generate per testcase
    pub testcase_funcs: RangeInclusive<usize>,
    pub signature_params: RangeInclusive<usize>,
    pub signature_rets: RangeInclusive<usize>,
    pub instructions_per_block: RangeInclusive<usize>,
    /// Number of variables that we allocate per function
    /// This value does not include the signature params
    pub vars_per_function: RangeInclusive<usize>,
    /// Number of blocks that we generate per function.
    /// This value does not include the entry block
    pub blocks_per_function: RangeInclusive<usize>,
    /// Number of params a block should take
    /// This value does not apply to block0 which takes the function params
    /// and is thus governed by `signature_params`
    pub block_signature_params: RangeInclusive<usize>,
    /// Max number of jump tables entries to generate
    pub jump_table_entries: RangeInclusive<usize>,

    /// The Switch API specializes either individual blocks or contiguous ranges.
    /// In `switch_cases` we decide to produce either a single block or a range.
    /// The size of the range is controlled by `switch_max_range_size`.
    pub switch_cases: RangeInclusive<usize>,
    pub switch_max_range_size: RangeInclusive<usize>,

    /// Stack slots.
    /// The combination of these two determines stack usage per function
    pub static_stack_slots_per_function: RangeInclusive<usize>,
    /// Size in bytes
    pub static_stack_slot_size: RangeInclusive<usize>,
    /// Stack slot alignment as a power of 2
    pub stack_slot_alignment_log2: RangeInclusive<usize>,
    /// Allowed stack probe sizes
    pub stack_probe_size_log2: RangeInclusive<usize>,

    /// Determines how often we generate a backwards branch
    /// Backwards branches are prone to infinite loops, and thus cause timeouts.
    pub backwards_branch_ratio: (usize, usize),

    /// How often should we allow integer division by zero traps.
    ///
    /// Some instructions such as Srem and Udiv can cause a `int_divz` trap
    /// under some inputs. We almost always insert a sequence of instructions
    /// that avoids these issues. However we can allow some `int_divz` traps
    /// by controlling this config.
    pub allowed_int_divz_ratio: (usize, usize),

    /// How often should we allow fcvt related traps.
    ///
    /// `Fcvt*` instructions fail under some inputs, most commonly NaN's.
    /// We insert a checking sequence to guarantee that those inputs never make
    /// it to the instruction, but sometimes we want to allow them.
    pub allowed_fcvt_traps_ratio: (usize, usize),

    /// Some flags really impact compile performance, we still want to test
    /// them, but probably at a lower rate, so that overall execution time isn't
    /// impacted as much
    pub compile_flag_ratio: HashMap<&'static str, (usize, usize)>,

    /// Range of values for the padding between basic blocks. Larger values will
    /// generate larger functions.
    pub bb_padding_log2_size: RangeInclusive<usize>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            max_test_case_inputs: 100,
            testcase_funcs: 1..=8,
            signature_params: 0..=16,
            signature_rets: 0..=16,
            instructions_per_block: 0..=64,
            vars_per_function: 0..=16,
            blocks_per_function: 0..=16,
            block_signature_params: 0..=16,
            jump_table_entries: 0..=16,
            switch_cases: 0..=64,
            // Ranges smaller than 2 don't make sense.
            switch_max_range_size: 2..=32,
            static_stack_slots_per_function: 0..=8,
            static_stack_slot_size: 0..=128,
            stack_slot_alignment_log2: 0..=10,
            // We need the mix of sizes that allows us to:
            //  * not generates any stack probes
            //  * generate unrolled stack probes
            //  * generate loop stack probes
            //
            // This depends on the total amount of stack space that we have for this function
            // (controlled by `static_stack_slots_per_function` and `static_stack_slot_size`)
            //
            // 1<<6 = 64 and 1<<14 = 16384
            //
            // This range allows us to generate all 3 cases within the current allowed
            // stack size range.
            stack_probe_size_log2: 6..=14,
            // 0.1% allows us to explore this, while not causing enough timeouts to significantly
            // impact execs/s
            backwards_branch_ratio: (1, 1000),
            allowed_int_divz_ratio: (1, 1_000_000),
            allowed_fcvt_traps_ratio: (1, 1_000_000),
            compile_flag_ratio: [("regalloc_checker", (1usize, 1000))].into_iter().collect(),
            // Generate up to 4KiB of padding between basic blocks. Although we only
            // explicitly generate up to 16 blocks, after SSA construction we can
            // end up with way more blocks than that (Seeing 400 blocks is not uncommon).
            // At 4KiB we end up at around 1.5MiB of padding per function, which seems reasonable.
            bb_padding_log2_size: 0..=12,
        }
    }
}
