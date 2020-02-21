use proptest::prelude::*;
use wiggle_runtime::GuestError;
use wiggle_test::HostMemory;

mod ctx;
use ctx::WasiCtx;

wiggle_generate::from_witx!({
    witx: ["tests/trivial.witx"],
    ctx: WasiCtx,
});

impl_errno!(types::Errno);

impl trivial::Trivial for WasiCtx {
    fn int_float_args(&mut self, an_int: u32, an_float: f32) -> Result<(), types::Errno> {
        println!("INT FLOAT ARGS: {} {}", an_int, an_float);
        Ok(())
    }
}

// There's nothing meaningful to test here - this just demonstrates the test machinery

#[derive(Debug)]
struct IntFloatExercise {
    pub an_int: u32,
    pub an_float: f32,
}

impl IntFloatExercise {
    pub fn test(&self) {
        let mut ctx = WasiCtx::new();
        let mut host_memory = HostMemory::new();
        let mut guest_memory = host_memory.guest_memory();

        let e = trivial::int_float_args(
            &mut ctx,
            &mut guest_memory,
            self.an_int as i32,
            self.an_float,
        );

        assert_eq!(e, types::Errno::Ok.into(), "int_float_args error");
    }

    pub fn strat() -> BoxedStrategy<Self> {
        (prop::num::u32::ANY, prop::num::f32::ANY)
            .prop_map(|(an_int, an_float)| IntFloatExercise { an_int, an_float })
            .boxed()
    }
}

proptest! {
    #[test]
    fn int_float_exercise(e in IntFloatExercise::strat()) {
        e.test()
    }
}
