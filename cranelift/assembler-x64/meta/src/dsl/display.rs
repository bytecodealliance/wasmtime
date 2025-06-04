// Fix up the mnemonic for locked instructions: we want to print
// "lock <inst>", not "lock_<inst>".
pub fn lock(mnemonic: &String) -> String {
    let inst_name = format!("lock {}", &mnemonic[5..]);
    inst_name
}
