pub mod test {
    // FIXME: parameterize macro on what ctx type is used here
    generate::from_witx!("test.witx");

    pub struct WasiCtx {
        guest_errors: Vec<::memory::GuestError>,
    }

    impl foo::Foo for WasiCtx {
        fn bar(&mut self, an_int: u32, an_float: f32) -> Result<(), types::Errno> {
            println!("BAR: {} {}", an_int, an_float);
            Ok(())
        }
        fn baz(
            &mut self,
            excuse: types::Excuse,
            a_better_excuse_by_reference: ::memory::GuestPtrMut<types::Excuse>,
            a_lamer_excuse_by_reference: ::memory::GuestPtr<types::Excuse>,
            two_layers_of_excuses: ::memory::GuestPtrMut<::memory::GuestPtr<types::Excuse>>,
        ) -> Result<(), types::Errno> {
            use memory::GuestTypeCopy;

            // Read enum value from mutable:
            let a_better_excuse =
                types::Excuse::read_val(&a_better_excuse_by_reference).map_err(|e| {
                    eprintln!("a_better_excuse_by_reference error: {}", e);
                    types::Errno::InvalidArg
                })?;

            // Read enum value from immutable ptr:
            let a_lamer_excuse =
                types::Excuse::read_val(&a_lamer_excuse_by_reference).map_err(|e| {
                    eprintln!("a_lamer_excuse_by_reference error: {}", e);
                    types::Errno::InvalidArg
                })?;

            // Write enum to mutable ptr:
            types::Excuse::write_val(a_lamer_excuse, &a_better_excuse_by_reference);

            // Read ptr value from mutable ptr:
            let one_layer_down =
                ::memory::GuestPtr::read_ptr(&two_layers_of_excuses).map_err(|e| {
                    eprintln!("one_layer_down error: {}", e);
                    types::Errno::InvalidArg
                })?;

            // Read enum value from that ptr:
            let two_layers_down = types::Excuse::read_val(&one_layer_down).map_err(|e| {
                eprintln!("two_layers_down error: {}", e);
                types::Errno::InvalidArg
            })?;

            // Write ptr value to mutable ptr:
            ::memory::GuestPtr::write_ptr(
                &a_better_excuse_by_reference.as_immut(),
                &two_layers_of_excuses,
            );

            println!(
                "BAZ: excuse: {:?}, better excuse: {:?}, lamer excuse: {:?}, two layers down: {:?}",
                excuse, a_better_excuse, a_lamer_excuse, two_layers_down
            );
            Ok(())
        }
    }

    // Errno is used as a first return value in the functions above, therefore
    // it must implement GuestErrorType with type Context = WasiCtx.
    // The context type should let you do logging or debugging or whatever you need
    // with these errors. We just push them to vecs.
    impl ::memory::GuestErrorType for types::Errno {
        type Context = WasiCtx;
        fn success() -> types::Errno {
            types::Errno::Ok
        }
        fn from_error(e: ::memory::GuestError, ctx: &mut WasiCtx) -> types::Errno {
            ctx.guest_errors.push(e);
            types::Errno::InvalidArg
        }
    }
}
/*
pub mod wasi {
    generate::from_witx!("crates/WASI/phases/snapshot/witx/wasi_snapshot_preview1.witx");
}
*/
