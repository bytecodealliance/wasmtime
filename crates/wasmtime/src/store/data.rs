use crate::store::StoreOpaque;
use crate::{StoreContext, StoreContextMut};
use std::fmt;
use std::marker;
use std::num::NonZeroU64;
use std::ops::{Index, IndexMut};
use std::sync::atomic::{AtomicU64, Ordering::SeqCst};

// This is defined here, in a private submodule, so we can explicitly reexport
// it only as `pub(crate)`. This avoids a ton of
// crate-private-type-in-public-interface errors that aren't really too
// interesting to deal with.
#[derive(Copy, Clone)]
pub struct InstanceId(pub(super) usize);

pub struct StoreData {
    id: NonZeroU64,
    funcs: Vec<crate::func::FuncData>,
    tables: Vec<wasmtime_runtime::ExportTable>,
    globals: Vec<wasmtime_runtime::ExportGlobal>,
    instances: Vec<crate::instance::InstanceData>,
    memories: Vec<wasmtime_runtime::ExportMemory>,
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
    tables => wasmtime_runtime::ExportTable,
    globals => wasmtime_runtime::ExportGlobal,
    instances => crate::instance::InstanceData,
    memories => wasmtime_runtime::ExportMemory,
}

impl StoreData {
    pub fn new() -> StoreData {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);

        // Only allow 2^63 stores at which point we start panicking to prevent
        // overflow. This should still last us to effectively the end of time.
        let id = NEXT_ID.fetch_add(1, SeqCst);
        if id & (1 << 63) != 0 {
            NEXT_ID.store(1 << 63, SeqCst);
            panic!("store id allocator overflow");
        }

        StoreData {
            id: NonZeroU64::new(id + 1).unwrap(),
            funcs: Vec::new(),
            tables: Vec::new(),
            globals: Vec::new(),
            instances: Vec::new(),
            memories: Vec::new(),
        }
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
        if id.store_id() != self.id {
            return false;
        }
        // this should be true as an invariant of our API, but double-check with
        // debug assertions enabled.
        if cfg!(debug_assertions) {
            assert!(id.index() < T::list(self).len());
        }
        true
    }
}

impl<T> Index<Stored<T>> for StoreData
where
    T: StoredData,
{
    type Output = T;

    #[inline]
    fn index(&self, index: Stored<T>) -> &Self::Output {
        assert!(
            index.store_id() == self.id,
            "object used with the wrong store"
        );
        &T::list(self)[index.index()]
    }
}

impl<T> IndexMut<Stored<T>> for StoreData
where
    T: StoredData,
{
    #[inline]
    fn index_mut(&mut self, index: Stored<T>) -> &mut Self::Output {
        assert!(
            index.store_id() == self.id,
            "object used with the wrong store"
        );
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

#[repr(C)] // used by reference in the C API
pub struct Stored<T> {
    store_id: NonZeroU64,
    index: usize,
    _marker: marker::PhantomData<fn() -> T>,
}

impl<T> Stored<T> {
    fn new(store_id: NonZeroU64, index: usize) -> Stored<T> {
        Stored {
            store_id,
            index,
            _marker: marker::PhantomData,
        }
    }

    fn store_id(&self) -> NonZeroU64 {
        self.store_id
    }

    fn index(&self) -> usize {
        self.index
    }
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
        write!(f, "store={}, index={}", self.store_id(), self.index())
    }
}
