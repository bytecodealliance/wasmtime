use std::ops::RangeInclusive;

/// Holds the range of acceptable values to use during the generation of testcases
pub struct Config {
    pub test_case_inputs: RangeInclusive<usize>,
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

    pub funcrefs_per_function: RangeInclusive<usize>,

    /// Stack slots.
    /// The combination of these two determines stack usage per function
    pub static_stack_slots_per_function: RangeInclusive<usize>,
    /// Size in bytes
    pub static_stack_slot_size: RangeInclusive<usize>,

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
}

impl Default for Config {
    fn default() -> Self {
        Config {
            test_case_inputs: 1..=10,
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
            funcrefs_per_function: 0..=8,
            static_stack_slots_per_function: 0..=8,
            static_stack_slot_size: 0..=128,
            // 0.1% allows us to explore this, while not causing enough timeouts to significantly
            // impact execs/s
            backwards_branch_ratio: (1, 1000),
            allowed_int_divz_ratio: (1, 1_000_000),
            allowed_fcvt_traps_ratio: (1, 1_000_000),
        }
    }
}
