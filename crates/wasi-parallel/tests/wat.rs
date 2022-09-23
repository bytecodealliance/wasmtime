//! Run the test cases in the `wat` directory.

mod test_case;

use std::convert::TryInto;
use test_case::*;
use wasmtime::{MemoryType, SharedMemory};

#[test]
fn run_example_wat() {
    assert_eq!(exec("tests/wat/example.wat").unwrap(), 0);
}

// Though a bit overkill, it is very convenient when troubleshooting to be able
// to check that we can successfully run a single iteration of a kernel as well.
#[test]
fn run_example_kernel_wat() {
    // Setup the imported memory.
    let engine = default_engine();
    let memory = SharedMemory::new(&engine, MemoryType::shared(1, 1)).unwrap();

    // Execute the kernel once (mimic running the 42nd iteration).
    let mut test_case =
        TestCase::new("tests/wat/example-kernel.wat", engine, Some(memory)).unwrap();
    let _ = test_case
        .invoke("kernel", &[42.into(), 0.into(), 0.into()])
        .unwrap();

    // Check that the first four bytes were stored correctly.
    let first_4_bytes = &test_case.memory_as_slice()[0..4];
    let first_i32 = i32::from_le_bytes(first_4_bytes.try_into().unwrap());
    assert_eq!(first_i32, 43);
}

// Here we check that we can actually run the nstream benchmark. This is also
// available using `cargo bench`, but this checks different parameters.
#[test]
fn run_nstream_wat() {
    const NUM_THREADS: i32 = 24;
    const NUM_ITEMS: i32 = 200000;
    const DEVICE_KIND: i32 = 0x1; // CPU

    // Setup the benchmark.
    let mut test_case = TestCase::new("tests/wat/nstream.wat", default_engine(), None).unwrap();
    let _ = test_case
        .invoke(
            "setup",
            &[NUM_THREADS.into(), NUM_ITEMS.into(), DEVICE_KIND.into()],
        )
        .unwrap();

    // Execute the benchmark.
    let _ = test_case.invoke("execute", &[]).unwrap();

    // Run the finalize step.
    let results = test_case.invoke("finish", &[]).unwrap();
    assert_eq!(results[0].i32().unwrap(), 0);
}

// For troubleshooting, we to check that we can successfully run a single
// iteration of the nstream kernel as well.
#[test]
fn run_nstream_kernel_wat() {
    // Set up the shared memory; nstream operates on memory buffers with
    // specific floating point values that we initialize manually here.
    const NUM_ITERATIONS: usize = 10;
    const BLOCK_SIZE: usize = 10;
    const ITEM_SIZE: usize = std::mem::size_of::<f32>();
    const BUFFER_SIZE: usize = NUM_ITERATIONS * BLOCK_SIZE * ITEM_SIZE;
    let mut memory_image = [0u8; BUFFER_SIZE * 3];
    for i in 0..NUM_ITERATIONS * BLOCK_SIZE {
        let a_i = i * ITEM_SIZE;
        memory_image[a_i..a_i + ITEM_SIZE].copy_from_slice(&0.0f32.to_le_bytes());
        let b_i = BUFFER_SIZE + i * ITEM_SIZE;
        memory_image[b_i..b_i + ITEM_SIZE].copy_from_slice(&2.0f32.to_le_bytes());
        let c_i = BUFFER_SIZE + BUFFER_SIZE + i * ITEM_SIZE;
        memory_image[c_i..c_i + ITEM_SIZE].copy_from_slice(&2.0f32.to_le_bytes());
    }

    // Copy to the imported shared memory.
    let engine = default_engine();
    let memory = SharedMemory::new(&engine, MemoryType::shared(0x800, 0x800)).unwrap();
    assert!(memory.data_size() > memory_image.len());
    unsafe {
        std::ptr::copy(
            memory_image.as_ptr(),
            memory.data() as *mut u8,
            memory_image.len(),
        );
    }

    // Execute the kernel once (mimic running only the second iteration).
    let mut test_case =
        TestCase::new("tests/wat/nstream-kernel.wat", engine, Some(memory)).unwrap();
    let params = vec![
        // Second iteration (0-based).
        1.into(),
        (NUM_ITERATIONS as i32).into(),
        (BLOCK_SIZE as i32).into(),
        // Buffer A.
        0.into(),
        (BUFFER_SIZE as i32).into(),
        // Buffer B.
        (BUFFER_SIZE as i32).into(),
        (BUFFER_SIZE as i32).into(),
        // Buffer C.
        (BUFFER_SIZE as i32 * 2).into(),
        (BUFFER_SIZE as i32).into(),
    ];
    let _ = test_case.invoke("kernel", &params).unwrap();

    // Check that the second iteration of the A buffer is filled in correctly.
    let memory_as_f32s = unsafe {
        std::slice::from_raw_parts(
            test_case.memory_as_slice().as_ptr() as *const _ as *const f32,
            NUM_ITERATIONS * BLOCK_SIZE,
        )
    };
    assert_eq!(memory_as_f32s.len() * ITEM_SIZE, BUFFER_SIZE);
    let iteration_memory = &memory_as_f32s[BLOCK_SIZE..BLOCK_SIZE + BLOCK_SIZE];
    assert!(iteration_memory.iter().all(|f| f == &8.0));
}

// Here we check that we can actually run the nstream benchmark. This is also
// available using `cargo bench`, but this checks different parameters.
#[test]
fn run_sum_wat() {
    const NUM_THREADS: i32 = 8;
    const ADDITIONS_PER_THREAD: i32 = 0x200_000;
    const DEVICE_KIND: i32 = 0x1; // CPU

    // Setup the benchmark.
    let mut test_case = TestCase::new("tests/wat/sum.wat", default_engine(), None).unwrap();
    let _ = test_case
        .invoke(
            "setup",
            &[
                NUM_THREADS.into(),
                ADDITIONS_PER_THREAD.into(),
                DEVICE_KIND.into(),
            ],
        )
        .unwrap();

    // Execute the benchmark.
    let _ = test_case.invoke("execute", &[]).unwrap();

    // Run the finalize step.
    let results = test_case.invoke("finish", &[]).unwrap();
    assert_eq!(results[0].i32().unwrap(), 0);
}
