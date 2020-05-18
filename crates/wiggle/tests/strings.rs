use proptest::prelude::*;
use wiggle::{BorrowChecker, GuestMemory, GuestPtr};
use wiggle_test::{impl_errno, HostMemory, MemArea, MemAreas, WasiCtx};

wiggle::from_witx!({
    witx: ["tests/strings.witx"],
    ctx: WasiCtx,
});

impl_errno!(types::Errno, types::GuestErrorConversion);

impl<'a> strings::Strings for WasiCtx<'a> {
    fn hello_string(&self, a_string: &GuestPtr<str>) -> Result<u32, types::Errno> {
        let s = a_string.as_str().expect("should be valid string");
        println!("a_string='{}'", &*s);
        Ok(s.len() as u32)
    }

    fn multi_string(
        &self,
        a: &GuestPtr<str>,
        b: &GuestPtr<str>,
        c: &GuestPtr<str>,
    ) -> Result<u32, types::Errno> {
        let sa = a.as_str().expect("A should be valid string");
        let sb = b.as_str().expect("B should be valid string");
        let sc = c.as_str().expect("C should be valid string");
        let total_len = sa.len() + sb.len() + sc.len();
        println!(
            "len={}, a='{}', b='{}', c='{}'",
            total_len, &*sa, &*sb, &*sc
        );
        Ok(total_len as u32)
    }
}

fn test_string_strategy() -> impl Strategy<Value = String> {
    "\\p{Greek}{1,256}"
}

#[derive(Debug)]
struct HelloStringExercise {
    test_word: String,
    string_ptr_loc: MemArea,
    return_ptr_loc: MemArea,
}

impl HelloStringExercise {
    pub fn strat() -> BoxedStrategy<Self> {
        (test_string_strategy(),)
            .prop_flat_map(|(test_word,)| {
                (
                    Just(test_word.clone()),
                    HostMemory::mem_area_strat(test_word.len() as u32),
                    HostMemory::mem_area_strat(4),
                )
            })
            .prop_map(|(test_word, string_ptr_loc, return_ptr_loc)| Self {
                test_word,
                string_ptr_loc,
                return_ptr_loc,
            })
            .prop_filter("non-overlapping pointers", |e| {
                MemArea::non_overlapping_set(&[e.string_ptr_loc, e.return_ptr_loc])
            })
            .boxed()
    }

    pub fn test(&self) {
        let ctx = WasiCtx::new();
        let host_memory = HostMemory::new();
        let bc = BorrowChecker::new();

        // Populate string in guest's memory
        let ptr =
            host_memory.ptr::<str>(&bc, (self.string_ptr_loc.ptr, self.test_word.len() as u32));
        for (slot, byte) in ptr.as_bytes().iter().zip(self.test_word.bytes()) {
            slot.expect("should be valid pointer")
                .write(byte)
                .expect("failed to write");
        }

        let res = strings::hello_string(
            &ctx,
            &host_memory,
            &bc,
            self.string_ptr_loc.ptr as i32,
            self.test_word.len() as i32,
            self.return_ptr_loc.ptr as i32,
        );
        assert_eq!(res, types::Errno::Ok.into(), "hello string errno");

        let given = host_memory
            .ptr::<u32>(&bc, self.return_ptr_loc.ptr)
            .read()
            .expect("deref ptr to return value");
        assert_eq!(self.test_word.len() as u32, given);
    }
}
proptest! {
    #[test]
    fn hello_string(e in HelloStringExercise::strat()) {
        e.test()
    }
}

#[derive(Debug)]
struct MultiStringExercise {
    a: String,
    b: String,
    c: String,
    sa_ptr_loc: MemArea,
    sb_ptr_loc: MemArea,
    sc_ptr_loc: MemArea,
    return_ptr_loc: MemArea,
}

impl MultiStringExercise {
    pub fn strat() -> BoxedStrategy<Self> {
        (
            test_string_strategy(),
            test_string_strategy(),
            test_string_strategy(),
            HostMemory::mem_area_strat(4),
        )
            .prop_flat_map(|(a, b, c, return_ptr_loc)| {
                (
                    Just(a.clone()),
                    Just(b.clone()),
                    Just(c.clone()),
                    HostMemory::byte_slice_strat(a.len() as u32, &MemAreas::from([return_ptr_loc])),
                    Just(return_ptr_loc),
                )
            })
            .prop_flat_map(|(a, b, c, sa_ptr_loc, return_ptr_loc)| {
                (
                    Just(a.clone()),
                    Just(b.clone()),
                    Just(c.clone()),
                    Just(sa_ptr_loc),
                    HostMemory::byte_slice_strat(
                        b.len() as u32,
                        &MemAreas::from([sa_ptr_loc, return_ptr_loc]),
                    ),
                    Just(return_ptr_loc),
                )
            })
            .prop_flat_map(|(a, b, c, sa_ptr_loc, sb_ptr_loc, return_ptr_loc)| {
                (
                    Just(a.clone()),
                    Just(b.clone()),
                    Just(c.clone()),
                    Just(sa_ptr_loc),
                    Just(sb_ptr_loc),
                    HostMemory::byte_slice_strat(
                        c.len() as u32,
                        &MemAreas::from([sa_ptr_loc, sb_ptr_loc, return_ptr_loc]),
                    ),
                    Just(return_ptr_loc),
                )
            })
            .prop_map(
                |(a, b, c, sa_ptr_loc, sb_ptr_loc, sc_ptr_loc, return_ptr_loc)| {
                    MultiStringExercise {
                        a,
                        b,
                        c,
                        sa_ptr_loc,
                        sb_ptr_loc,
                        sc_ptr_loc,
                        return_ptr_loc,
                    }
                },
            )
            .boxed()
    }

    pub fn test(&self) {
        let ctx = WasiCtx::new();
        let host_memory = HostMemory::new();
        let bc = BorrowChecker::new();

        let write_string = |val: &str, loc: MemArea| {
            let ptr = host_memory.ptr::<str>(&bc, (loc.ptr, val.len() as u32));
            for (slot, byte) in ptr.as_bytes().iter().zip(val.bytes()) {
                slot.expect("should be valid pointer")
                    .write(byte)
                    .expect("failed to write");
            }
        };

        write_string(&self.a, self.sa_ptr_loc);
        write_string(&self.b, self.sb_ptr_loc);
        write_string(&self.c, self.sc_ptr_loc);

        let res = strings::multi_string(
            &ctx,
            &host_memory,
            &bc,
            self.sa_ptr_loc.ptr as i32,
            self.a.len() as i32,
            self.sb_ptr_loc.ptr as i32,
            self.b.len() as i32,
            self.sc_ptr_loc.ptr as i32,
            self.c.len() as i32,
            self.return_ptr_loc.ptr as i32,
        );
        assert_eq!(res, types::Errno::Ok.into(), "multi string errno");

        let given = host_memory
            .ptr::<u32>(&bc, self.return_ptr_loc.ptr)
            .read()
            .expect("deref ptr to return value");
        assert_eq!((self.a.len() + self.b.len() + self.c.len()) as u32, given);
    }
}
proptest! {
    #[test]
    fn multi_string(e in MultiStringExercise::strat()) {
        e.test()
    }
}
