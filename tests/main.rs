use memory::GuestRef;
use proptest::prelude::*;

generate::from_witx!({
    witx: ["tests/test.witx"],
    ctx: WasiCtx,
});

pub struct WasiCtx {
    guest_errors: Vec<::memory::GuestError>,
}

impl WasiCtx {
    pub fn new() -> Self {
        Self {
            guest_errors: vec![],
        }
    }
}

impl foo::Foo for WasiCtx {
    fn bar(&mut self, an_int: u32, an_float: f32) -> Result<(), types::Errno> {
        println!("BAR: {} {}", an_int, an_float);
        Ok(())
    }

    fn baz(
        &mut self,
        input1: types::Excuse,
        input2_ptr: ::memory::GuestPtrMut<types::Excuse>,
        input3_ptr: ::memory::GuestPtr<types::Excuse>,
        input4_ptr_ptr: ::memory::GuestPtrMut<::memory::GuestPtr<types::Excuse>>,
    ) -> Result<(), types::Errno> {
        println!("BAZ input1 {:?}", input1);
        // Read enum value from mutable:
        let mut input2_ref: ::memory::GuestRefMut<types::Excuse> =
            input2_ptr.as_ref_mut().map_err(|e| {
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
        let input4_ptr: ::memory::GuestPtr<types::Excuse> =
            input4_ptr_ptr.read_ptr_from_guest().map_err(|e| {
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
        input4_ptr_ptr.write_ptr_to_guest(&input2_ptr.as_immut());

        Ok(())
    }

    fn bat(&mut self, an_int: u32) -> Result<f32, types::Errno> {
        Ok((an_int as f32) * 2.0)
    }

    fn sum_of_pair(&mut self, an_pair: &types::PairInts) -> Result<i64, types::Errno> {
        Ok(an_pair.first as i64 + an_pair.second as i64)
    }

    fn sum_of_pair_of_ptrs(&mut self, an_pair: &types::PairIntPtrs) -> Result<i64, types::Errno> {
        let first = *an_pair
            .first
            .as_ref()
            .expect("dereferencing GuestPtr should succeed");
        let second = *an_pair
            .second
            .as_ref()
            .expect("dereferncing GuestPtr should succeed");
        Ok(first as i64 + second as i64)
    }
}
// Errno is used as a first return value in the functions above, therefore
// it must implement GuestErrorType with type Context = WasiCtx.
// The context type should let you do logging or debugging or whatever you need
// with these errors. We just push them to vecs.
impl ::memory::GuestErrorType for types::Errno {
    type Context = WasiCtx;
    fn success() -> types::Errno {
        types::Errno::Ok
    }
    fn from_error(e: ::memory::GuestError, ctx: &mut WasiCtx) -> types::Errno {
        eprintln!("GUEST ERROR: {:?}", e);
        ctx.guest_errors.push(e);
        types::Errno::InvalidArg
    }
}

#[repr(align(4096))]
struct HostMemory {
    buffer: [u8; 4096],
}
impl HostMemory {
    pub fn new() -> Self {
        HostMemory { buffer: [0; 4096] }
    }
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.buffer.as_mut_ptr()
    }
    pub fn len(&self) -> usize {
        self.buffer.len()
    }
    pub fn mem_area_strat(align: u32) -> BoxedStrategy<MemArea> {
        prop::num::u32::ANY
            .prop_map(move |p| {
                let p_aligned = p - (p % align); // Align according to argument
                let ptr = p_aligned % 4096; // Put inside memory
                MemArea { ptr, len: align }
            })
            .boxed()
    }
}

#[derive(Debug)]
struct MemArea {
    ptr: u32,
    len: u32,
}

// This code is a whole lot like the Region::overlaps func thats at the core of the code under
// test.
// So, I implemented this one with std::ops::Range so it is less likely I wrote the same bug in two
// places.
fn overlapping(a: &MemArea, b: &MemArea) -> bool {
    // a_range is all elems in A
    let a_range = std::ops::Range {
        start: a.ptr,
        end: a.ptr + a.len - 1,
    };
    // b_range is all elems in B
    let b_range = std::ops::Range {
        start: b.ptr,
        end: b.ptr + b.len - 1,
    };
    // No element in B is contained in A:
    for b_elem in b_range.clone() {
        if a_range.contains(&b_elem) {
            return true;
        }
    }
    // No element in A is contained in B:
    for a_elem in a_range {
        if b_range.contains(&a_elem) {
            return true;
        }
    }
    return false;
}

fn non_overlapping_set(areas: &[&MemArea]) -> bool {
    // A is all areas
    for (i, a) in areas.iter().enumerate() {
        // (A, B) is every pair of areas
        for b in areas[i + 1..].iter() {
            if overlapping(a, b) {
                return false;
            }
        }
    }
    return true;
}

#[test]
fn hostmemory_is_aligned() {
    let mut h = HostMemory::new();
    assert_eq!(h.as_mut_ptr() as usize % 4096, 0);
    let mut h = Box::new(HostMemory::new());
    assert_eq!(h.as_mut_ptr() as usize % 4096, 0);
}

#[derive(Debug)]
struct BatExercise {
    pub input: u32,
    pub return_loc: MemArea,
}

impl BatExercise {
    pub fn test(&self) {
        let mut ctx = WasiCtx::new();
        let mut host_memory = HostMemory::new();
        let mut guest_memory =
            memory::GuestMemory::new(host_memory.as_mut_ptr(), host_memory.len() as u32);

        let bat_err = foo::bat(
            &mut ctx,
            &mut guest_memory,
            self.input as i32,
            self.return_loc.ptr as i32,
        );

        let return_val: GuestRef<f32> = guest_memory
            .ptr(self.return_loc.ptr)
            .expect("return loc ptr")
            .as_ref()
            .expect("return val ref");
        assert_eq!(bat_err, types::Errno::Ok.into(), "bat errno");
        assert_eq!(*return_val, (self.input as f32) * 2.0, "bat return val");
    }

    pub fn strat() -> BoxedStrategy<Self> {
        (prop::num::u32::ANY, HostMemory::mem_area_strat(4))
            .prop_map(|(input, return_loc)| BatExercise { input, return_loc })
            .boxed()
    }
}

proptest! {
    #[test]
    fn bat(e in BatExercise::strat()) {
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
struct BazExercise {
    pub input1: types::Excuse,
    pub input2: types::Excuse,
    pub input2_loc: MemArea,
    pub input3: types::Excuse,
    pub input3_loc: MemArea,
    pub input4: types::Excuse,
    pub input4_loc: MemArea,
    pub input4_ptr_loc: MemArea,
}

impl BazExercise {
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
                )| BazExercise {
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
                non_overlapping_set(&[
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
        let mut guest_memory =
            memory::GuestMemory::new(host_memory.as_mut_ptr(), host_memory.len() as u32);

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

        let baz_err = foo::baz(
            &mut ctx,
            &mut guest_memory,
            self.input1.into(),
            self.input2_loc.ptr as i32,
            self.input3_loc.ptr as i32,
            self.input4_ptr_loc.ptr as i32,
        );
        assert_eq!(baz_err, types::Errno::Ok.into(), "baz errno");

        // Implementation of baz writes input3 to the input2_loc:
        let written_to_input2_loc: i32 = *guest_memory
            .ptr(self.input2_loc.ptr)
            .expect("input2 ptr")
            .as_ref()
            .expect("input2 ref");

        assert_eq!(
            written_to_input2_loc,
            self.input3.into(),
            "baz written to input2"
        );

        // Implementation of baz writes input2_loc to input4_ptr_loc:
        let written_to_input4_ptr: u32 = *guest_memory
            .ptr(self.input4_ptr_loc.ptr)
            .expect("input4_ptr_loc ptr")
            .as_ref()
            .expect("input4_ptr_loc ref");

        assert_eq!(
            written_to_input4_ptr, self.input2_loc.ptr,
            "baz written to input4_ptr"
        );
    }
}
proptest! {
    #[test]
    fn baz(e in BazExercise::strat()) {
        e.test();
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
                non_overlapping_set(&[&e.input_loc, &e.return_loc])
            })
            .boxed()
    }

    pub fn test(&self) {
        let mut ctx = WasiCtx::new();
        let mut host_memory = HostMemory::new();
        let mut guest_memory =
            memory::GuestMemory::new(host_memory.as_mut_ptr(), host_memory.len() as u32);

        *guest_memory
            .ptr_mut(self.input_loc.ptr)
            .expect("input ptr")
            .as_ref_mut()
            .expect("input ref_mut") = self.input.first;
        *guest_memory
            .ptr_mut(self.input_loc.ptr + 4)
            .expect("input ptr")
            .as_ref_mut()
            .expect("input ref_mut") = self.input.second;
        let sum_err = foo::sum_of_pair(
            &mut ctx,
            &mut guest_memory,
            self.input_loc.ptr as i32,
            self.return_loc.ptr as i32,
        );

        assert_eq!(sum_err, types::Errno::Ok.into(), "sum errno");

        let return_val: i64 = *guest_memory
            .ptr(self.return_loc.ptr)
            .expect("return ptr")
            .as_ref()
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
                non_overlapping_set(&[
                    &e.input_first_loc,
                    &e.input_second_loc,
                    &e.input_struct_loc,
                    &e.return_loc,
                ])
            })
            .boxed()
    }
    pub fn test(&self) {
        let mut ctx = WasiCtx::new();
        let mut host_memory = HostMemory::new();
        let mut guest_memory =
            memory::GuestMemory::new(host_memory.as_mut_ptr(), host_memory.len() as u32);

        *guest_memory
            .ptr_mut(self.input_first_loc.ptr)
            .expect("input_first ptr")
            .as_ref_mut()
            .expect("input_first ref") = self.input_first;
        *guest_memory
            .ptr_mut(self.input_second_loc.ptr)
            .expect("input_second ptr")
            .as_ref_mut()
            .expect("input_second ref") = self.input_second;

        *guest_memory
            .ptr_mut(self.input_struct_loc.ptr)
            .expect("input_struct ptr")
            .as_ref_mut()
            .expect("input_struct ref") = self.input_first_loc.ptr;
        *guest_memory
            .ptr_mut(self.input_struct_loc.ptr + 4)
            .expect("input_struct ptr")
            .as_ref_mut()
            .expect("input_struct ref") = self.input_second_loc.ptr;

        let res = foo::sum_of_pair_of_ptrs(
            &mut ctx,
            &mut guest_memory,
            self.input_struct_loc.ptr as i32,
            self.return_loc.ptr as i32,
        );

        assert_eq!(res, types::Errno::Ok.into(), "sum of pair of ptrs errno");

        let doubled: i64 = *guest_memory
            .ptr(self.return_loc.ptr)
            .expect("return ptr")
            .as_ref()
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
