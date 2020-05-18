use proptest::prelude::*;
use wiggle::{BorrowChecker, GuestMemory, GuestPtr};
use wiggle_test::{impl_errno, HostMemory, MemArea, WasiCtx};

wiggle::from_witx!({
    witx: ["tests/structs.witx"],
    ctx: WasiCtx,
});

impl_errno!(types::Errno, types::GuestErrorConversion);

impl<'a> structs::Structs for WasiCtx<'a> {
    fn sum_of_pair(&self, an_pair: &types::PairInts) -> Result<i64, types::Errno> {
        Ok(an_pair.first as i64 + an_pair.second as i64)
    }

    fn sum_of_pair_of_ptrs(&self, an_pair: &types::PairIntPtrs) -> Result<i64, types::Errno> {
        let first = an_pair
            .first
            .read()
            .expect("dereferencing GuestPtr should succeed");
        let second = an_pair
            .second
            .read()
            .expect("dereferncing GuestPtr should succeed");
        Ok(first as i64 + second as i64)
    }

    fn sum_of_int_and_ptr(&self, an_pair: &types::PairIntAndPtr) -> Result<i64, types::Errno> {
        let first = an_pair
            .first
            .read()
            .expect("dereferencing GuestPtr should succeed");
        let second = an_pair.second as i64;
        Ok(first as i64 + second)
    }

    fn return_pair_ints(&self) -> Result<types::PairInts, types::Errno> {
        Ok(types::PairInts {
            first: 10,
            second: 20,
        })
    }

    fn return_pair_of_ptrs<'b>(
        &self,
        first: &GuestPtr<'b, i32>,
        second: &GuestPtr<'b, i32>,
    ) -> Result<types::PairIntPtrs<'b>, types::Errno> {
        Ok(types::PairIntPtrs {
            first: *first,
            second: *second,
        })
    }
}

#[derive(Debug)]
struct SumOfPairExercise {
    pub input: types::PairInts,
    pub input_loc: MemArea,
    pub return_loc: MemArea,
}

impl SumOfPairExercise {
    pub fn strat() -> BoxedStrategy<Self> {
        (
            prop::num::i32::ANY,
            prop::num::i32::ANY,
            HostMemory::mem_area_strat(8),
            HostMemory::mem_area_strat(8),
        )
            .prop_map(|(first, second, input_loc, return_loc)| SumOfPairExercise {
                input: types::PairInts { first, second },
                input_loc,
                return_loc,
            })
            .prop_filter("non-overlapping pointers", |e| {
                MemArea::non_overlapping_set(&[e.input_loc, e.return_loc])
            })
            .boxed()
    }

    pub fn test(&self) {
        let ctx = WasiCtx::new();
        let host_memory = HostMemory::new();
        let bc = BorrowChecker::new();

        host_memory
            .ptr(&bc, self.input_loc.ptr)
            .write(self.input.first)
            .expect("input ref_mut");
        host_memory
            .ptr(&bc, self.input_loc.ptr + 4)
            .write(self.input.second)
            .expect("input ref_mut");
        let sum_err = structs::sum_of_pair(
            &ctx,
            &host_memory,
            &bc,
            self.input_loc.ptr as i32,
            self.return_loc.ptr as i32,
        );

        assert_eq!(sum_err, types::Errno::Ok.into(), "sum errno");

        let return_val: i64 = host_memory
            .ptr(&bc, self.return_loc.ptr)
            .read()
            .expect("return ref");

        assert_eq!(
            return_val,
            self.input.first as i64 + self.input.second as i64,
            "sum return value"
        );
    }
}

proptest! {
    #[test]
    fn sum_of_pair(e in SumOfPairExercise::strat()) {
        e.test();
    }
}

#[derive(Debug)]
struct SumPairPtrsExercise {
    input_first: i32,
    input_second: i32,
    input_first_loc: MemArea,
    input_second_loc: MemArea,
    input_struct_loc: MemArea,
    return_loc: MemArea,
}

impl SumPairPtrsExercise {
    pub fn strat() -> BoxedStrategy<Self> {
        (
            prop::num::i32::ANY,
            prop::num::i32::ANY,
            HostMemory::mem_area_strat(4),
            HostMemory::mem_area_strat(4),
            HostMemory::mem_area_strat(8),
            HostMemory::mem_area_strat(8),
        )
            .prop_map(
                |(
                    input_first,
                    input_second,
                    input_first_loc,
                    input_second_loc,
                    input_struct_loc,
                    return_loc,
                )| SumPairPtrsExercise {
                    input_first,
                    input_second,
                    input_first_loc,
                    input_second_loc,
                    input_struct_loc,
                    return_loc,
                },
            )
            .prop_filter("non-overlapping pointers", |e| {
                MemArea::non_overlapping_set(&[
                    e.input_first_loc,
                    e.input_second_loc,
                    e.input_struct_loc,
                    e.return_loc,
                ])
            })
            .boxed()
    }
    pub fn test(&self) {
        let ctx = WasiCtx::new();
        let host_memory = HostMemory::new();
        let bc = BorrowChecker::new();

        host_memory
            .ptr(&bc, self.input_first_loc.ptr)
            .write(self.input_first)
            .expect("input_first ref");
        host_memory
            .ptr(&bc, self.input_second_loc.ptr)
            .write(self.input_second)
            .expect("input_second ref");

        host_memory
            .ptr(&bc, self.input_struct_loc.ptr)
            .write(self.input_first_loc.ptr)
            .expect("input_struct ref");
        host_memory
            .ptr(&bc, self.input_struct_loc.ptr + 4)
            .write(self.input_second_loc.ptr)
            .expect("input_struct ref");

        let res = structs::sum_of_pair_of_ptrs(
            &ctx,
            &host_memory,
            &bc,
            self.input_struct_loc.ptr as i32,
            self.return_loc.ptr as i32,
        );

        assert_eq!(res, types::Errno::Ok.into(), "sum of pair of ptrs errno");

        let doubled: i64 = host_memory
            .ptr(&bc, self.return_loc.ptr)
            .read()
            .expect("return ref");

        assert_eq!(
            doubled,
            (self.input_first as i64) + (self.input_second as i64),
            "sum of pair of ptrs return val"
        );
    }
}
proptest! {
    #[test]
    fn sum_of_pair_of_ptrs(e in SumPairPtrsExercise::strat()) {
        e.test()
    }
}

#[derive(Debug)]
struct SumIntAndPtrExercise {
    input_first: i32,
    input_second: i32,
    input_first_loc: MemArea,
    input_struct_loc: MemArea,
    return_loc: MemArea,
}

impl SumIntAndPtrExercise {
    pub fn strat() -> BoxedStrategy<Self> {
        (
            prop::num::i32::ANY,
            prop::num::i32::ANY,
            HostMemory::mem_area_strat(4),
            HostMemory::mem_area_strat(8),
            HostMemory::mem_area_strat(8),
        )
            .prop_map(
                |(input_first, input_second, input_first_loc, input_struct_loc, return_loc)| {
                    SumIntAndPtrExercise {
                        input_first,
                        input_second,
                        input_first_loc,
                        input_struct_loc,
                        return_loc,
                    }
                },
            )
            .prop_filter("non-overlapping pointers", |e| {
                MemArea::non_overlapping_set(&[e.input_first_loc, e.input_struct_loc, e.return_loc])
            })
            .boxed()
    }
    pub fn test(&self) {
        let ctx = WasiCtx::new();
        let host_memory = HostMemory::new();
        let bc = BorrowChecker::new();

        host_memory
            .ptr(&bc, self.input_first_loc.ptr)
            .write(self.input_first)
            .expect("input_first ref");
        host_memory
            .ptr(&bc, self.input_struct_loc.ptr)
            .write(self.input_first_loc.ptr)
            .expect("input_struct ref");
        host_memory
            .ptr(&bc, self.input_struct_loc.ptr + 4)
            .write(self.input_second)
            .expect("input_struct ref");

        let res = structs::sum_of_int_and_ptr(
            &ctx,
            &host_memory,
            &bc,
            self.input_struct_loc.ptr as i32,
            self.return_loc.ptr as i32,
        );

        assert_eq!(res, types::Errno::Ok.into(), "sum of int and ptr errno");

        let doubled: i64 = host_memory
            .ptr(&bc, self.return_loc.ptr)
            .read()
            .expect("return ref");

        assert_eq!(
            doubled,
            (self.input_first as i64) + (self.input_second as i64),
            "sum of pair of ptrs return val"
        );
    }
}
proptest! {
    #[test]
    fn sum_of_int_and_ptr(e in SumIntAndPtrExercise::strat()) {
        e.test()
    }
}

#[derive(Debug)]
struct ReturnPairInts {
    pub return_loc: MemArea,
}

impl ReturnPairInts {
    pub fn strat() -> BoxedStrategy<Self> {
        HostMemory::mem_area_strat(8)
            .prop_map(|return_loc| ReturnPairInts { return_loc })
            .boxed()
    }

    pub fn test(&self) {
        let ctx = WasiCtx::new();
        let host_memory = HostMemory::new();
        let bc = BorrowChecker::new();

        let err = structs::return_pair_ints(&ctx, &host_memory, &bc, self.return_loc.ptr as i32);

        assert_eq!(err, types::Errno::Ok.into(), "return struct errno");

        let return_struct: types::PairInts = host_memory
            .ptr(&bc, self.return_loc.ptr)
            .read()
            .expect("return ref");

        assert_eq!(
            return_struct,
            types::PairInts {
                first: 10,
                second: 20
            },
            "return_pair_ints return value"
        );
    }
}

proptest! {
    #[test]
    fn return_pair_ints(e in ReturnPairInts::strat()) {
        e.test();
    }
}

#[derive(Debug)]
struct ReturnPairPtrsExercise {
    input_first: i32,
    input_second: i32,
    input_first_loc: MemArea,
    input_second_loc: MemArea,
    return_loc: MemArea,
}

impl ReturnPairPtrsExercise {
    pub fn strat() -> BoxedStrategy<Self> {
        (
            prop::num::i32::ANY,
            prop::num::i32::ANY,
            HostMemory::mem_area_strat(4),
            HostMemory::mem_area_strat(4),
            HostMemory::mem_area_strat(8),
        )
            .prop_map(
                |(input_first, input_second, input_first_loc, input_second_loc, return_loc)| {
                    ReturnPairPtrsExercise {
                        input_first,
                        input_second,
                        input_first_loc,
                        input_second_loc,
                        return_loc,
                    }
                },
            )
            .prop_filter("non-overlapping pointers", |e| {
                MemArea::non_overlapping_set(&[e.input_first_loc, e.input_second_loc, e.return_loc])
            })
            .boxed()
    }
    pub fn test(&self) {
        let ctx = WasiCtx::new();
        let host_memory = HostMemory::new();
        let bc = BorrowChecker::new();

        host_memory
            .ptr(&bc, self.input_first_loc.ptr)
            .write(self.input_first)
            .expect("input_first ref");
        host_memory
            .ptr(&bc, self.input_second_loc.ptr)
            .write(self.input_second)
            .expect("input_second ref");

        let res = structs::return_pair_of_ptrs(
            &ctx,
            &host_memory,
            &bc,
            self.input_first_loc.ptr as i32,
            self.input_second_loc.ptr as i32,
            self.return_loc.ptr as i32,
        );

        assert_eq!(res, types::Errno::Ok.into(), "return pair of ptrs errno");

        let ptr_pair_int_ptrs: types::PairIntPtrs<'_> = host_memory
            .ptr(&bc, self.return_loc.ptr)
            .read()
            .expect("failed to read return location");
        let ret_first_ptr = ptr_pair_int_ptrs.first;
        let ret_second_ptr = ptr_pair_int_ptrs.second;
        assert_eq!(
            self.input_first,
            ret_first_ptr
                .read()
                .expect("deref extracted ptr to first element")
        );
        assert_eq!(
            self.input_second,
            ret_second_ptr
                .read()
                .expect("deref extracted ptr to second element")
        );
    }
}
proptest! {
    #[test]
    fn return_pair_of_ptrs(e in ReturnPairPtrsExercise::strat()) {
        e.test()
    }
}
