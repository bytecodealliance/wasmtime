use proptest::prelude::*;
use wiggle_runtime::{GuestArray, GuestError, GuestPtr, GuestPtrMut, GuestType};
use wiggle_test::{impl_errno, HostMemory, MemArea, WasiCtx};

wiggle_generate::from_witx!({
    witx: ["tests/arrays.witx"],
    ctx: WasiCtx,
});

impl_errno!(types::Errno);

impl arrays::Arrays for WasiCtx {
    fn reduce_excuses(
        &mut self,
        excuses: &types::ConstExcuseArray,
    ) -> Result<types::Excuse, types::Errno> {
        let last = GuestType::read(
            &excuses
                .iter()
                .last()
                .expect("input array is non-empty")
                .expect("valid ptr to ptr"),
        )
        .expect("valid ptr to some Excuse value");
        Ok(*last.as_ref().expect("dereferencing ptr should succeed"))
    }

    fn populate_excuses(&mut self, excuses: &types::ExcuseArray) -> Result<(), types::Errno> {
        for excuse in excuses.iter() {
            let ptr_to_ptr = GuestType::read(&excuse.expect("valid ptr to ptr"))
                .expect("valid ptr to some Excuse value");
            let mut ptr = ptr_to_ptr
                .as_ref_mut()
                .expect("dereferencing mut ptr should succeed");
            *ptr = types::Excuse::Sleeping;
        }
        Ok(())
    }
}

#[derive(Debug)]
struct ReduceExcusesExcercise {
    excuse_values: Vec<types::Excuse>,
    excuse_ptr_locs: Vec<MemArea>,
    array_ptr_loc: MemArea,
    array_len_loc: MemArea,
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
                    HostMemory::mem_area_strat(4),
                )
            })
            .prop_map(
                |(excuse_values, excuse_ptr_locs, array_ptr_loc, array_len_loc, return_ptr_loc)| {
                    Self {
                        excuse_values,
                        excuse_ptr_locs,
                        array_ptr_loc,
                        array_len_loc,
                        return_ptr_loc,
                    }
                },
            )
            .prop_filter("non-overlapping pointers", |e| {
                let mut all = vec![&e.array_ptr_loc, &e.array_len_loc, &e.return_ptr_loc];
                all.extend(e.excuse_ptr_locs.iter());
                MemArea::non_overlapping_set(&all)
            })
            .boxed()
    }

    pub fn test(&self) {
        let mut ctx = WasiCtx::new();
        let mut host_memory = HostMemory::new();
        let mut guest_memory = host_memory.guest_memory();

        // Populate memory with pointers to generated Excuse values
        for (&excuse, ptr) in self.excuse_values.iter().zip(self.excuse_ptr_locs.iter()) {
            *guest_memory
                .ptr_mut(ptr.ptr)
                .expect("ptr mut to Excuse value")
                .as_ref_mut()
                .expect("deref ptr mut to Excuse value") = excuse;
        }

        // Populate array length info
        *guest_memory
            .ptr_mut(self.array_len_loc.ptr)
            .expect("ptr to array len")
            .as_ref_mut()
            .expect("deref ptr mut to array len") = self.excuse_ptr_locs.len() as u32;

        // Populate the array with pointers to generated Excuse values
        {
            let mut next: GuestPtrMut<'_, GuestPtr<types::Excuse>> = guest_memory
                .ptr_mut(self.array_ptr_loc.ptr)
                .expect("ptr to array mut");
            for ptr in &self.excuse_ptr_locs {
                next.write(
                    &guest_memory
                        .ptr::<types::Excuse>(ptr.ptr)
                        .expect("ptr to Excuse value"),
                );
                next = next.elem(1).expect("increment ptr by 1");
            }
        }

        let res = arrays::reduce_excuses(
            &mut ctx,
            &mut guest_memory,
            self.array_ptr_loc.ptr as i32,
            self.array_len_loc.ptr as i32,
            self.return_ptr_loc.ptr as i32,
        );

        assert_eq!(res, types::Errno::Ok.into(), "reduce excuses errno");

        let expected = *self
            .excuse_values
            .last()
            .expect("generated vec of excuses should be non-empty");
        let given: types::Excuse = *guest_memory
            .ptr(self.return_ptr_loc.ptr)
            .expect("ptr to returned value")
            .as_ref()
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
    array_len_loc: MemArea,
    elements: Vec<MemArea>,
}

impl PopulateExcusesExcercise {
    pub fn strat() -> BoxedStrategy<Self> {
        (1..256u32)
            .prop_flat_map(|len| {
                let len_usize = len as usize;
                (
                    HostMemory::mem_area_strat(4 * len),
                    HostMemory::mem_area_strat(4),
                    proptest::collection::vec(HostMemory::mem_area_strat(4), len_usize..=len_usize),
                )
            })
            .prop_map(|(array_ptr_loc, array_len_loc, elements)| Self {
                array_ptr_loc,
                array_len_loc,
                elements,
            })
            .prop_filter("non-overlapping pointers", |e| {
                let mut all = vec![&e.array_ptr_loc, &e.array_len_loc];
                all.extend(e.elements.iter());
                MemArea::non_overlapping_set(&all)
            })
            .boxed()
    }

    pub fn test(&self) {
        let mut ctx = WasiCtx::new();
        let mut host_memory = HostMemory::new();
        let mut guest_memory = host_memory.guest_memory();

        // Populate array length info
        *guest_memory
            .ptr_mut(self.array_len_loc.ptr)
            .expect("ptr mut to array len")
            .as_ref_mut()
            .expect("deref ptr mut to array len") = self.elements.len() as u32;

        // Populate array with valid pointers to Excuse type in memory
        {
            let mut next: GuestPtrMut<'_, GuestPtrMut<types::Excuse>> = guest_memory
                .ptr_mut(self.array_ptr_loc.ptr)
                .expect("ptr mut to the first element of array");
            for ptr in &self.elements {
                next.write(
                    &guest_memory
                        .ptr_mut::<types::Excuse>(ptr.ptr)
                        .expect("ptr mut to Excuse value"),
                );
                next = next.elem(1).expect("increment ptr by 1");
            }
        }

        let res = arrays::populate_excuses(
            &mut ctx,
            &mut guest_memory,
            self.array_ptr_loc.ptr as i32,
            self.array_len_loc.ptr as i32,
        );
        assert_eq!(res, types::Errno::Ok.into(), "populate excuses errno");

        let arr: GuestArray<'_, GuestPtr<'_, types::Excuse>> = guest_memory
            .ptr(self.array_ptr_loc.ptr)
            .expect("ptr to the first element of array")
            .array(self.elements.len() as u32)
            .expect("as array");
        for el in arr.iter() {
            let ptr_to_ptr = GuestType::read(&el.expect("valid ptr to ptr"))
                .expect("valid ptr to some Excuse value");
            assert_eq!(
                *ptr_to_ptr
                    .as_ref()
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
