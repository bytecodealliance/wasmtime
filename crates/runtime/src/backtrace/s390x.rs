pub unsafe fn get_next_older_pc_from_fp(fp: usize) -> usize {
    *(fp as *mut usize).offset(14)
}

pub unsafe fn get_next_older_fp_from_fp(fp: usize) -> usize {
    *(fp as *mut usize)
}

pub fn reached_entry_sp(fp: usize, first_wasm_sp: usize) -> bool {
    fp == first_wasm_sp
}

pub fn assert_entry_sp_is_aligned(sp: usize) {
    assert_eq!(sp % 8, 0, "stack should always be aligned to 8");
}

pub fn assert_fp_is_aligned(fp: usize) {
    assert_eq!(fp % 8, 0, "stack should always be aligned to 8");
}
