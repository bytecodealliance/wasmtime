use crate::runtime::vm::VMStore;
use crate::{StoreContextMut, store::StoreId};
use core::marker::PhantomData;

pub struct StoreToken<T> {
    id: StoreId,
    _phantom: PhantomData<fn() -> T>,
}

impl<T> Clone for StoreToken<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            _phantom: PhantomData,
        }
    }
}

impl<T> Copy for StoreToken<T> {}

impl<T> StoreToken<T> {
    pub fn new(store: StoreContextMut<T>) -> Self {
        Self {
            id: store.0.id(),
            _phantom: PhantomData,
        }
    }

    pub fn as_context_mut<'a>(&self, store: &'a mut dyn VMStore) -> StoreContextMut<'a, T> {
        assert_eq!(store.store_opaque().id(), self.id);
        // We know the store with this ID has data type parameter `T` because
        // we witnessed that in `Self::new`, which is the only way `self` could
        // have been safely created:
        unsafe { store.unchecked_context_mut::<T>() }
    }
}
