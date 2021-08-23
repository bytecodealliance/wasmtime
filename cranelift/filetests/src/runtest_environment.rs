use anyhow::anyhow;
use cranelift_codegen::data_value::DataValue;
use cranelift_codegen::ir::Type;
use cranelift_reader::parse_heap_command;
use cranelift_reader::{Comment, HeapCommand};

/// Stores info about the expected environment for a test function.
#[derive(Debug, Clone)]
pub struct RuntestEnvironment {
    pub heaps: Vec<HeapCommand>,
}

impl RuntestEnvironment {
    /// Parse the environment from a set of comments
    pub fn parse(comments: &[Comment]) -> anyhow::Result<Self> {
        let mut env = RuntestEnvironment { heaps: Vec::new() };

        for comment in comments.iter() {
            if let Some(heap_command) = parse_heap_command(comment.text)? {
                let heap_index = env.heaps.len() as u64;
                let expected_ptr = heap_index * 16;
                if Some(expected_ptr) != heap_command.ptr_offset.map(|p| p.into()) {
                    return Err(anyhow!(
                        "Invalid ptr offset, expected vmctx+{}",
                        expected_ptr
                    ));
                }

                let expected_bound = (heap_index * 16) + 8;
                if Some(expected_bound) != heap_command.bound_offset.map(|p| p.into()) {
                    return Err(anyhow!(
                        "Invalid bound offset, expected vmctx+{}",
                        expected_bound
                    ));
                }

                env.heaps.push(heap_command);
            };
        }

        Ok(env)
    }

    pub fn is_active(&self) -> bool {
        !self.heaps.is_empty()
    }

    /// Allocates a struct to be injected into the test.
    pub fn runtime_struct(&self) -> RuntestContext {
        RuntestContext::new(&self)
    }
}

type HeapMemory = Vec<u8>;

/// A struct that provides info about the environment to the test
#[derive(Debug, Clone)]
pub struct RuntestContext {
    /// Store the heap memory alongside the context info so that we don't accidentally deallocate
    /// it too early.
    heaps: Vec<HeapMemory>,

    /// This is the actual struct that gets passed into the `vmctx`  argument of the tests.
    /// It has a specific memory layout that all tests agree with.
    ///
    /// Currently we only have to store heap info, so we store the heap start and end addresses in
    /// a 64 bit slot for each heap.
    ///
    /// ┌────────────┐
    /// │heap0: start│
    /// ├────────────┤
    /// │heap0: end  │
    /// ├────────────┤
    /// │heap1: start│
    /// ├────────────┤
    /// │heap1: end  │
    /// ├────────────┤
    /// │etc...      │
    /// └────────────┘
    context_struct: Vec<u64>,
}

impl RuntestContext {
    pub fn new(env: &RuntestEnvironment) -> Self {
        let heaps: Vec<HeapMemory> = env
            .heaps
            .iter()
            .map(|cmd| {
                let size: u64 = cmd.size.into();
                vec![0u8; size as usize]
            })
            .collect();

        let context_struct = heaps
            .iter()
            .flat_map(|heap| [heap.as_ptr(), heap.as_ptr().wrapping_add(heap.len())])
            .map(|p| p as usize as u64)
            .collect();

        Self {
            heaps,
            context_struct,
        }
    }

    /// Creates a [DataValue] with a target isa pointer type to the context struct.
    pub fn pointer(&self, ty: Type) -> DataValue {
        let ptr = self.context_struct.as_ptr() as usize as i128;
        DataValue::from_integer(ptr, ty).expect("Failed to cast pointer to native target size")
    }
}
