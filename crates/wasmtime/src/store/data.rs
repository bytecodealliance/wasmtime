use crate::store::StoreOpaque;
use crate::{StoreContext, StoreContextMut};
use std::fmt;
use std::marker;
use std::num::NonZeroU64;
use std::ops::Index;
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
    instances: Vec<crate::instance::RuntimeInstance>,
    memories: Vec<wasmtime_runtime::ExportMemory>,
}

pub trait StoredData: Sized {
    fn get<'a>(id: Stored<Self>, data: &'a StoreData) -> &'a Self;
    fn insert(self, data: &mut StoreData) -> Stored<Self>;
    fn assert_contained(id: Stored<Self>, data: &StoreData);
}

macro_rules! impl_store_data {
    ($($field:ident => $t:ty,)*) => ($(
        impl StoredData for $t {
            #[inline]
            fn get<'a>(id: Stored<Self>, data: &'a StoreData) -> &'a Self {
                assert!(id.store_id() == data.id,
                "object used with the wrong store");
                &data.$field[id.index()]
            }

            fn insert(self, data: &mut StoreData) -> Stored<Self> {
                let index = data.$field.len();
                data.$field.push(self);
                Stored::new(data.id, index)
            }

            fn assert_contained(id: Stored<Self>, data: &StoreData) {
                assert!(id.index() < data.$field.len());
            }
        }
    )*)
}

impl_store_data! {
    funcs => crate::func::FuncData,
    tables => wasmtime_runtime::ExportTable,
    globals => wasmtime_runtime::ExportGlobal,
    instances => crate::instance::RuntimeInstance,
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
        data.insert(self)
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
            T::assert_contained(id, self);
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
        T::get(index, self)
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
impl<I> Index<I> for StoreOpaque<'_>
where
    StoreData: Index<I>,
{
    type Output = <StoreData as Index<I>>::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        self.store_data().index(index)
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
