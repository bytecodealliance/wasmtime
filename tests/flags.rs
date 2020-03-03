use proptest::prelude::*;
use std::convert::TryFrom;
use wiggle_runtime::{GuestError, GuestPtr};
use wiggle_test::{impl_errno, HostMemory, MemArea, WasiCtx};

wiggle::from_witx!({
    witx: ["tests/flags.witx"],
    ctx: WasiCtx,
});

impl_errno!(types::Errno);

impl flags::Flags for WasiCtx {
    fn configure_car(
        &self,
        old_config: types::CarConfig,
        other_config_ptr: GuestPtr<types::CarConfig>,
    ) -> Result<types::CarConfig, types::Errno> {
        let other_config = *other_config_ptr.as_ref().map_err(|e| {
            eprintln!("old_config_ptr error: {}", e);
            types::Errno::InvalidArg
        })?;
        Ok(old_config ^ other_config)
    }
}

fn car_config_strat() -> impl Strategy<Value = types::CarConfig> {
    (1u8..=types::CarConfig::ALL_FLAGS.into())
        .prop_map(|v| {
            types::CarConfig::try_from(v).expect("invalid value for types::CarConfig flag")
        })
        .boxed()
}

#[derive(Debug)]
struct ConfigureCarExercise {
    old_config: types::CarConfig,
    other_config: types::CarConfig,
    other_config_by_ptr: MemArea,
    return_ptr_loc: MemArea,
}

impl ConfigureCarExercise {
    pub fn strat() -> BoxedStrategy<Self> {
        (
            car_config_strat(),
            car_config_strat(),
            HostMemory::mem_area_strat(4),
            HostMemory::mem_area_strat(4),
        )
            .prop_map(
                |(old_config, other_config, other_config_by_ptr, return_ptr_loc)| Self {
                    old_config,
                    other_config,
                    other_config_by_ptr,
                    return_ptr_loc,
                },
            )
            .prop_filter("non-overlapping ptrs", |e| {
                MemArea::non_overlapping_set(&[&e.other_config_by_ptr, &e.return_ptr_loc])
            })
            .boxed()
    }

    pub fn test(&self) {
        let mut ctx = WasiCtx::new();
        let mut host_memory = HostMemory::new();
        let mut guest_memory = host_memory.guest_memory();

        // Populate input ptr
        *guest_memory
            .ptr_mut(self.other_config_by_ptr.ptr)
            .expect("ptr mut to CarConfig")
            .as_ref_mut()
            .expect("deref ptr mut to CarConfig") = self.other_config;

        let res = flags::configure_car(
            &mut ctx,
            &mut guest_memory,
            self.old_config.into(),
            self.other_config_by_ptr.ptr as i32,
            self.return_ptr_loc.ptr as i32,
        );
        assert_eq!(res, types::Errno::Ok.into(), "configure car errno");

        let res_config = *guest_memory
            .ptr::<types::CarConfig>(self.return_ptr_loc.ptr)
            .expect("ptr to returned CarConfig")
            .as_ref()
            .expect("deref to CarConfig value");

        assert_eq!(
            self.old_config ^ self.other_config,
            res_config,
            "returned CarConfig should be an XOR of inputs"
        );
    }
}
proptest! {
    #[test]
    fn configure_car(e in ConfigureCarExercise::strat()) {
        e.test()
    }
}
