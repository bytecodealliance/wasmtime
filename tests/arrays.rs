use proptest::prelude::*;
use wiggle_runtime::{GuestError, GuestMemory, GuestPtr};
use wiggle_test::{impl_errno, HostMemory, MemArea, WasiCtx};

wiggle::from_witx!({
    witx: ["tests/arrays.witx"],
    ctx: WasiCtx,
});

impl_errno!(types::Errno);

impl arrays::Arrays for WasiCtx {
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
        let mut ctx = WasiCtx::new();
        let mut host_memory = HostMemory::new();

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
            &mut ctx,
            &mut host_memory,
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
