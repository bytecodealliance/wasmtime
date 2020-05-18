use proptest::prelude::*;
use wiggle::{BorrowChecker, GuestMemory};
use wiggle_test::{impl_errno, HostMemory, MemArea, WasiCtx};

wiggle::from_witx!({
    witx: ["tests/atoms.witx"],
    ctx: WasiCtx,
});

impl_errno!(types::Errno, types::GuestErrorConversion);

impl<'a> atoms::Atoms for WasiCtx<'a> {
    fn int_float_args(&self, an_int: u32, an_float: f32) -> Result<(), types::Errno> {
        println!("INT FLOAT ARGS: {} {}", an_int, an_float);
        Ok(())
    }
    fn double_int_return_float(&self, an_int: u32) -> Result<types::AliasToFloat, types::Errno> {
        Ok((an_int as f32) * 2.0)
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
        let ctx = WasiCtx::new();
        let host_memory = HostMemory::new();
        let bc = BorrowChecker::new();

        let e = atoms::int_float_args(&ctx, &host_memory, &bc, self.an_int as i32, self.an_float);

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
#[derive(Debug)]
struct DoubleIntExercise {
    pub input: u32,
    pub return_loc: MemArea,
}

impl DoubleIntExercise {
    pub fn test(&self) {
        let ctx = WasiCtx::new();
        let host_memory = HostMemory::new();
        let bc = BorrowChecker::new();

        let e = atoms::double_int_return_float(
            &ctx,
            &host_memory,
            &bc,
            self.input as i32,
            self.return_loc.ptr as i32,
        );

        let return_val = host_memory
            .ptr::<types::AliasToFloat>(&bc, self.return_loc.ptr)
            .read()
            .expect("failed to read return");
        assert_eq!(e, types::Errno::Ok.into(), "errno");
        assert_eq!(return_val, (self.input as f32) * 2.0, "return val");
    }

    pub fn strat() -> BoxedStrategy<Self> {
        (prop::num::u32::ANY, HostMemory::mem_area_strat(4))
            .prop_map(|(input, return_loc)| DoubleIntExercise { input, return_loc })
            .boxed()
    }
}

proptest! {
    #[test]
    fn double_int_return_float(e in DoubleIntExercise::strat()) {
        e.test()
    }
}
