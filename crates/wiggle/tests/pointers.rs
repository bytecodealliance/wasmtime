use proptest::prelude::*;
use wiggle::{GuestMemory, GuestPtr};
use wiggle_test::{impl_errno, HostMemory, MemArea, WasiCtx};

wiggle::from_witx!({
    witx: ["tests/pointers.witx"],
});

impl_errno!(types::Errno);

impl<'a> pointers::Pointers for WasiCtx<'a> {
    fn pointers_and_enums(
        &mut self,
        memory: &mut GuestMemory<'_>,
        input1: types::Excuse,
        input2_ptr: GuestPtr<types::Excuse>,
        input3_ptr: GuestPtr<types::Excuse>,
        input4_ptr_ptr: GuestPtr<GuestPtr<types::Excuse>>,
    ) -> Result<(), types::Errno> {
        println!("BAZ input1 {input1:?}");
        let input2: types::Excuse = memory.read(input2_ptr).map_err(|e| {
            eprintln!("input2_ptr error: {e}");
            types::Errno::InvalidArg
        })?;
        println!("input2 {input2:?}");

        // Read enum value from immutable ptr:
        let input3 = memory.read(input3_ptr).map_err(|e| {
            eprintln!("input3_ptr error: {e}");
            types::Errno::InvalidArg
        })?;
        println!("input3 {input3:?}");

        // Write enum to mutable ptr:
        memory.write(input2_ptr, input3).map_err(|e| {
            eprintln!("input2_ptr error: {e}");
            types::Errno::InvalidArg
        })?;
        println!("wrote to input2_ref {input3:?}");

        // Read ptr value from mutable ptr:
        let input4_ptr: GuestPtr<types::Excuse> = memory.read(input4_ptr_ptr).map_err(|e| {
            eprintln!("input4_ptr_ptr error: {e}");
            types::Errno::InvalidArg
        })?;

        // Read enum value from that ptr:
        let input4: types::Excuse = memory.read(input4_ptr).map_err(|e| {
            eprintln!("input4_ptr error: {e}");
            types::Errno::InvalidArg
        })?;
        println!("input4 {input4:?}");

        // Write ptr value to mutable ptr:
        memory.write(input4_ptr_ptr, input2_ptr).map_err(|e| {
            eprintln!("input4_ptr_ptr error: {e}");
            types::Errno::InvalidArg
        })?;

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
                    e.input2_loc,
                    e.input3_loc,
                    e.input4_loc,
                    e.input4_ptr_loc,
                ])
            })
            .boxed()
    }
    pub fn test(&self) {
        let mut ctx = WasiCtx::new();
        let mut host_memory = HostMemory::new();
        let mut memory = host_memory.guest_memory();

        memory
            .write(GuestPtr::new(self.input2_loc.ptr), self.input2)
            .expect("input2 ref_mut");

        memory
            .write(GuestPtr::new(self.input3_loc.ptr), self.input3)
            .expect("input3 ref_mut");

        memory
            .write(GuestPtr::new(self.input4_loc.ptr), self.input4)
            .expect("input4 ref_mut");

        memory
            .write(GuestPtr::new(self.input4_ptr_loc.ptr), self.input4_loc.ptr)
            .expect("input4 ptr ref_mut");

        let e = pointers::pointers_and_enums(
            &mut ctx,
            &mut memory,
            self.input1 as i32,
            self.input2_loc.ptr as i32,
            self.input3_loc.ptr as i32,
            self.input4_ptr_loc.ptr as i32,
        )
        .unwrap();
        assert_eq!(e, types::Errno::Ok as i32, "errno");

        // Implementation of pointers_and_enums writes input3 to the input2_loc:
        let written_to_input2_loc: i32 = memory
            .read(GuestPtr::new(self.input2_loc.ptr))
            .expect("input2 ref");

        assert_eq!(
            written_to_input2_loc, self.input3 as i32,
            "pointers_and_enums written to input2"
        );

        // Implementation of pointers_and_enums writes input2_loc to input4_ptr_loc:
        let written_to_input4_ptr: u32 = memory
            .read(GuestPtr::new(self.input4_ptr_loc.ptr))
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
