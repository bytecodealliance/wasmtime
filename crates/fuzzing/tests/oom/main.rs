mod arc;
mod bforest_map;
mod bforest_set;
mod bit_set;
mod boxed;
mod btree_map;
mod config;
mod engine;
mod entity_set;
mod error;
mod func_type;
mod hash_map;
mod hash_set;
mod index_map;
mod instance_pre;
mod linker;
mod module;
mod primary_map;
mod secondary_map;
mod smoke;
mod store;
mod string;
mod vec;

use wasmtime_fuzzing::oom::OomTestAllocator;

#[global_allocator]
static GLOBAL_ALLOCATOR: OomTestAllocator = OomTestAllocator::new();

/// Entity key for testing fallible `PrimaryMap`s and such.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Key(u32);
wasmtime_environ::entity_impl!(Key);
