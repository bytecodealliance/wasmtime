use crate::component::instance::{Instance, InstanceData};
use crate::store::{StoreOpaque, Stored};
use crate::AsContext;
use anyhow::{Context, Result};
use std::mem::MaybeUninit;
use std::ptr::NonNull;
use std::sync::Arc;
use wasmtime_environ::component::{CanonicalOptions, ComponentTypes, CoreExport, TypeFuncIndex};
use wasmtime_environ::FuncIndex;
use wasmtime_runtime::{Export, ExportFunction, VMTrampoline};

const MAX_STACK_PARAMS: usize = 16;
const MAX_STACK_RESULTS: usize = 1;

/// A helper macro to safely map `MaybeUninit<T>` to `MaybeUninit<U>` where `U`
/// is a field projection within `T`.
///
/// This is intended to be invoked as:
///
/// ```ignore
/// struct MyType {
///     field: u32,
/// }
///
/// let initial: &mut MaybeUninit<MyType> = ...;
/// let field: &mut MaybeUninit<u32> = map_maybe_uninit!(initial.field);
/// ```
///
/// Note that array accesses are also supported:
///
/// ```ignore
///
/// let initial: &mut MaybeUninit<[u32; 2]> = ...;
/// let element: &mut MaybeUninit<u32> = map_maybe_uninit!(initial[1]);
/// ```
macro_rules! map_maybe_uninit {
    ($maybe_uninit:ident $($field:tt)*) => (#[allow(unused_unsafe)] unsafe {
        use crate::component::func::MaybeUninitExt;

        let m: &mut std::mem::MaybeUninit<_> = $maybe_uninit;
        // Note the usage of `addr_of_mut!` here which is an attempt to "stay
        // safe" here where we never accidentally create `&mut T` where `T` is
        // actually uninitialized, hopefully appeasing the Rust unsafe
        // guidelines gods.
        m.map(|p| std::ptr::addr_of_mut!((*p)$($field)*))
    })
}

trait MaybeUninitExt<T> {
    /// Maps `MaybeUninit<T>` to `MaybeUninit<U>` using the closure provided.
    ///
    /// Note that this is `unsafe` as there is no guarantee that `U` comes from
    /// `T`.
    unsafe fn map<U>(&mut self, f: impl FnOnce(*mut T) -> *mut U) -> &mut MaybeUninit<U>;
}

impl<T> MaybeUninitExt<T> for MaybeUninit<T> {
    unsafe fn map<U>(&mut self, f: impl FnOnce(*mut T) -> *mut U) -> &mut MaybeUninit<U> {
        let new_ptr = f(self.as_mut_ptr());
        std::mem::transmute::<*mut U, &mut MaybeUninit<U>>(new_ptr)
    }
}

mod host;
mod options;
mod typed;
pub use self::host::*;
pub use self::options::*;
pub use self::typed::*;

/// A WebAssembly component function.
//
// FIXME: write more docs here
#[derive(Copy, Clone, Debug)]
pub struct Func(Stored<FuncData>);

#[doc(hidden)]
pub struct FuncData {
    trampoline: VMTrampoline,
    export: ExportFunction,
    ty: TypeFuncIndex,
    types: Arc<ComponentTypes>,
    options: Options,
    instance: Instance,
}

impl Func {
    pub(crate) fn from_lifted_func(
        store: &mut StoreOpaque,
        instance: &Instance,
        data: &InstanceData,
        ty: TypeFuncIndex,
        func: &CoreExport<FuncIndex>,
        options: &CanonicalOptions,
    ) -> Func {
        let export = match data.lookup_export(store, func) {
            Export::Function(f) => f,
            _ => unreachable!(),
        };
        let trampoline = store.lookup_trampoline(unsafe { export.anyfunc.as_ref() });
        let memory = options
            .memory
            .map(|i| NonNull::new(data.instance().runtime_memory(i)).unwrap());
        let realloc = options.realloc.map(|i| data.instance().runtime_realloc(i));
        let options = unsafe { Options::new(store.id(), memory, realloc, options.string_encoding) };
        Func(store.store_data_mut().insert(FuncData {
            trampoline,
            export,
            options,
            ty,
            types: data.component_types().clone(),
            instance: *instance,
        }))
    }

    /// Attempt to cast this [`Func`] to a statically typed [`TypedFunc`] with
    /// the provided `Params` and `Return`.
    ///
    /// This function will perform a type-check at runtime that the [`Func`]
    /// takes `Params` as parameters and returns `Return`. If the type-check
    /// passes then a [`TypedFunc`] will be returned which can be used to invoke
    /// the function in an efficient, statically-typed, and ergonomic manner.
    ///
    /// The `Params` type parameter here is a tuple of the parameters to the
    /// function. A function which takes no arguments should use `()`, a
    /// function with one argument should use `(T,)`, etc.
    ///
    /// The `Return` type parameter is the return value of this function. A
    /// return value of `()` means that there's no return (similar to a Rust
    /// unit return) and otherwise a type `T` can be specified.
    ///
    /// Types specified here are mainly those that implement the
    /// [`ComponentValue`] trait. This trait is implemented for built-in types
    /// to Rust such as integer primitives, floats, `Option<T>`, `Result<T, E>`,
    /// strings, and `Vec<T>`. As parameters you'll be passing native Rust
    /// types.
    ///
    /// For the `Return` type parameter many types need to be wrapped in a
    /// [`Value<T>`]. For example functions which return a string should use the
    /// `Return` type parameter as `Value<String>` instead of a bare `String`.
    /// The usage of [`Value`] indicates that a type is stored in linear memory.
    //
    // FIXME: Having to remember when to use `Value<T>` vs `T` is going to trip
    // people up using this API. It's not clear, though, how to fix that.
    ///
    /// # Errors
    ///
    /// If the function does not actually take `Params` as its parameters or
    /// return `Return` then an error will be returned.
    ///
    /// # Panics
    ///
    /// This function will panic if `self` is not owned by the `store`
    /// specified.
    ///
    /// # Examples
    ///
    /// Calling a function which takes no parameters and has no return value:
    ///
    /// ```
    /// # use wasmtime::component::Func;
    /// # use wasmtime::Store;
    /// # fn foo(func: &Func, store: &mut Store<()>) -> anyhow::Result<()> {
    /// let typed = func.typed::<(), (), _>(&store)?;
    /// typed.call(store, ())?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Calling a function which takes one string parameter and returns a
    /// string:
    ///
    /// ```
    /// # use wasmtime::component::{Func, Value};
    /// # use wasmtime::Store;
    /// # fn foo(func: &Func, mut store: Store<()>) -> anyhow::Result<()> {
    /// let typed = func.typed::<(&str,), Value<String>, _>(&store)?;
    /// let ret = typed.call(&mut store, ("Hello, ",))?;
    /// let ret = ret.cursor(&store);
    /// println!("returned string was: {}", ret.to_str()?);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Calling a function which takes multiple parameters and returns a boolean:
    ///
    /// ```
    /// # use wasmtime::component::Func;
    /// # use wasmtime::Store;
    /// # fn foo(func: &Func, mut store: Store<()>) -> anyhow::Result<()> {
    /// let typed = func.typed::<(u32, Option<&str>, &[u8]), bool, _>(&store)?;
    /// let ok: bool = typed.call(&mut store, (1, Some("hello"), b"bytes!"))?;
    /// println!("return value was: {ok}");
    /// # Ok(())
    /// # }
    /// ```
    pub fn typed<Params, Return, S>(&self, store: S) -> Result<TypedFunc<Params, Return>>
    where
        Params: ComponentParams + Lower,
        Return: Lift,
        S: AsContext,
    {
        self.typecheck::<Params, Return>(store.as_context().0)?;
        unsafe { Ok(TypedFunc::new_unchecked(*self)) }
    }

    fn typecheck<Params, Return>(&self, store: &StoreOpaque) -> Result<()>
    where
        Params: ComponentParams + Lower,
        Return: Lift,
    {
        let data = &store[self.0];
        let ty = &data.types[data.ty];

        Params::typecheck_params(&ty.params, &data.types)
            .context("type mismatch with parameters")?;
        Return::typecheck(&ty.result, &data.types).context("type mismatch with result")?;

        Ok(())
    }
}
