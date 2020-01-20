pub mod test {
    pub struct WasiCtx {} // FIXME: parameterize macro on what ctx type is used here
    generate::from_witx!("test.witx");
}
/*
pub mod wasi {
    generate::from_witx!("crates/WASI/phases/snapshot/witx/wasi_snapshot_preview1.witx");
}
*/
