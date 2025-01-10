use proptest::prelude::*;
use wiggle::{GuestMemory, GuestPtr};
use wiggle_test::{HostMemory, MemArea, WasiCtx, impl_errno};

wiggle::from_witx!({
    witx: ["$CARGO_MANIFEST_DIR/tests/atoms.witx"],
});

impl_errno!(types::Errno);

impl<'a> atoms::Atoms for WasiCtx<'a> {
    fn int_float_args(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        an_int: u32,
        an_float: f32,
    ) -> Result<(), types::Errno> {
        println!("INT FLOAT ARGS: {an_int} {an_float}");
        Ok(())
    }
    fn double_int_return_float(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        an_int: u32,
    ) -> Result<types::AliasToFloat, types::Errno> {
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
        let mut ctx = WasiCtx::new();
        let mut host_memory = HostMemory::new();

        let e = atoms::int_float_args(
            &mut ctx,
            &mut host_memory.guest_memory(),
            self.an_int as i32,
            self.an_float,
        )
        .unwrap();

        assert_eq!(e, types::Errno::Ok as i32, "int_float_args error");
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
        let mut ctx = WasiCtx::new();
        let mut host_memory = HostMemory::new();
        let mut memory = host_memory.guest_memory();

        let e = atoms::double_int_return_float(
            &mut ctx,
            &mut memory,
            self.input as i32,
            self.return_loc.ptr as i32,
        )
        .unwrap();

        let return_val = memory
            .read(GuestPtr::<types::AliasToFloat>::new(self.return_loc.ptr))
            .expect("failed to read return");
        assert_eq!(e, types::Errno::Ok as i32, "errno");
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
