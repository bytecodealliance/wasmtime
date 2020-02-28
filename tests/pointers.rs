use proptest::prelude::*;
use wiggle_runtime::{GuestError, GuestPtr, GuestPtrMut, GuestRefMut, GuestType};
use wiggle_test::{impl_errno, HostMemory, MemArea, WasiCtx};

wiggle::from_witx!({
    witx: ["tests/pointers.witx"],
    ctx: WasiCtx,
});

impl_errno!(types::Errno);

impl pointers::Pointers for WasiCtx {
    fn pointers_and_enums(
        &mut self,
        input1: types::Excuse,
        input2_ptr: GuestPtrMut<types::Excuse>,
        input3_ptr: GuestPtr<types::Excuse>,
        input4_ptr_ptr: GuestPtrMut<GuestPtr<types::Excuse>>,
    ) -> Result<(), types::Errno> {
        println!("BAZ input1 {:?}", input1);
        // Read enum value from mutable:
        let mut input2_ref: GuestRefMut<types::Excuse> = input2_ptr.as_ref_mut().map_err(|e| {
            eprintln!("input2_ptr error: {}", e);
            types::Errno::InvalidArg
        })?;
        let input2: types::Excuse = *input2_ref;
        println!("input2 {:?}", input2);

        // Read enum value from immutable ptr:
        let input3 = *input3_ptr.as_ref().map_err(|e| {
            eprintln!("input3_ptr error: {}", e);
            types::Errno::InvalidArg
        })?;
        println!("input3 {:?}", input3);

        // Write enum to mutable ptr:
        *input2_ref = input3;
        println!("wrote to input2_ref {:?}", input3);

        // Read ptr value from mutable ptr:
        let input4_ptr: GuestPtr<types::Excuse> = GuestType::read(&input4_ptr_ptr.as_immut())
            .map_err(|e| {
                eprintln!("input4_ptr_ptr error: {}", e);
                types::Errno::InvalidArg
            })?;

        // Read enum value from that ptr:
        let input4: types::Excuse = *input4_ptr.as_ref().map_err(|e| {
            eprintln!("input4_ptr error: {}", e);
            types::Errno::InvalidArg
        })?;
        println!("input4 {:?}", input4);

        // Write ptr value to mutable ptr:
        input4_ptr_ptr.write(&input2_ptr.as_immut());

        Ok(())
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
struct PointersAndEnumsExercise {
    pub input1: types::Excuse,
    pub input2: types::Excuse,
    pub input2_loc: MemArea,
    pub input3: types::Excuse,
    pub input3_loc: MemArea,
    pub input4: types::Excuse,
    pub input4_loc: MemArea,
    pub input4_ptr_loc: MemArea,
}

impl PointersAndEnumsExercise {
    pub fn strat() -> BoxedStrategy<Self> {
        (
            excuse_strat(),
            excuse_strat(),
            HostMemory::mem_area_strat(4),
            excuse_strat(),
            HostMemory::mem_area_strat(4),
            excuse_strat(),
            HostMemory::mem_area_strat(4),
            HostMemory::mem_area_strat(4),
        )
            .prop_map(
                |(
                    input1,
                    input2,
                    input2_loc,
                    input3,
                    input3_loc,
                    input4,
                    input4_loc,
                    input4_ptr_loc,
                )| PointersAndEnumsExercise {
                    input1,
                    input2,
                    input2_loc,
                    input3,
                    input3_loc,
                    input4,
                    input4_loc,
                    input4_ptr_loc,
                },
            )
            .prop_filter("non-overlapping pointers", |e| {
                MemArea::non_overlapping_set(&[
                    &e.input2_loc,
                    &e.input3_loc,
                    &e.input4_loc,
                    &e.input4_ptr_loc,
                ])
            })
            .boxed()
    }
    pub fn test(&self) {
        let mut ctx = WasiCtx::new();
        let mut host_memory = HostMemory::new();
        let mut guest_memory = host_memory.guest_memory();

        *guest_memory
            .ptr_mut(self.input2_loc.ptr)
            .expect("input2 ptr")
            .as_ref_mut()
            .expect("input2 ref_mut") = self.input2;

        *guest_memory
            .ptr_mut(self.input3_loc.ptr)
            .expect("input3 ptr")
            .as_ref_mut()
            .expect("input3 ref_mut") = self.input3;

        *guest_memory
            .ptr_mut(self.input4_loc.ptr)
            .expect("input4 ptr")
            .as_ref_mut()
            .expect("input4 ref_mut") = self.input4;

        *guest_memory
            .ptr_mut(self.input4_ptr_loc.ptr)
            .expect("input4 ptr ptr")
            .as_ref_mut()
            .expect("input4 ptr ref_mut") = self.input4_loc.ptr;

        let e = pointers::pointers_and_enums(
            &mut ctx,
            &mut guest_memory,
            self.input1.into(),
            self.input2_loc.ptr as i32,
            self.input3_loc.ptr as i32,
            self.input4_ptr_loc.ptr as i32,
        );
        assert_eq!(e, types::Errno::Ok.into(), "errno");

        // Implementation of pointers_and_enums writes input3 to the input2_loc:
        let written_to_input2_loc: i32 = *guest_memory
            .ptr(self.input2_loc.ptr)
            .expect("input2 ptr")
            .as_ref()
            .expect("input2 ref");

        assert_eq!(
            written_to_input2_loc,
            self.input3.into(),
            "pointers_and_enums written to input2"
        );

        // Implementation of pointers_and_enums writes input2_loc to input4_ptr_loc:
        let written_to_input4_ptr: u32 = *guest_memory
            .ptr(self.input4_ptr_loc.ptr)
            .expect("input4_ptr_loc ptr")
            .as_ref()
            .expect("input4_ptr_loc ref");

        assert_eq!(
            written_to_input4_ptr, self.input2_loc.ptr,
            "pointers_and_enums written to input4_ptr"
        );
    }
}
proptest! {
    #[test]
    fn pointers_and_enums(e in PointersAndEnumsExercise::strat()) {
        e.test();
    }
}
