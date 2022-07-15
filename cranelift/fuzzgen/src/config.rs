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
    pub jump_tables_per_function: RangeInclusive<usize>,
    pub jump_table_entries: RangeInclusive<usize>,

    /// Stack slots.
    /// The combination of these two determines stack usage per function
    pub static_stack_slots_per_function: RangeInclusive<usize>,
    /// Size in bytes
    pub static_stack_slot_size: RangeInclusive<usize>,
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
            jump_tables_per_function: 0..=4,
            jump_table_entries: 0..=16,
            static_stack_slots_per_function: 0..=8,
            static_stack_slot_size: 0..=128,
        }
    }
}
