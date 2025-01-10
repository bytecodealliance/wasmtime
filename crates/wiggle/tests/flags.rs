use proptest::prelude::*;
use wiggle::{GuestMemory, GuestPtr};
use wiggle_test::{HostMemory, MemArea, WasiCtx, impl_errno};

wiggle::from_witx!({
    witx: ["$CARGO_MANIFEST_DIR/tests/flags.witx"],
});

impl_errno!(types::Errno);

impl<'a> flags::Flags for WasiCtx<'a> {
    fn configure_car(
        &mut self,
        memory: &mut GuestMemory<'_>,
        old_config: types::CarConfig,
        other_config_ptr: GuestPtr<types::CarConfig>,
    ) -> Result<types::CarConfig, types::Errno> {
        let other_config = memory.read(other_config_ptr).map_err(|e| {
            eprintln!("old_config_ptr error: {e}");
            types::Errno::InvalidArg
        })?;
        Ok(old_config ^ other_config)
    }
}

fn car_config_strat() -> impl Strategy<Value = types::CarConfig> {
    (1u8..=types::CarConfig::all().into())
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
                MemArea::non_overlapping_set(&[e.other_config_by_ptr, e.return_ptr_loc])
            })
            .boxed()
    }

    pub fn test(&self) {
        let mut ctx = WasiCtx::new();
        let mut host_memory = HostMemory::new();
        let mut memory = host_memory.guest_memory();

        // Populate input ptr
        memory
            .write(
                GuestPtr::new(self.other_config_by_ptr.ptr),
                self.other_config,
            )
            .expect("deref ptr mut to CarConfig");

        let res = flags::configure_car(
            &mut ctx,
            &mut memory,
            self.old_config.bits() as i32,
            self.other_config_by_ptr.ptr as i32,
            self.return_ptr_loc.ptr as i32,
        )
        .unwrap();
        assert_eq!(res, types::Errno::Ok as i32, "configure car errno");

        let res_config = memory
            .read(GuestPtr::<types::CarConfig>::new(self.return_ptr_loc.ptr))
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

#[test]
fn flags_fmt() {
    let empty = format!("{}", types::CarConfig::empty());
    assert_eq!(empty, "CarConfig(CarConfig(0x0) (0x0))");
    let one_flag = format!("{}", types::CarConfig::AWD);
    assert_eq!(one_flag, "CarConfig(CarConfig(AWD) (0x2))");
    let two_flags = format!("{}", types::CarConfig::AUTOMATIC | types::CarConfig::SUV);
    assert_eq!(two_flags, "CarConfig(CarConfig(AUTOMATIC | SUV) (0x5))");
    let all = format!("{}", types::CarConfig::all());
    assert_eq!(all, "CarConfig(CarConfig(AUTOMATIC | AWD | SUV) (0x7))");
}
