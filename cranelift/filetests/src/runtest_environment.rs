use anyhow::anyhow;
use cranelift_codegen::ir::{ArgumentPurpose, Function};
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

    /// Allocates memory for heaps
    pub fn allocate_memory(&self) -> Vec<HeapMemory> {
        self.heaps
            .iter()
            .map(|cmd| {
                let size: u64 = cmd.size.into();
                vec![0u8; size as usize]
            })
            .collect()
    }

    /// Validates the signature of a [Function] ensuring that if this environment is active, the
    /// function has a `vmctx` argument
    pub fn validate_signature(&self, func: &Function) -> Result<(), String> {
        let first_arg_is_vmctx = func
            .signature
            .params
            .first()
            .map(|p| p.purpose == ArgumentPurpose::VMContext)
            .unwrap_or(false);

        if !first_arg_is_vmctx && self.is_active() {
            return Err(concat!(
                "This test requests a heap, but the first argument is not `i64 vmctx`.\n",
                "See docs/testing.md for more info on using heap annotations."
            )
            .to_string());
        }

        Ok(())
    }
}

pub(crate) type HeapMemory = Vec<u8>;
