pub mod test {
    generate::from_witx!("test.witx");

    pub struct WasiCtx {} // FIXME: parameterize macro on what ctx type is used here

    impl types::WitxErrorConversion for WasiCtx {
        fn success_to_errno(&mut self) -> types::Errno {
            types::Errno::Ok
        }
        fn memory_error_to_errno(&mut self, e: ::memory::MemoryError) -> types::Errno {
            eprintln!("memory error: {:?}", e);
            types::Errno::InvalidArg
        }
        fn value_error_to_errno(&mut self, e: ::memory::GuestValueError) -> types::Errno {
            eprintln!("guest value error: {:?}", e);
            types::Errno::InvalidArg
        }
    }
}
/*
pub mod wasi {
    generate::from_witx!("crates/WASI/phases/snapshot/witx/wasi_snapshot_preview1.witx");
}
*/
