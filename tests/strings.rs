use proptest::prelude::*;
use wiggle_runtime::{GuestError, GuestMemory, GuestPtr};
use wiggle_test::{impl_errno, HostMemory, MemArea, WasiCtx};

wiggle::from_witx!({
    witx: ["tests/strings.witx"],
    ctx: WasiCtx,
});

impl_errno!(types::Errno);

impl strings::Strings for WasiCtx {
    fn hello_string(&self, a_string: &GuestPtr<str>) -> Result<u32, types::Errno> {
        let s = a_string.as_raw().expect("should be valid string");
        unsafe {
            println!("a_string='{}'", &*s);
            Ok((*s).len() as u32)
        }
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
                MemArea::non_overlapping_set(&[&e.string_ptr_loc, &e.return_ptr_loc])
            })
            .boxed()
    }

    pub fn test(&self) {
        let ctx = WasiCtx::new();
        let host_memory = HostMemory::new();

        // Populate string in guest's memory
        let ptr = host_memory.ptr::<str>((self.string_ptr_loc.ptr, self.test_word.len() as u32));
        for (slot, byte) in ptr.as_bytes().iter().zip(self.test_word.bytes()) {
            slot.expect("should be valid pointer")
                .write(byte)
                .expect("failed to write");
        }

        let res = strings::hello_string(
            &ctx,
            &host_memory,
            self.string_ptr_loc.ptr as i32,
            self.test_word.len() as i32,
            self.return_ptr_loc.ptr as i32,
        );
        assert_eq!(res, types::Errno::Ok.into(), "hello string errno");

        let given = host_memory
            .ptr::<u32>(self.return_ptr_loc.ptr)
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
