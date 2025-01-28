use crate::prelude::*;
use crate::store::StoreOpaque;
use crate::{StoreContext, StoreContextMut};
use core::fmt;
use core::marker;
use core::num::NonZeroU64;
use core::ops::{Index, IndexMut};

// This is defined here, in a private submodule, so we can explicitly reexport
// it only as `pub(crate)`. This avoids a ton of
// crate-private-type-in-public-interface errors that aren't really too
// interesting to deal with.
#[derive(Copy, Clone)]
pub struct InstanceId(pub(super) usize);

impl InstanceId {
    pub fn from_index(idx: usize) -> InstanceId {
        InstanceId(idx)
    }
}

pub struct StoreData {
    id: StoreId,
    funcs: Vec<crate::func::FuncData>,
    tables: Vec<crate::runtime::vm::ExportTable>,
    globals: Vec<crate::runtime::vm::ExportGlobal>,
    instances: Vec<crate::instance::InstanceData>,
    memories: Vec<crate::runtime::vm::ExportMemory>,
    #[cfg(feature = "component-model")]
    pub(crate) components: crate::component::ComponentStoreData,
}

pub trait StoredData: Sized {
    fn list(data: &StoreData) -> &Vec<Self>;
    fn list_mut(data: &mut StoreData) -> &mut Vec<Self>;
}

macro_rules! impl_store_data {
    ($($field:ident => $t:ty,)*) => ($(
        impl StoredData for $t {
            #[inline]
            fn list(data: &StoreData) -> &Vec<Self> { &data.$field }
            #[inline]
            fn list_mut(data: &mut StoreData) -> &mut Vec<Self> { &mut data.$field }
        }
    )*)
}

impl_store_data! {
    funcs => crate::func::FuncData,
    tables => crate::runtime::vm::ExportTable,
    globals => crate::runtime::vm::ExportGlobal,
    instances => crate::instance::InstanceData,
    memories => crate::runtime::vm::ExportMemory,
}

impl StoreData {
    pub fn new() -> StoreData {
        StoreData {
            id: StoreId::allocate(),
            funcs: Vec::new(),
            tables: Vec::new(),
            globals: Vec::new(),
            instances: Vec::new(),
            memories: Vec::new(),
            #[cfg(feature = "component-model")]
            components: Default::default(),
        }
    }

    pub fn id(&self) -> StoreId {
        self.id
    }

    pub fn insert<T>(&mut self, data: T) -> Stored<T>
    where
        T: StoredData,
    {
        let list = T::list_mut(self);
        let index = list.len();
        list.push(data);
        Stored::new(self.id, index)
    }

    pub fn next_id<T>(&self) -> Stored<T>
    where
        T: StoredData,
    {
        Stored::new(self.id, T::list(self).len())
    }

    pub fn contains<T>(&self, id: Stored<T>) -> bool
    where
        T: StoredData,
    {
        if id.store_id != self.id {
            return false;
        }
        // This should be true as an invariant of our API, but double-check with
        // debug assertions enabled.
        debug_assert!(id.index() < T::list(self).len());
        true
    }

    pub fn iter<T>(&self) -> impl ExactSizeIterator<Item = Stored<T>> + use<T>
    where
        T: StoredData,
    {
        let id = self.id;
        (0..T::list(self).len()).map(move |i| Stored::new(id, i))
    }

    pub(crate) fn reserve_funcs(&mut self, count: usize) {
        self.funcs.reserve(count);
    }
}

impl<T> Index<Stored<T>> for StoreData
where
    T: StoredData,
{
    type Output = T;

    #[inline]
    fn index(&self, index: Stored<T>) -> &Self::Output {
        index.assert_belongs_to(self.id);
        // Note that if this is ever a performance bottleneck it should be safe
        // to use unchecked indexing here because presence of a `Stored<T>` is
        // proof of an item having been inserted into a store and lists in
        // stores are never shrunk. After the store check above the actual index
        // should always be valid.
        &T::list(self)[index.index()]
    }
}

impl<T> IndexMut<Stored<T>> for StoreData
where
    T: StoredData,
{
    #[inline]
    fn index_mut(&mut self, index: Stored<T>) -> &mut Self::Output {
        index.assert_belongs_to(self.id);
        // Note that this could be unchecked indexing, see the note in `Index`
        // above.
        &mut T::list_mut(self)[index.index()]
    }
}

// forward StoreContext => StoreData
impl<I, T> Index<I> for StoreContext<'_, T>
where
    StoreData: Index<I>,
{
    type Output = <StoreData as Index<I>>::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        self.0.store_data.index(index)
    }
}

// forward StoreContextMut => StoreData
impl<I, T> Index<I> for StoreContextMut<'_, T>
where
    StoreData: Index<I>,
{
    type Output = <StoreData as Index<I>>::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        self.0.store_data.index(index)
    }
}

// forward StoreOpaque => StoreData
impl<I> Index<I> for StoreOpaque
where
    StoreData: Index<I>,
{
    type Output = <StoreData as Index<I>>::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        self.store_data().index(index)
    }
}
impl<I> IndexMut<I> for StoreOpaque
where
    StoreData: IndexMut<I>,
{
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.store_data_mut().index_mut(index)
    }
}

/// A unique identifier to get attached to a store.
///
/// This identifier is embedded into the `Stored<T>` structure and is used to
/// identify the original store that items come from. For example a `Memory` is
/// owned by a `Store` and will embed a `StoreId` internally to say which store
/// it came from. Comparisons with this value are how panics are generated for
/// mismatching the item that a store belongs to.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)] // NB: relied on in the C API
pub struct StoreId(NonZeroU64);

impl StoreId {
    /// Allocates a new unique identifier for a store that has never before been
    /// used in this process.
    pub fn allocate() -> StoreId {
        // When 64-bit atomics are allowed then allow 2^63 stores at which point
        // we start panicking to prevent overflow.
        //
        // If a store is created once per microsecond then this will last the
        // current process for 584,540 years before overflowing.
        const OVERFLOW_THRESHOLD: u64 = 1 << 63;

        #[cfg(target_has_atomic = "64")]
        let id = {
            use core::sync::atomic::{AtomicU64, Ordering::Relaxed};

            // Note the usage of `Relaxed` ordering here which should be ok
            // since we're only looking for atomicity on this counter and this
            // otherwise isn't used to synchronize memory stored anywhere else.
            static NEXT_ID: AtomicU64 = AtomicU64::new(0);
            let id = NEXT_ID.fetch_add(1, Relaxed);
            if id > OVERFLOW_THRESHOLD {
                NEXT_ID.store(OVERFLOW_THRESHOLD, Relaxed);
                panic!("store id allocator overflow");
            }
            id
        };

        // When 64-bit atomics are not allowed use a `RwLock<u64>`. This is
        // already used elsewhere in Wasmtime and currently has the
        // implementation of panic-on-contention, but it's at least no worse
        // than what wasmtime had before and is at least correct and UB-free.
        #[cfg(not(target_has_atomic = "64"))]
        let id = {
            use crate::sync::RwLock;
            static NEXT_ID: RwLock<u64> = RwLock::new(0);

            let mut lock = NEXT_ID.write();
            if *lock > OVERFLOW_THRESHOLD {
                panic!("store id allocator overflow");
            }
            let ret = *lock;
            *lock += 1;
            ret
        };

        StoreId(NonZeroU64::new(id + 1).unwrap())
    }

    #[inline]
    pub fn assert_belongs_to(&self, store: StoreId) {
        if *self == store {
            return;
        }
        store_id_mismatch();
    }

    /// Raw accessor for the C API.
    pub fn as_raw(&self) -> NonZeroU64 {
        self.0
    }

    /// Raw constructor for the C API.
    pub fn from_raw(id: NonZeroU64) -> StoreId {
        StoreId(id)
    }
}

#[repr(C)] // used by reference in the C API, also in `wasmtime_func_t`.
pub struct Stored<T> {
    store_id: StoreId,
    index: usize,
    _marker: marker::PhantomData<fn() -> T>,
}

impl<T> Stored<T> {
    fn new(store_id: StoreId, index: usize) -> Stored<T> {
        Stored {
            store_id,
            index,
            _marker: marker::PhantomData,
        }
    }

    #[inline]
    pub fn assert_belongs_to(&self, store: StoreId) {
        self.store_id.assert_belongs_to(store)
    }

    fn index(&self) -> usize {
        self.index
    }
}

#[cold]
fn store_id_mismatch() {
    panic!("object used with the wrong store");
}

impl<T> PartialEq for Stored<T> {
    fn eq(&self, other: &Stored<T>) -> bool {
        self.store_id == other.store_id && self.index == other.index
    }
}

impl<T> Copy for Stored<T> {}

impl<T> Clone for Stored<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> fmt::Debug for Stored<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "store={}, index={}", self.store_id.0, self.index())
    }
}
