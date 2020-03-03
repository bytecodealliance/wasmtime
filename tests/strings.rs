use proptest::prelude::*;
use wiggle_runtime::{GuestError, GuestPtrMut, GuestString};
use wiggle_test::{impl_errno, HostMemory, MemArea, WasiCtx};

wiggle::from_witx!({
    witx: ["tests/strings.witx"],
    ctx: WasiCtx,
});

impl_errno!(types::Errno);

impl strings::Strings for WasiCtx {
    fn hello_string(&self, a_string: &GuestString<'_>) -> Result<u32, types::Errno> {
        let as_ref = a_string.as_ref().expect("deref ptr should succeed");
        let as_str = as_ref.as_str().expect("valid UTF-8 string");
        println!("a_string='{}'", as_str);
        Ok(as_str.len() as u32)
    }
}

fn test_string_strategy() -> impl Strategy<Value = String> {
    "\\p{Greek}{1,256}"
}

#[derive(Debug)]
struct HelloStringExercise {
    test_word: String,
    string_ptr_loc: MemArea,
    string_len_loc: MemArea,
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
                    HostMemory::mem_area_strat(4),
                )
            })
            .prop_map(
                |(test_word, string_ptr_loc, string_len_loc, return_ptr_loc)| Self {
                    test_word,
                    string_ptr_loc,
                    string_len_loc,
                    return_ptr_loc,
                },
            )
            .prop_filter("non-overlapping pointers", |e| {
                MemArea::non_overlapping_set(&[
                    &e.string_ptr_loc,
                    &e.string_len_loc,
                    &e.return_ptr_loc,
                ])
            })
            .boxed()
    }

    pub fn test(&self) {
        let mut ctx = WasiCtx::new();
        let mut host_memory = HostMemory::new();
        let mut guest_memory = host_memory.guest_memory();

        // Populate string length
        *guest_memory
            .ptr_mut(self.string_len_loc.ptr)
            .expect("ptr mut to string len")
            .as_ref_mut()
            .expect("deref ptr mut to string len") = self.test_word.len() as u32;

        // Populate string in guest's memory
        {
            let mut next: GuestPtrMut<'_, u8> = guest_memory
                .ptr_mut(self.string_ptr_loc.ptr)
                .expect("ptr mut to the first byte of string");
            for byte in self.test_word.as_bytes() {
                *next.as_ref_mut().expect("deref mut") = *byte;
                next = next.elem(1).expect("increment ptr by 1");
            }
        }

        let res = strings::hello_string(
            &mut ctx,
            &mut guest_memory,
            self.string_ptr_loc.ptr as i32,
            self.string_len_loc.ptr as i32,
            self.return_ptr_loc.ptr as i32,
        );
        assert_eq!(res, types::Errno::Ok.into(), "hello string errno");

        let given = *guest_memory
            .ptr::<u32>(self.return_ptr_loc.ptr)
            .expect("ptr to return value")
            .as_ref()
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
