pub mod test {
    // FIXME: parameterize macro on what ctx type is used here
    generate::from_witx!("test.witx");

    pub struct WasiCtx {
        mem_errors: Vec<::memory::MemoryError>,
        value_errors: Vec<::memory::GuestValueError>,
    }

    // Errno is used as a first return value in the functions above, therefore
    // it must implement GuestError with type Context = WasiCtx.
    // The context type should let you do logging or debugging or whatever you need
    // with these errors. We just push them to vecs.
    impl ::memory::GuestError for types::Errno {
        type Context = WasiCtx;
        fn is_success(&self) -> bool {
            match self {
                types::Errno::Ok => true,
                _ => false,
            }
        }
        fn from_memory_error(e: ::memory::MemoryError, ctx: &mut WasiCtx) -> types::Errno {
            ctx.mem_errors.push(e);
            types::Errno::InvalidArg
        }
        fn from_value_error(e: ::memory::GuestValueError, ctx: &mut WasiCtx) -> types::Errno {
            ctx.value_errors.push(e);
            types::Errno::InvalidArg
        }
    }
}
/*
pub mod wasi {
    generate::from_witx!("crates/WASI/phases/snapshot/witx/wasi_snapshot_preview1.witx");
}
*/
