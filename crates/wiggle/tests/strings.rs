use proptest::prelude::*;
use wiggle::{GuestMemory, GuestPtr};
use wiggle_test::{impl_errno, HostMemory, MemArea, MemAreas, WasiCtx};

wiggle::from_witx!({
    witx: ["$CARGO_MANIFEST_DIR/tests/strings.witx"],
});

impl_errno!(types::Errno);

impl<'a> strings::Strings for WasiCtx<'a> {
    fn hello_string(
        &mut self,
        memory: &mut GuestMemory<'_>,
        a_string: GuestPtr<str>,
    ) -> Result<u32, types::Errno> {
        let s = memory
            .as_str(a_string)
            .expect("should be valid string")
            .expect("expected non-shared memory");
        println!("a_string='{}'", &*s);
        Ok(s.len() as u32)
    }

    fn multi_string(
        &mut self,
        memory: &mut GuestMemory<'_>,
        a: GuestPtr<str>,
        b: GuestPtr<str>,
        c: GuestPtr<str>,
    ) -> Result<u32, types::Errno> {
        let sa = memory
            .as_str(a)
            .expect("A should be valid string")
            .expect("expected non-shared memory");
        let sb = memory
            .as_str(b)
            .expect("B should be valid string")
            .expect("expected non-shared memory");
        let sc = memory
            .as_str(c)
            .expect("C should be valid string")
            .expect("expected non-shared memory");
        let total_len = sa.len() + sb.len() + sc.len();
        println!(
            "len={}, a='{}', b='{}', c='{}'",
            total_len, &*sa, &*sb, &*sc
        );
        Ok(total_len as u32)
    }
}

fn unicode_string_strategy() -> impl Strategy<Value = String> {
    "\\p{Greek}{1,256}"
}
fn ascii_string_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0..9]{1,256}"
}

#[derive(Debug)]
struct HelloStringExercise {
    test_word: String,
    string_ptr_loc: MemArea,
    return_ptr_loc: MemArea,
}

impl HelloStringExercise {
    pub fn strat() -> BoxedStrategy<Self> {
        (unicode_string_strategy(),)
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
        let mut ctx = WasiCtx::new();
        let mut host_memory = HostMemory::new();
        let mut memory = host_memory.guest_memory();

        // Populate string in guest's memory
        let ptr = GuestPtr::<str>::new((self.string_ptr_loc.ptr, self.test_word.len() as u32));
        for (slot, byte) in ptr.as_bytes().iter().zip(self.test_word.bytes()) {
            memory
                .write(slot.expect("should be valid pointer"), byte)
                .expect("failed to write");
        }

        let res = strings::hello_string(
            &mut ctx,
            &mut memory,
            self.string_ptr_loc.ptr as i32,
            self.test_word.len() as i32,
            self.return_ptr_loc.ptr as i32,
        )
        .unwrap();
        assert_eq!(res, types::Errno::Ok as i32, "hello string errno");

        let given = memory
            .read(GuestPtr::<u32>::new(self.return_ptr_loc.ptr))
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
            unicode_string_strategy(),
            unicode_string_strategy(),
            unicode_string_strategy(),
            HostMemory::mem_area_strat(4),
        )
            .prop_flat_map(|(a, b, c, return_ptr_loc)| {
                (
                    Just(a.clone()),
                    Just(b.clone()),
                    Just(c.clone()),
                    HostMemory::byte_slice_strat(
                        a.len() as u32,
                        1,
                        &MemAreas::from([return_ptr_loc]),
                    ),
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
                        1,
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
                        1,
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
        let mut ctx = WasiCtx::new();
        let mut host_memory = HostMemory::new();
        let mut memory = host_memory.guest_memory();

        let mut write_string = |val: &str, loc: MemArea| {
            let ptr = GuestPtr::<str>::new((loc.ptr, val.len() as u32));
            for (slot, byte) in ptr.as_bytes().iter().zip(val.bytes()) {
                memory
                    .write(slot.expect("should be valid pointer"), byte)
                    .expect("failed to write");
            }
        };

        write_string(&self.a, self.sa_ptr_loc);
        write_string(&self.b, self.sb_ptr_loc);
        write_string(&self.c, self.sc_ptr_loc);

        let res = strings::multi_string(
            &mut ctx,
            &mut memory,
            self.sa_ptr_loc.ptr as i32,
            self.a.len() as i32,
            self.sb_ptr_loc.ptr as i32,
            self.b.len() as i32,
            self.sc_ptr_loc.ptr as i32,
            self.c.len() as i32,
            self.return_ptr_loc.ptr as i32,
        )
        .unwrap();
        assert_eq!(res, types::Errno::Ok as i32, "multi string errno");

        let given = memory
            .read(GuestPtr::<u32>::new(self.return_ptr_loc.ptr))
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

#[derive(Debug)]
struct OverlappingStringExercise {
    a: String,
    sa_ptr_loc: MemArea,
    offset_b: u32,
    offset_c: u32,
    return_ptr_loc: MemArea,
}

impl OverlappingStringExercise {
    pub fn strat() -> BoxedStrategy<Self> {
        // using ascii so we can window into it without worrying about codepoints
        (ascii_string_strategy(), HostMemory::mem_area_strat(4))
            .prop_flat_map(|(a, return_ptr_loc)| {
                (
                    Just(a.clone()),
                    HostMemory::mem_area_strat(a.len() as u32),
                    0..(a.len() as u32),
                    0..(a.len() as u32),
                    Just(return_ptr_loc),
                )
            })
            .prop_map(|(a, sa_ptr_loc, offset_b, offset_c, return_ptr_loc)| Self {
                a,
                sa_ptr_loc,
                offset_b,
                offset_c,
                return_ptr_loc,
            })
            .prop_filter("non-overlapping pointers", |e| {
                MemArea::non_overlapping_set(&[e.sa_ptr_loc, e.return_ptr_loc])
            })
            .boxed()
    }

    pub fn test(&self) {
        let mut ctx = WasiCtx::new();
        let mut host_memory = HostMemory::new();
        let mut memory = host_memory.guest_memory();

        let mut write_string = |val: &str, loc: MemArea| {
            let ptr = GuestPtr::<str>::new((loc.ptr, val.len() as u32));
            for (slot, byte) in ptr.as_bytes().iter().zip(val.bytes()) {
                memory
                    .write(slot.expect("should be valid pointer"), byte)
                    .expect("failed to write");
            }
        };

        write_string(&self.a, self.sa_ptr_loc);

        let a_len = self.a.as_bytes().len() as i32;
        let res = strings::multi_string(
            &mut ctx,
            &mut memory,
            self.sa_ptr_loc.ptr as i32,
            a_len,
            (self.sa_ptr_loc.ptr + self.offset_b) as i32,
            a_len - self.offset_b as i32,
            (self.sa_ptr_loc.ptr + self.offset_c) as i32,
            a_len - self.offset_c as i32,
            self.return_ptr_loc.ptr as i32,
        )
        .unwrap();
        assert_eq!(res, types::Errno::Ok as i32, "multi string errno");

        let given = memory
            .read(GuestPtr::<u32>::new(self.return_ptr_loc.ptr))
            .expect("deref ptr to return value");
        assert_eq!(
            ((3 * a_len) - (self.offset_b as i32 + self.offset_c as i32)) as u32,
            given
        );
    }
}

proptest! {
    #[test]
    fn overlapping_string(e in OverlappingStringExercise::strat()) {
        e.test()
    }
}
