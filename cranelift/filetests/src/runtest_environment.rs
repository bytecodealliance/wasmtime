use anyhow::anyhow;
use cranelift_codegen::ir::immediates::Uimm64;
use cranelift_codegen::ir::{ArgumentPurpose, Function};
use cranelift_reader::{parse_heap_command, parse_table_command};
use cranelift_reader::{Comment, HeapCommand, TableCommand};

#[derive(Debug, Clone)]
pub enum RuntestEntry {
    Heap(HeapCommand),
    Table(TableCommand),
}

impl RuntestEntry {
    /// Tries to parse an entry from a comment, returning None if it isn't possible
    pub fn parse_from_comment(comment: &Comment) -> anyhow::Result<Option<Self>> {
        if let Some(heap_command) = parse_heap_command(comment.text)? {
            return Ok(Some(RuntestEntry::Heap(heap_command)));
        }
        if let Some(table_command) = parse_table_command(comment.text)? {
            return Ok(Some(RuntestEntry::Table(table_command)));
        }
        Ok(None)
    }

    pub fn ptr_offset(&self) -> Option<Uimm64> {
        match self {
            RuntestEntry::Heap(heap) => heap.ptr_offset,
            RuntestEntry::Table(table) => table.ptr_offset,
        }
    }

    pub fn bound_offset(&self) -> Option<Uimm64> {
        match self {
            RuntestEntry::Heap(heap) => heap.bound_offset,
            RuntestEntry::Table(table) => table.bound_offset,
        }
    }
}

/// Stores info about the expected environment for a test function.
#[derive(Debug, Clone)]
pub struct RuntestEnvironment {
    pub entries: Vec<RuntestEntry>,
}

impl RuntestEnvironment {
    /// Parse the environment from a set of comments
    pub fn parse(comments: &[Comment]) -> anyhow::Result<Self> {
        let mut env = RuntestEnvironment {
            entries: Vec::new(),
        };

        // The order of the VMCtx memory is going to be dictated by the order of the comments
        // we also enforce the correct vmctx offsets on the comments based on that.
        for comment in comments.iter() {
            let entry = env.entries.len() as u64;
            let expected_ptr = entry * 16;
            let expected_bound = (entry * 16) + 8;

            if let Some(entry) = RuntestEntry::parse_from_comment(comment)? {
                if Some(expected_ptr) != entry.ptr_offset().map(|p| p.into()) {
                    return Err(anyhow!(
                        "Invalid ptr offset, expected vmctx+{}",
                        expected_ptr
                    ));
                }

                if Some(expected_bound) != entry.bound_offset().map(|p| p.into()) {
                    return Err(anyhow!(
                        "Invalid bound offset, expected vmctx+{}",
                        expected_bound
                    ));
                }

                env.entries.push(entry);
            };
        }

        Ok(env)
    }

    pub fn is_active(&self) -> bool {
        !self.entries.is_empty()
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

    /// Allocates a struct to be injected into the test.
    pub fn runtime_struct(
        &self,
        mut alloc_heap: impl FnMut(u64) -> u64,
        mut alloc_table: impl FnMut(u64, u64) -> u64,
    ) -> Vec<u64> {
        let context_struct = self
            .entries
            .iter()
            .flat_map(|entry| match entry {
                RuntestEntry::Heap(heap) => {
                    let size: u64 = heap.size.into();
                    [alloc_heap(size), size]
                }
                RuntestEntry::Table(table) => {
                    let entry_size: u64 = table.entry_size.into();
                    let entry_count: u64 = table.entry_count.into();
                    let bytes = entry_size * entry_count;

                    [alloc_table(entry_size, entry_count), bytes]
                }
            })
            .collect();

        context_struct
    }
}

pub(crate) type HeapMemory = Vec<u8>;
pub(crate) type TableMemory = Vec<u8>;
