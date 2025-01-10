use proptest::prelude::*;
use wiggle::{GuestMemory, GuestPtr};
use wiggle_test::{HostMemory, MemArea, WasiCtx, impl_errno};

wiggle::from_witx!({
    witx: ["$CARGO_MANIFEST_DIR/tests/ints.witx"],
});

impl_errno!(types::Errno);

impl<'a> ints::Ints for WasiCtx<'a> {
    fn cookie_cutter(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        init_cookie: types::Cookie,
    ) -> Result<types::Bool, types::Errno> {
        let res = if init_cookie == types::COOKIE_START {
            types::Bool::True
        } else {
            types::Bool::False
        };
        Ok(res)
    }
}

fn cookie_strat() -> impl Strategy<Value = types::Cookie> {
    (0..std::u64::MAX)
        .prop_map(|x| types::Cookie::try_from(x).expect("within range of cookie"))
        .boxed()
}

#[derive(Debug)]
struct CookieCutterExercise {
    cookie: types::Cookie,
    return_ptr_loc: MemArea,
}

impl CookieCutterExercise {
    pub fn strat() -> BoxedStrategy<Self> {
        (cookie_strat(), HostMemory::mem_area_strat(4))
            .prop_map(|(cookie, return_ptr_loc)| Self {
                cookie,
                return_ptr_loc,
            })
            .boxed()
    }

    pub fn test(&self) {
        let mut ctx = WasiCtx::new();
        let mut host_memory = HostMemory::new();
        let mut memory = host_memory.guest_memory();

        let res = ints::cookie_cutter(
            &mut ctx,
            &mut memory,
            self.cookie as i64,
            self.return_ptr_loc.ptr as i32,
        )
        .unwrap();
        assert_eq!(res, types::Errno::Ok as i32, "cookie cutter errno");

        let is_cookie_start = memory
            .read(GuestPtr::<types::Bool>::new(self.return_ptr_loc.ptr))
            .expect("deref to Bool value");

        assert_eq!(
            if is_cookie_start == types::Bool::True {
                true
            } else {
                false
            },
            self.cookie == types::COOKIE_START,
            "returned Bool should test if input was Cookie::START",
        );
    }
}
proptest! {
    #[test]
    fn cookie_cutter(e in CookieCutterExercise::strat()) {
        e.test()
    }
}
