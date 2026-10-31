mod any;
mod host;
mod host_dynamic;
mod host_static;
mod host_tables;
mod ty;

use crate::hash_map::HashMap;
use crate::hash_set::HashSet;
use crate::prelude::*;
use alloc::collections::{BTreeMap, BTreeSet, VecDeque};

pub use any::*;
pub use host_dynamic::*;
pub use host_static::*;
pub use host_tables::*;
pub use ty::*;

/// Trait for types that can report the host heap memory allocated for their children.
///
/// This trait must be implemented by any type `T` stored in a [`ResourceTable`].
/// Implementations should report only dynamically allocated bytes owned by
/// descendants of `self` (such as `Vec` capacity, `String` capacity, or boxed
/// children), **not** the inline bytes of `self` (`size_of_val(self)`).
///
/// Types that own no growable heap allocations should implement
/// [`FixedHostHeapUsage`] instead, for which there is a blanket `HostHeapUsage`
/// implementation.
///
/// # Deriving
///
/// In most cases, derive [`HostHeapUsage`](macro@crate::component::HostHeapUsage)
/// (for types with growable fields like `String`, `Vec<T>`, or `HashMap<K, V>`)
/// or [`FixedHostHeapUsage`](macro@crate::component::FixedHostHeapUsage) (for
/// fixed-size types, which statically checks that every field is fixed-size):
///
/// ```
/// use wasmtime::component::{FixedHostHeapUsage, HostHeapUsage};
///
/// #[derive(FixedHostHeapUsage)]
/// struct FixedConfig {
///     id: u32,
///     timeout: Option<std::time::Duration>,
/// }
///
/// #[derive(HostHeapUsage)]
/// struct MyResource {
///     config: FixedConfig,
///     name: String,
///     buffer: Vec<u8>,
/// }
/// ```
///
/// # Manual Implementation
///
/// Implement `HostHeapUsage` manually when a type holds fields that do not
/// implement `HostHeapUsage` directly or when maintaining a cached count.
///
/// ## Caching heap size for large collections
///
/// The default `HostHeapUsage` implementations for `Vec<T>`, `HashSet<T>`, and
/// `HashMap<K, V>` traverse their elements to sum child allocations, running in
/// time linear to collection length. When a resource holds a large collection
/// that is mutated frequently via [`ResourceTable::update`], embedders
/// can avoid rescanning elements on every update by maintaining a cached byte
/// count alongside the collection:
///
/// ```
/// use std::collections::HashMap;
/// use wasmtime::component::HostHeapUsage;
///
/// #[derive(Default)]
/// struct CachedMap {
///     entries: HashMap<String, Vec<u8>>,
///     child_bytes: usize,
/// }
///
/// impl CachedMap {
///     fn insert(&mut self, key: String, value: Vec<u8>) {
///         let key_bytes = key.heap_size_of_children();
///         self.child_bytes += value.heap_size_of_children();
///         match self.entries.insert(key, value) {
///             Some(old) => self.child_bytes -= old.heap_size_of_children(),
///             None => self.child_bytes += key_bytes,
///         }
///     }
/// }
///
/// impl HostHeapUsage for CachedMap {
///     fn heap_size_of_children(&self) -> usize {
///         let table_bytes = self.entries.capacity()
///             * (size_of::<String>() + size_of::<Vec<u8>>() + 1);
///         table_bytes + self.child_bytes
///     }
/// }
/// ```
///
/// [`ResourceTable`]: super::ResourceTable
/// [`ResourceTable::update`]: super::ResourceTable::update
pub trait HostHeapUsage {
    /// Returns the number of bytes allocated on the heap owned by descendants
    /// of this value, excluding `size_of_val(self)`.
    fn heap_size_of_children(&self) -> usize;
}

/// Marker trait for types whose [`HostHeapUsage::heap_size_of_children`] can
/// never change through mutation of `&mut self`.
///
/// This holds for types that own no growable heap data: scalars, plain enums
/// and structs of such types, OS handles, and handles to state that is not
/// solely owned (e.g. `Arc<T>` where `T` is bounded elsewhere). Such types may
/// be mutated with [`ResourceTable::get_mut`]; all other types must use
/// [`ResourceTable::update`].
///
/// Implementing `FixedHostHeapUsage` automatically provides a [`HostHeapUsage`]
/// implementation (returning `0` heap children) via a blanket impl. Prefer
/// `#[derive(FixedHostHeapUsage)]` where possible so the compiler verifies
/// that every field is also fixed-size.
///
/// # Contract
///
/// Violating this contract does not cause memory unsafety, but makes the
/// table under-report usage for the resource. If underlying fields have
/// interior mutability that allows large or unbounded allocations resulting
/// from guest hostcalls, this could be a DoS attack vector.
///
/// [`ResourceTable`]: super::ResourceTable
/// [`ResourceTable::get_mut`]: super::ResourceTable::get_mut
/// [`ResourceTable::update`]: super::ResourceTable::update
pub trait FixedHostHeapUsage: HostHeapUsage + Sized {}

impl<T: FixedHostHeapUsage> HostHeapUsage for T {
    #[inline]
    fn heap_size_of_children(&self) -> usize {
        0
    }
}

/// Helper trait used by `#[derive(FixedHostHeapUsage)]` to check field types
/// without conflicting with `impl<T: HostHeapUsage> HostHeapUsage for Option<T>`.
#[doc(hidden)]
pub trait FixedHostHeapField {}

impl<T: FixedHostHeapUsage> FixedHostHeapField for T {}
impl<T: FixedHostHeapField> FixedHostHeapField for Option<T> {}
impl<T: FixedHostHeapField, E: FixedHostHeapField> FixedHostHeapField for Result<T, E> {}
impl<T: FixedHostHeapField, const N: usize> FixedHostHeapField for [T; N] {}

impl FixedHostHeapUsage for () {}
impl FixedHostHeapUsage for bool {}
impl FixedHostHeapUsage for u8 {}
impl FixedHostHeapUsage for u16 {}
impl FixedHostHeapUsage for u32 {}
impl FixedHostHeapUsage for u64 {}
impl FixedHostHeapUsage for u128 {}
impl FixedHostHeapUsage for usize {}
impl FixedHostHeapUsage for i8 {}
impl FixedHostHeapUsage for i16 {}
impl FixedHostHeapUsage for i32 {}
impl FixedHostHeapUsage for i64 {}
impl FixedHostHeapUsage for i128 {}
impl FixedHostHeapUsage for isize {}
impl FixedHostHeapUsage for f32 {}
impl FixedHostHeapUsage for f64 {}
impl FixedHostHeapUsage for char {}
impl FixedHostHeapUsage for core::time::Duration {}
impl FixedHostHeapUsage for core::net::IpAddr {}
impl FixedHostHeapUsage for core::net::Ipv4Addr {}
impl FixedHostHeapUsage for core::net::Ipv6Addr {}
impl FixedHostHeapUsage for core::net::SocketAddr {}
impl FixedHostHeapUsage for core::net::SocketAddrV4 {}
impl FixedHostHeapUsage for core::net::SocketAddrV6 {}
// Boxed error values are out of scope for resource table limits.
impl FixedHostHeapUsage for crate::Error {}
#[cfg(feature = "anyhow")]
impl FixedHostHeapUsage for crate::anyhow::Error {}

impl<T: ?Sized> FixedHostHeapUsage for core::marker::PhantomData<T> {}

impl<T: HostHeapUsage, const N: usize> HostHeapUsage for [T; N] {
    fn heap_size_of_children(&self) -> usize {
        self.iter().map(|t| t.heap_size_of_children()).sum()
    }
}

macro_rules! impl_tuple_heap_usage {
    ($($t:ident : $idx:tt),+) => {
        impl<$($t: FixedHostHeapField),+> FixedHostHeapField for ($($t,)+) {}

        impl<$($t: HostHeapUsage),+> HostHeapUsage for ($($t,)+) {
            fn heap_size_of_children(&self) -> usize {
                0 $(+ self.$idx.heap_size_of_children())+
            }
        }
    };
}
impl_tuple_heap_usage!(T0: 0);
impl_tuple_heap_usage!(T0: 0, T1: 1);
impl_tuple_heap_usage!(T0: 0, T1: 1, T2: 2);
impl_tuple_heap_usage!(T0: 0, T1: 1, T2: 2, T3: 3);

impl<T: HostHeapUsage> HostHeapUsage for Option<T> {
    fn heap_size_of_children(&self) -> usize {
        match self {
            Some(t) => t.heap_size_of_children(),
            None => 0,
        }
    }
}

impl<T: HostHeapUsage, E: HostHeapUsage> HostHeapUsage for Result<T, E> {
    fn heap_size_of_children(&self) -> usize {
        match self {
            Ok(t) => t.heap_size_of_children(),
            Err(e) => e.heap_size_of_children(),
        }
    }
}

impl HostHeapUsage for String {
    #[inline]
    fn heap_size_of_children(&self) -> usize {
        self.capacity()
    }
}

// Traversal of collection elements makes heap usage queries linear in
// collection size; see the `HostHeapUsage` docs for a cached-bytes pattern
// when mutating large collections frequently.
impl<T: HostHeapUsage> HostHeapUsage for Vec<T> {
    fn heap_size_of_children(&self) -> usize {
        self.capacity() * size_of::<T>()
            + self
                .iter()
                .map(|t| t.heap_size_of_children())
                .sum::<usize>()
    }
}

impl<T: HostHeapUsage> HostHeapUsage for VecDeque<T> {
    fn heap_size_of_children(&self) -> usize {
        self.capacity() * size_of::<T>()
            + self
                .iter()
                .map(|t| t.heap_size_of_children())
                .sum::<usize>()
    }
}

impl<T: HostHeapUsage> HostHeapUsage for BTreeSet<T> {
    fn heap_size_of_children(&self) -> usize {
        self.len() * size_of::<T>()
            + self
                .iter()
                .map(|t| t.heap_size_of_children())
                .sum::<usize>()
    }
}

impl<K: HostHeapUsage, V: HostHeapUsage> HostHeapUsage for BTreeMap<K, V> {
    fn heap_size_of_children(&self) -> usize {
        self.len() * (size_of::<K>() + size_of::<V>())
            + self
                .iter()
                .map(|(k, v)| k.heap_size_of_children() + v.heap_size_of_children())
                .sum::<usize>()
    }
}

// Approximate hashbrown table allocation + element children.
impl<T: HostHeapUsage, S> HostHeapUsage for HashSet<T, S> {
    fn heap_size_of_children(&self) -> usize {
        self.capacity() * (size_of::<T>() + 1)
            + self
                .iter()
                .map(|t| t.heap_size_of_children())
                .sum::<usize>()
    }
}

// Approximate hashbrown table allocation + key/value children.
impl<K: HostHeapUsage, V: HostHeapUsage, S> HostHeapUsage for HashMap<K, V, S> {
    fn heap_size_of_children(&self) -> usize {
        self.capacity() * (size_of::<K>() + size_of::<V>() + 1)
            + self
                .iter()
                .map(|(k, v)| k.heap_size_of_children() + v.heap_size_of_children())
                .sum::<usize>()
    }
}

// Store-indexed handles; their backing state is owned by the Store.
impl FixedHostHeapUsage for crate::Func {}
impl FixedHostHeapUsage for crate::Global {}
impl FixedHostHeapUsage for crate::Memory {}
impl FixedHostHeapUsage for crate::Table {}
impl FixedHostHeapUsage for crate::Tag {}
impl FixedHostHeapUsage for crate::Instance {}
impl FixedHostHeapUsage for crate::Module {}
#[cfg(feature = "debug")]
impl FixedHostHeapUsage for crate::FrameHandle {}
#[cfg(feature = "threads")]
impl FixedHostHeapUsage for crate::SharedMemory {}
