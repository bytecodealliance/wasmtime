use proptest::prelude::*;
use wiggle::{GuestMemory, GuestPtr, GuestType};
use wiggle_test::{impl_errno, HostMemory, MemArea, MemAreas, WasiCtx};

wiggle::from_witx!({
    witx: ["$CARGO_MANIFEST_DIR/tests/arrays.witx"],
    ctx: WasiCtx,
});

impl_errno!(types::Errno, types::GuestErrorConversion);

impl<'a> arrays::Arrays for WasiCtx<'a> {
    fn reduce_excuses(
        &self,
        excuses: &types::ConstExcuseArray,
    ) -> Result<types::Excuse, types::Errno> {
        let last = &excuses
            .iter()
            .last()
            .expect("input array is non-empty")
            .expect("valid ptr to ptr")
            .read()
            .expect("valid ptr to some Excuse value");
        Ok(last.read().expect("dereferencing ptr should succeed"))
    }

    fn populate_excuses(&self, excuses: &types::ExcuseArray) -> Result<(), types::Errno> {
        for excuse in excuses.iter() {
            let ptr_to_excuse = excuse
                .expect("valid ptr to ptr")
                .read()
                .expect("valid ptr to some Excuse value");
            ptr_to_excuse
                .write(types::Excuse::Sleeping)
                .expect("dereferencing mut ptr should succeed");
        }
        Ok(())
    }
}

#[derive(Debug)]
struct ReduceExcusesExcercise {
    excuse_values: Vec<types::Excuse>,
    excuse_ptr_locs: Vec<MemArea>,
    array_ptr_loc: MemArea,
    return_ptr_loc: MemArea,
}

impl ReduceExcusesExcercise {
    pub fn strat() -> BoxedStrategy<Self> {
        (1..256u32)
            .prop_flat_map(|len| {
                let len_usize = len as usize;
                (
                    proptest::collection::vec(excuse_strat(), len_usize..=len_usize),
                    proptest::collection::vec(HostMemory::mem_area_strat(4), len_usize..=len_usize),
                    HostMemory::mem_area_strat(4 * len),
                    HostMemory::mem_area_strat(4),
                )
            })
            .prop_map(
                |(excuse_values, excuse_ptr_locs, array_ptr_loc, return_ptr_loc)| Self {
                    excuse_values,
                    excuse_ptr_locs,
                    array_ptr_loc,
                    return_ptr_loc,
                },
            )
            .prop_filter("non-overlapping pointers", |e| {
                let mut all = vec![e.array_ptr_loc, e.return_ptr_loc];
                all.extend(e.excuse_ptr_locs.iter());
                MemArea::non_overlapping_set(all)
            })
            .boxed()
    }

    pub fn test(&self) {
        let ctx = WasiCtx::new();
        let host_memory = HostMemory::new();

        // Populate memory with pointers to generated Excuse values
        for (&excuse, ptr) in self.excuse_values.iter().zip(self.excuse_ptr_locs.iter()) {
            host_memory
                .ptr(ptr.ptr)
                .write(excuse)
                .expect("deref ptr mut to Excuse value");
        }

        // Populate the array with pointers to generated Excuse values
        {
            let array: GuestPtr<'_, [GuestPtr<types::Excuse>]> =
                host_memory.ptr((self.array_ptr_loc.ptr, self.excuse_ptr_locs.len() as u32));
            for (slot, ptr) in array.iter().zip(&self.excuse_ptr_locs) {
                let slot = slot.expect("array should be in bounds");
                slot.write(host_memory.ptr(ptr.ptr))
                    .expect("should succeed in writing array");
            }
        }

        let res = arrays::reduce_excuses(
            &ctx,
            &host_memory,
            self.array_ptr_loc.ptr as i32,
            self.excuse_ptr_locs.len() as i32,
            self.return_ptr_loc.ptr as i32,
        );

        assert_eq!(res, types::Errno::Ok.into(), "reduce excuses errno");

        let expected = *self
            .excuse_values
            .last()
            .expect("generated vec of excuses should be non-empty");
        let given: types::Excuse = host_memory
            .ptr(self.return_ptr_loc.ptr)
            .read()
            .expect("deref ptr to returned value");
        assert_eq!(expected, given, "reduce excuses return val");
    }
}
proptest! {
    #[test]
    fn reduce_excuses(e in ReduceExcusesExcercise::strat()) {
        e.test()
    }
}

fn excuse_strat() -> impl Strategy<Value = types::Excuse> {
    prop_oneof![
        Just(types::Excuse::DogAte),
        Just(types::Excuse::Traffic),
        Just(types::Excuse::Sleeping),
    ]
    .boxed()
}

#[derive(Debug)]
struct PopulateExcusesExcercise {
    array_ptr_loc: MemArea,
    elements: Vec<MemArea>,
}

impl PopulateExcusesExcercise {
    pub fn strat() -> BoxedStrategy<Self> {
        (1..256u32)
            .prop_flat_map(|len| {
                let len_usize = len as usize;
                (
                    HostMemory::mem_area_strat(4 * len),
                    proptest::collection::vec(HostMemory::mem_area_strat(4), len_usize..=len_usize),
                )
            })
            .prop_map(|(array_ptr_loc, elements)| Self {
                array_ptr_loc,
                elements,
            })
            .prop_filter("non-overlapping pointers", |e| {
                let mut all = vec![e.array_ptr_loc];
                all.extend(e.elements.iter());
                MemArea::non_overlapping_set(all)
            })
            .boxed()
    }

    pub fn test(&self) {
        let ctx = WasiCtx::new();
        let host_memory = HostMemory::new();

        // Populate array with valid pointers to Excuse type in memory
        let ptr = host_memory.ptr::<[GuestPtr<'_, types::Excuse>]>((
            self.array_ptr_loc.ptr,
            self.elements.len() as u32,
        ));
        for (ptr, val) in ptr.iter().zip(&self.elements) {
            ptr.expect("should be valid pointer")
                .write(host_memory.ptr(val.ptr))
                .expect("failed to write value");
        }

        let res = arrays::populate_excuses(
            &ctx,
            &host_memory,
            self.array_ptr_loc.ptr as i32,
            self.elements.len() as i32,
        );
        assert_eq!(res, types::Errno::Ok.into(), "populate excuses errno");

        let arr: GuestPtr<'_, [GuestPtr<'_, types::Excuse>]> =
            host_memory.ptr((self.array_ptr_loc.ptr, self.elements.len() as u32));
        for el in arr.iter() {
            let ptr_to_ptr = el
                .expect("valid ptr to ptr")
                .read()
                .expect("valid ptr to some Excuse value");
            assert_eq!(
                ptr_to_ptr
                    .read()
                    .expect("dereferencing ptr to some Excuse value"),
                types::Excuse::Sleeping,
                "element should equal Excuse::Sleeping"
            );
        }
    }
}
proptest! {
    #[test]
    fn populate_excuses(e in PopulateExcusesExcercise::strat()) {
        e.test()
    }
}

impl<'a> array_traversal::ArrayTraversal for WasiCtx<'a> {
    fn sum_of_element(
        &self,
        elements: &GuestPtr<[types::PairInts]>,
        index: u32,
    ) -> Result<i32, types::Errno> {
        let elem_ptr = elements.get(index).ok_or(types::Errno::InvalidArg)?;
        let pair = elem_ptr.read().map_err(|_| types::Errno::DontWantTo)?;
        Ok(pair.first.wrapping_add(pair.second))
    }
    fn sum_of_elements(
        &self,
        elements: &GuestPtr<[types::PairInts]>,
        start: u32,
        end: u32,
    ) -> Result<i32, types::Errno> {
        let elem_range = elements
            .get_range(start..end)
            .ok_or(types::Errno::InvalidArg)?;
        let mut sum: i32 = 0;
        for e in elem_range.iter() {
            let pair = e
                .map_err(|_| types::Errno::DontWantTo)?
                .read()
                .map_err(|_| types::Errno::PhysicallyUnable)?;
            sum = sum.wrapping_add(pair.first).wrapping_add(pair.second);
        }
        Ok(sum)
    }
}

impl types::PairInts {
    pub fn strat() -> BoxedStrategy<Self> {
        (prop::num::i32::ANY, prop::num::i32::ANY)
            .prop_map(|(first, second)| types::PairInts { first, second })
            .boxed()
    }
}

#[derive(Debug)]
struct SumElementsExercise {
    elements: Vec<types::PairInts>,
    element_loc: MemArea,
    return_loc: MemArea,
    start_ix: u32,
    end_ix: u32,
}

impl SumElementsExercise {
    pub fn strat() -> BoxedStrategy<Self> {
        (
            prop::collection::vec(types::PairInts::strat(), 1..256),
            HostMemory::mem_area_strat(4),
        )
            .prop_flat_map(|(elements, return_loc)| {
                let len = elements.len() as u32;
                (
                    Just(elements),
                    HostMemory::byte_slice_strat(
                        len * types::PairInts::guest_size(),
                        types::PairInts::guest_size(),
                        &MemAreas::from([return_loc]),
                    ),
                    Just(return_loc),
                    0..len,
                    0..len,
                )
            })
            .prop_map(
                |(elements, element_loc, return_loc, start_ix, end_ix)| SumElementsExercise {
                    elements,
                    element_loc,
                    return_loc,
                    start_ix,
                    end_ix,
                },
            )
            .boxed()
    }
    pub fn test(&self) {
        let ctx = WasiCtx::new();
        let host_memory = HostMemory::new();

        // Populate array
        let ptr = host_memory
            .ptr::<[types::PairInts]>((self.element_loc.ptr, self.elements.len() as u32));
        for (ptr, val) in ptr.iter().zip(&self.elements) {
            ptr.expect("should be valid pointer")
                .write(val.clone())
                .expect("failed to write value");
        }

        let res = array_traversal::sum_of_element(
            &ctx,
            &host_memory,
            self.element_loc.ptr as i32,
            self.elements.len() as i32,
            self.start_ix as i32,
            self.return_loc.ptr as i32,
        );
        assert_eq!(res, types::Errno::Ok.into(), "sum_of_element errno");
        let result_ptr = host_memory.ptr::<i32>(self.return_loc.ptr);
        let result = result_ptr.read().expect("read result");

        let e = self
            .elements
            .get(self.start_ix as usize)
            .expect("start_ix must be in bounds");
        assert_eq!(result, e.first.wrapping_add(e.second), "sum of element");

        // Off the end of the array:
        let res = array_traversal::sum_of_element(
            &ctx,
            &host_memory,
            self.element_loc.ptr as i32,
            self.elements.len() as i32,
            self.elements.len() as i32,
            self.return_loc.ptr as i32,
        );
        assert_eq!(
            res,
            types::Errno::InvalidArg.into(),
            "out of bounds sum_of_element errno"
        );

        let res = array_traversal::sum_of_elements(
            &ctx,
            &host_memory,
            self.element_loc.ptr as i32,
            self.elements.len() as i32,
            self.start_ix as i32,
            self.end_ix as i32,
            self.return_loc.ptr as i32,
        );
        if self.start_ix <= self.end_ix {
            assert_eq!(
                res,
                types::Errno::Ok.into(),
                "expected ok sum_of_elements errno"
            );
            let result_ptr = host_memory.ptr::<i32>(self.return_loc.ptr);
            let result = result_ptr.read().expect("read result");

            let mut expected_sum: i32 = 0;
            for elem in self
                .elements
                .get(self.start_ix as usize..self.end_ix as usize)
                .unwrap()
                .iter()
            {
                expected_sum = expected_sum
                    .wrapping_add(elem.first)
                    .wrapping_add(elem.second);
            }
            assert_eq!(result, expected_sum, "sum of elements");
        } else {
            assert_eq!(
                res,
                types::Errno::InvalidArg.into(),
                "expected error out-of-bounds sum_of_elements"
            );
        }

        // Index an array off the end of the array:
        let res = array_traversal::sum_of_elements(
            &ctx,
            &host_memory,
            self.element_loc.ptr as i32,
            self.elements.len() as i32,
            self.start_ix as i32,
            self.elements.len() as i32 + 1,
            self.return_loc.ptr as i32,
        );
        assert_eq!(
            res,
            types::Errno::InvalidArg.into(),
            "out of bounds sum_of_elements errno"
        );
    }
}
proptest! {
    #[test]
    fn sum_elements(e in SumElementsExercise::strat()) {
        e.test()
    }
}
