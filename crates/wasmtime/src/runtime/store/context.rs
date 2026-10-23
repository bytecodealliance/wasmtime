use crate::store::{Store, StoreInner};

/// A temporary handle to a [`&Store<T>`][`Store`].
///
/// This type is suitable for [`AsContext`] trait bounds on methods if desired.
/// For more information, see [`Store`].
// NB the repr(transparent) here is for the C API and it's important that the
// representation of this `struct` is a pointer for now. If the representation
// changes then the C API will need to be updated
#[repr(transparent)]
pub struct StoreContext<'a, T: 'static>(pub(crate) &'a StoreInner<T>);

/// A temporary handle to a [`&mut Store<T>`][`Store`].
///
/// This type is suitable for [`AsContextMut`] or [`AsContext`] trait bounds on
/// methods if desired.  For more information, see [`Store`].
// NB the repr(transparent) here is for the same reason as above.
#[repr(transparent)]
pub struct StoreContextMut<'a, T: 'static>(pub(crate) &'a mut StoreInner<T>);

/// A trait used to get shared access to a [`Store`] in Wasmtime.
///
/// This trait is used as a bound on the first argument of many methods within
/// Wasmtime. This trait is implemented for types like [`Store`],
/// [`Caller`](crate::Caller), and [`StoreContext`] itself. Implementors of this
/// trait provide access to a [`StoreContext`] via some means, allowing the
/// method in question to get access to the store's internal information.
///
/// Note that this is only used in contexts where the store's information is
/// read, but not written. For example methods that return type information will
/// use this trait as a bound. More commonly, though, mutation is required and
/// [`AsContextMut`] is needed.
pub trait AsContext {
    /// The host information associated with the [`Store`], aka the `T` in
    /// [`Store<T>`].
    type Data: 'static;

    /// Returns the store context that this type provides access to.
    fn as_context(&self) -> StoreContext<'_, Self::Data>;
}

/// A trait used to get exclusive mutable access to a [`Store`] in Wasmtime.
///
/// This trait is used as a bound on the first argument of many methods within
/// Wasmtime. This trait is implemented for types like [`Store`],
/// [`Caller`](crate::Caller), and [`StoreContextMut`] itself. Implementors of
/// this trait provide access to a [`StoreContextMut`] via some means, allowing
/// the method in question to get access to the store's internal information.
///
/// This is notably used for methods that may require some mutation of the
/// [`Store`] itself. For example calling a wasm function can mutate linear
/// memory or globals. Creation of a [`Func`](crate::Func) will update internal
/// data structures. This ends up being quite a common bound in Wasmtime, but
/// typically you can simply pass `&mut store` or `&mut caller` to satisfy it.
///
/// # Calling multiple methods that take `&mut impl AsContextMut`
///
/// As of Rust 1.53.0, [generic methods that take a generic `&mut T` do not get
/// "automatic reborrowing"][reborrowing] and therefore you cannot call multiple
/// generic methods with the same `&mut T` without manually inserting
/// reborrows. This affects the many `wasmtime` API methods that take `&mut impl
/// AsContextMut`.
///
/// For example, this fails to compile because the context is moved into the
/// first call:
///
/// ```compile_fail
/// use wasmtime::{AsContextMut, Instance};
///
/// fn foo(cx: &mut impl AsContextMut, instance: Instance) {
///     // `cx` is not reborrowed, but moved into this call.
///     let my_export = instance.get_export(cx, "my_export");
///
///     // Therefore, this use of `cx` is a use-after-move and prohibited by the
///     // borrow checker.
///     let other_export = instance.get_export(cx, "other_export");
/// #   drop((my_export, other_export));
/// }
/// ```
///
/// To fix this, manually insert reborrows like `&mut *cx` that would otherwise
/// normally be inserted automatically by the Rust compiler for non-generic
/// methods:
///
/// ```
/// use wasmtime::{AsContextMut, Instance};
///
/// fn foo(cx: &mut impl AsContextMut, instance: Instance) {
///     let my_export = instance.get_export(&mut *cx, "my_export");
///
///     // This works now, since `cx` was reborrowed above, rather than moved!
///     let other_export = instance.get_export(&mut *cx, "other_export");
/// #   drop((my_export, other_export));
/// }
/// ```
///
/// [reborrowing]: https://github.com/rust-lang/rust/issues/85161
pub trait AsContextMut: AsContext {
    /// Returns the store context that this type provides access to.
    fn as_context_mut(&mut self) -> StoreContextMut<'_, Self::Data>;
}

impl<T: 'static> AsContext for Store<T> {
    type Data = T;

    #[inline]
    fn as_context(&self) -> StoreContext<'_, T> {
        StoreContext(&self.inner)
    }
}

impl<T: 'static> AsContextMut for Store<T> {
    #[inline]
    fn as_context_mut(&mut self) -> StoreContextMut<'_, T> {
        StoreContextMut(&mut self.inner)
    }
}

impl<T: 'static> AsContext for StoreContext<'_, T> {
    type Data = T;

    #[inline]
    fn as_context(&self) -> StoreContext<'_, T> {
        StoreContext(&*self.0)
    }
}

impl<T: 'static> AsContext for StoreContextMut<'_, T> {
    type Data = T;

    #[inline]
    fn as_context(&self) -> StoreContext<'_, T> {
        StoreContext(&*self.0)
    }
}

impl<T: 'static> AsContextMut for StoreContextMut<'_, T> {
    #[inline]
    fn as_context_mut(&mut self) -> StoreContextMut<'_, T> {
        StoreContextMut(&mut *self.0)
    }
}

impl<'a, T: 'static> From<StoreContextMut<'a, T>> for StoreContext<'a, T> {
    #[inline]
    fn from(store: StoreContextMut<'a, T>) -> StoreContext<'a, T> {
        StoreContext(store.0)
    }
}

// Implementations for internal consumers, but these aren't public types so
// they're not publicly accessible for crate consumers.
impl<T: 'static> AsContext for &'_ StoreInner<T> {
    type Data = T;

    #[inline]
    fn as_context(&self) -> StoreContext<'_, T> {
        StoreContext(self)
    }
}

impl<T: 'static> AsContext for &'_ mut StoreInner<T> {
    type Data = T;

    #[inline]
    fn as_context(&self) -> StoreContext<'_, T> {
        StoreContext(self)
    }
}

impl<T: 'static> AsContextMut for &'_ mut StoreInner<T> {
    #[inline]
    fn as_context_mut(&mut self) -> StoreContextMut<'_, T> {
        StoreContextMut(&mut **self)
    }
}

// forward AsContext for &T
impl<T: AsContext> AsContext for &'_ T {
    type Data = T::Data;

    #[inline]
    fn as_context(&self) -> StoreContext<'_, T::Data> {
        T::as_context(*self)
    }
}

// forward AsContext for &mut T
impl<T: AsContext> AsContext for &'_ mut T {
    type Data = T::Data;

    #[inline]
    fn as_context(&self) -> StoreContext<'_, T::Data> {
        T::as_context(*self)
    }
}

// forward AsContextMut for &mut T
impl<T: AsContextMut> AsContextMut for &'_ mut T {
    #[inline]
    fn as_context_mut(&mut self) -> StoreContextMut<'_, T::Data> {
        T::as_context_mut(*self)
    }
}

//
impl<'a, T: AsContext> From<&'a T> for StoreContext<'a, T::Data> {
    fn from(t: &'a T) -> StoreContext<'a, T::Data> {
        t.as_context()
    }
}

impl<'a, T: AsContext> From<&'a mut T> for StoreContext<'a, T::Data> {
    fn from(t: &'a mut T) -> StoreContext<'a, T::Data> {
        T::as_context(t)
    }
}

impl<'a, T: AsContextMut> From<&'a mut T> for StoreContextMut<'a, T::Data> {
    fn from(t: &'a mut T) -> StoreContextMut<'a, T::Data> {
        t.as_context_mut()
    }
}

/// State yielded to and epoch deadline callback or a call hook.
///
/// This structure is similar to [`StoreContextMut`] except that it does not
/// implement [`AsContextMut`] and instead only implements [`AsContext`]. This
/// means that the store can be inspected and read but not mutated. This
/// structure does allow mutable access to the underlying `T` within the
/// store, however.
///
/// This type is used in APIs such as [`Store::epoch_deadline_callback`] and
/// [`Store::call_hook`] where the embedder is given access to the store at
/// points in time where it's not valid to mutate WebAssembly-facing state.
/// For example when an epoch deadline happens WebAssembly is between two
/// instructions and general state in the module cannot be mutated spec-wise.
/// This type expresses this static guarantee by ensuring that store data
/// can be read, embedder-specific data can be mutated, but nothing within the
/// store otherwise can be mutated.
pub struct StoreHookState<'a, T: 'static> {
    inner: StoreContextMut<'a, T>,
}

impl<'a, T> StoreHookState<'a, T> {
    pub(crate) fn new(inner: StoreContextMut<'a, T>) -> Self {
        Self { inner }
    }

    /// Returns a shared reference to the underlying store data.
    pub fn data(&self) -> &T {
        self.inner.data()
    }

    /// Returns a mutable reference to the underlying store data.
    pub fn data_mut(&mut self) -> &mut T {
        self.inner.data_mut()
    }
}

impl<'a, T> AsContext for StoreHookState<'a, T> {
    type Data = T;

    fn as_context(&self) -> StoreContext<'_, T> {
        self.inner.as_context()
    }
}
