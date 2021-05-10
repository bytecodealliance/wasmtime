use crate::store::StoreOpaque;
use crate::{StoreContext, StoreContextMut};
use std::convert::TryFrom;
use std::fmt;
use std::marker;
use std::ops::Index;
use std::sync::atomic::{AtomicU64, Ordering::SeqCst};

// This is defined here, in a private submodule, so we can explicitly reexport
// it only as `pub(crate)`. This avoids a ton of
// crate-private-type-in-public-interface errors that aren't really too
// interesting to deal with.
#[derive(Copy, Clone)]
pub struct InstanceId(pub(super) usize);

pub struct StoreData {
    id: u64,
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

        // Currently we neither recycle ids nor do we allow overlap of ids (e.g.
        // the ABA problem). We also only allocate a certain number of bits
        // (controlled by INDEX_BITS below) for the id. Make sure that the id
        // fits in the allocated bits (currently 40 bits).
        //
        // Note that this is the maximal number of `Store` instances that a
        // process can make before it needs to be restarted. That means this
        // needs to be pretty reasonable. At the assumption of creating 10k
        // stores per second 40 bits allows that program to run for ~3.5 years.
        // Hopefully programs don't run that long.
        //
        // If a program does indeed run that long then we rest the counter back
        // to a known bad value (and it's basically impossible the counter will
        // wrap back to zero inbetween this time) and then panic the current
        // thread.
        let id = NEXT_ID.fetch_add(1, SeqCst);
        let upper_bits_used = (id >> (64 - INDEX_BITS)) != 0;
        if upper_bits_used {
            NEXT_ID.store(1 << (64 - INDEX_BITS), SeqCst);
            panic!("store id allocator overflow");
        }

        StoreData {
            id,
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

#[repr(transparent)]
pub struct Stored<T> {
    // See documentation below on `INDEX_BITS` for how this is interpreted.
    bits: u64,
    _marker: marker::PhantomData<fn() -> T>,
}

// This is the maximal number of bits that the index of an item within a store
// can take up. As this is set to 24 that allows for 16 million items. Note
// that this is not a limit on something like the number of functions within an
// instance, only a limit on the number of externally referenced items in a
// Store. For example this is more equivalent to exported functions of a module
// rather than functions themselves.
//
// The reason for this limitation is that we want `Stored<T>` to fit into a
// 64-bit value (for the C API). This 64-bit value gives us limited, well, uh,
// bits, to work with. We need to pack both a "store id" as well as an index
// within the store into those 64 bits. Given that there's no implementation of
// recycling store IDs at this time it also means that the number of bits
// allocated to the store id represents the maximal number of stores that a
// process can create for its entire lifetime.
//
// These factors led to the choice of bits here for this. This can be moved
// around a bit, but the hope is that this is good enough for all practical
// users.
//
// The choice of 24 means that the limitations of wasmtime are:
//
// * 24 bits for the index, meaning 16 million items maximum. As noted above
//   this is 16 million *host references* to wasm items, so this is akin to
//   creating 16 million instances within one store or creating an instance
//   that has 16 million exported function. If 24 bits isn't enough then we
//   may need to look into compile-time options to change this perhaps.
//
// * 40 bits for the store id. This is a whole lot more bits than the index,
//   but intentionally so. As the maximal number of stores for the entire
//   process that's far more limiting than the number of items within a store
//   (which are typically drastically lower than 16 million and/or limited via
//   other means, e.g. wasm module validation, instance limits, etc).
//
// So all-in-all we try to maximize the number of store bits without placing
// too many restrictions on the number of items within a store. Using 40
// bits practically means that if you create 10k stores a second your program
// can run for ~3.5 years. Hopefully that's enough?
//
// If we didn't need to be clever in the C API and returned structs-by-value
// instead of returning 64-bit integers then we could just change this to a
// u64/usize pair which would solve all of these problems. Hopefully, though,
// no one will ever run into these limits...
const INDEX_BITS: usize = 24;

impl<T> Stored<T> {
    fn new(store_id: u64, index: usize) -> Stored<T> {
        let masked_index = ((1 << INDEX_BITS) - 1) & index;
        if masked_index != index {
            panic!("too many items have been allocated into the store");
        }
        Stored {
            bits: (store_id << INDEX_BITS) | u64::try_from(masked_index).unwrap(),
            _marker: marker::PhantomData,
        }
    }

    fn store_id(&self) -> u64 {
        self.bits >> INDEX_BITS
    }

    fn index(&self) -> usize {
        usize::try_from(self.bits & ((1 << INDEX_BITS) - 1)).unwrap()
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
