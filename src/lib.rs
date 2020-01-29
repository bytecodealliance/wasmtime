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
            // Read enum value from mutable:
            let mut a_better_excuse_ref: ::memory::GuestRefMut<types::Excuse> =
                a_better_excuse_by_reference.as_ref_mut().map_err(|e| {
                    eprintln!("a_better_excuse_by_reference error: {}", e);
                    types::Errno::InvalidArg
                })?;
            let a_better_excuse: types::Excuse = *a_better_excuse_ref;

            // Read enum value from immutable ptr:
            let a_lamer_excuse = *a_lamer_excuse_by_reference.as_ref().map_err(|e| {
                eprintln!("a_lamer_excuse_by_reference error: {}", e);
                types::Errno::InvalidArg
            })?;

            // Write enum to mutable ptr:
            *a_better_excuse_ref = a_lamer_excuse;

            // Read ptr value from mutable ptr:
            let one_layer_down: ::memory::GuestPtr<types::Excuse> =
                two_layers_of_excuses.read_ptr_from_guest().map_err(|e| {
                    eprintln!("one_layer_down error: {}", e);
                    types::Errno::InvalidArg
                })?;

            // Read enum value from that ptr:
            let two_layers_down: types::Excuse = *one_layer_down.as_ref().map_err(|e| {
                eprintln!("two_layers_down error: {}", e);
                types::Errno::InvalidArg
            })?;

            // Write ptr value to mutable ptr:
            two_layers_of_excuses.write_ptr_to_guest(&a_better_excuse_by_reference.as_immut());

            println!(
                "BAZ: excuse: {:?}, better excuse: {:?}, lamer excuse: {:?}, two layers down: {:?}",
                excuse, a_better_excuse, a_lamer_excuse, two_layers_down
            );
            Ok(())
        }

        fn bat(&mut self, an_int: u32) -> Result<f32, types::Errno> {
            println!("bat: {}", an_int);
            Ok((an_int as f32) * 2.0)
        }

        fn sum_of_pair(&mut self, an_pair: &types::PairInts) -> Result<i64, types::Errno> {
            println!("sum of pair: {:?}", an_pair);
            Ok(an_pair.first as i64 + an_pair.second as i64)
        }
        fn sum_of_pair_of_ptrs(
            &mut self,
            an_pair: &types::PairIntPtrs,
        ) -> Result<i64, types::Errno> {
            let first = *an_pair.first.as_ref().unwrap();
            let second = *an_pair.second.as_ref().unwrap();
            println!("sum of pair of ptrs: {} + {}", first, second);
            Ok(first as i64 + second as i64)
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
