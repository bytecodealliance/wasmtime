use crate::prelude::*;
use crate::store::{StoreData, StoreOpaque, StoredData};
use crate::{AsContext, AsContextMut, Store, StoreContextMut};

/// Default amount of fuel allowed for all guest-to-host calls in the component
/// model.
///
/// This is the maximal amount of data which will be copied from the guest to
/// the host by default. This is set to a very large value to avoid breaking
/// existing embeddings for when this feature was backported.
const DEFAULT_HOSTCALL_FUEL: usize = 2 << 30;

macro_rules! component_store_data {
    ($($field:ident => $t:ty,)*) => (
        pub struct ComponentStoreData {
            $($field: Vec<$t>,)*

            /// Fuel to be used for each time the guest calls the host or
            /// transfers data to the host.
            ///
            /// Caps the size of the allocations made on the host to this amount
            /// effectively.
            hostcall_fuel: usize,
        }

        $(
            impl StoredData for $t {
                #[inline]
                fn list(data: &StoreData) -> &Vec<Self> {
                    &data.components.$field
                }
                #[inline]
                fn list_mut(data: &mut StoreData) -> &mut Vec<Self> {
                    &mut data.components.$field
                }
            }
        )*

        impl Default for ComponentStoreData {
            fn default() -> Self {
                Self {
                    $($field: Default::default(),)*
                    hostcall_fuel: DEFAULT_HOSTCALL_FUEL,
                }
            }
        }
    )
}

component_store_data! {
    funcs => crate::component::func::FuncData,
    instances => Option<Box<crate::component::instance::InstanceData>>,
}

impl StoreOpaque {
    pub(crate) fn hostcall_fuel(&self) -> usize {
        self.store_data().components.hostcall_fuel
    }

    pub(crate) fn set_hostcall_fuel(&mut self, fuel: usize) {
        self.store_data_mut().components.hostcall_fuel = fuel;
    }
}

impl<T> Store<T> {
    /// Returns the amount of "hostcall fuel" used for guest-to-host component
    /// calls.
    ///
    /// This is either the default amount if it hasn't been configured or
    /// returns the last value passed to [`Store::set_hostcall_fuel`].
    ///
    /// See [`Store::set_hostcall_fuel`] `for more details.
    pub fn hostcall_fuel(&self) -> usize {
        self.as_context().0.hostcall_fuel()
    }

    /// Sets the amount of "hostcall fuel" used for guest-to-host component
    /// calls.
    ///
    /// Whenever the guest calls the host it often wants to transfer some data
    /// as well, such as strings or lists. This configured fuel value can be
    /// used to limit the amount of data that the host allocates on behalf of
    /// the guest. This is a DoS mitigation mechanism to prevent a malicious
    /// guest from causing the host to allocate an unbounded amount of memory
    /// for example.
    ///
    /// Fuel is considered distinct for each host call. The host is responsible
    /// for ensuring it retains a proper amount of data between host calls if
    /// applicable. The `fuel` provided here will be the initial value for each
    /// time the guest calls the host.
    ///
    /// The `fuel` value here should roughly corresponds to the maximal number
    /// of bytes that the guest may transfer to the host in one call.
    ///
    /// Note that data transferred from the host to the guest is not limited
    /// because it's already resident on the host itself. Only data from the
    /// guest to the host is limited.
    ///
    /// The default value for this is 128 MiB.
    pub fn set_hostcall_fuel(&mut self, fuel: usize) {
        self.as_context_mut().set_hostcall_fuel(fuel)
    }
}

impl<T> StoreContextMut<'_, T> {
    /// See [`Store::hostcall_fuel`].
    pub fn hostcall_fuel(&self) -> usize {
        self.0.hostcall_fuel()
    }

    /// See [`Store::set_hostcall_fuel`].
    pub fn set_hostcall_fuel(&mut self, fuel: usize) {
        self.0.set_hostcall_fuel(fuel)
    }
}
