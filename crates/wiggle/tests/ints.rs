use proptest::prelude::*;
use std::convert::TryFrom;
use wiggle::GuestMemory;
use wiggle_test::{impl_errno, HostMemory, MemArea, WasiCtx};

wiggle::from_witx!({
    witx: ["tests/ints.witx"],
    ctx: WasiCtx,
});

impl_errno!(types::Errno, types::GuestErrorConversion);

impl<'a> ints::Ints for WasiCtx<'a> {
    fn cookie_cutter(&self, init_cookie: types::Cookie) -> Result<types::Bool, types::Errno> {
        let res = if init_cookie == types::Cookie::START {
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
        let ctx = WasiCtx::new();
        let host_memory = HostMemory::new();

        let res = ints::cookie_cutter(
            &ctx,
            &host_memory,
            self.cookie.into(),
            self.return_ptr_loc.ptr as i32,
        );
        assert_eq!(res, types::Errno::Ok.into(), "cookie cutter errno");

        let is_cookie_start = host_memory
            .ptr::<types::Bool>(self.return_ptr_loc.ptr)
            .read()
            .expect("deref to Bool value");

        assert_eq!(
            if is_cookie_start == types::Bool::True {
                true
            } else {
                false
            },
            self.cookie == types::Cookie::START,
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
