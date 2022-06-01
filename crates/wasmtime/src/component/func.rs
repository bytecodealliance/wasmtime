use crate::component::instance::InstanceData;
use crate::store::{StoreOpaque, Stored};
use crate::{AsContext, StoreContextMut};
use anyhow::{bail, Context, Result};
use std::convert::TryFrom;
use std::sync::Arc;
use wasmtime_environ::component::{
    CanonicalOptions, ComponentTypes, CoreExport, FuncTypeIndex, StringEncoding,
};
use wasmtime_environ::FuncIndex;
use wasmtime_runtime::{Export, ExportFunction, ExportMemory, VMTrampoline};

mod typed;
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
    ty: FuncTypeIndex,
    types: Arc<ComponentTypes>,
    options: Options,
}

pub(crate) struct Options {
    string_encoding: StringEncoding,
    intrinsics: Option<Intrinsics>,
}

struct Intrinsics {
    memory: ExportMemory,
    realloc: ExportFunction,
}

impl Func {
    pub(crate) fn from_lifted_func(
        store: &mut StoreOpaque,
        instance: &InstanceData,
        ty: FuncTypeIndex,
        func: &CoreExport<FuncIndex>,
        options: &CanonicalOptions,
    ) -> Func {
        let export = match instance.lookup_export(store, func) {
            Export::Function(f) => f,
            _ => unreachable!(),
        };
        let trampoline = store.lookup_trampoline(unsafe { export.anyfunc.as_ref() });
        let intrinsics = options.memory.map(|i| {
            let memory = instance.runtime_memory(i);
            let realloc = instance.runtime_realloc(options.realloc.unwrap());
            Intrinsics { memory, realloc }
        });
        Func(store.store_data_mut().insert(FuncData {
            trampoline,
            export,
            options: Options {
                intrinsics,
                string_encoding: options.string_encoding,
            },
            ty,
            types: instance.component_types().clone(),
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
        Params: ComponentParams,
        Return: ComponentValue,
        S: AsContext,
    {
        self.typecheck::<Params, Return>(store.as_context().0)?;
        unsafe { Ok(TypedFunc::new_unchecked(*self)) }
    }

    fn typecheck<Params, Return>(&self, store: &StoreOpaque) -> Result<()>
    where
        Params: ComponentParams,
        Return: ComponentValue,
    {
        let data = &store[self.0];
        let ty = &data.types[data.ty];

        Params::typecheck(&ty.params, &data.types).context("type mismatch with parameters")?;
        Return::typecheck(&ty.result, &data.types, Op::Lift)
            .context("type mismatch with result")?;

        Ok(())
    }

    fn realloc<'a, T>(
        &self,
        store: &'a mut StoreContextMut<'_, T>,
        old: usize,
        old_size: usize,
        old_align: u32,
        new_size: usize,
    ) -> Result<(&'a mut [u8], usize)> {
        let (realloc, memory) = match &store.0[self.0].options.intrinsics {
            Some(Intrinsics {
                memory, realloc, ..
            }) => (realloc.clone(), memory.clone()),
            None => unreachable!(),
        };

        // Invoke the wasm malloc function using its raw and statically known
        // signature.
        let result = unsafe {
            // FIXME: needs memory64 support
            assert!(!memory.memory.memory.memory64);
            usize::try_from(crate::TypedFunc::<(u32, u32, u32, u32), u32>::call_raw(
                store,
                realloc.anyfunc,
                (
                    u32::try_from(old)?,
                    u32::try_from(old_size)?,
                    old_align,
                    u32::try_from(new_size)?,
                ),
            )?)?
        };

        let memory = self.memory_mut(store.0);

        let result_slice = match memory.get_mut(result..).and_then(|s| s.get_mut(..new_size)) {
            Some(end) => end,
            None => bail!("realloc return: beyond end of memory"),
        };

        Ok((result_slice, result))
    }

    /// Asserts that this function has an associated memory attached to it and
    /// then returns the slice of memory tied to the lifetime of the provided
    /// store.
    fn memory<'a>(&self, store: &'a StoreOpaque) -> &'a [u8] {
        let memory = match &store[self.0].options.intrinsics {
            Some(Intrinsics { memory, .. }) => memory,
            None => unreachable!(),
        };

        unsafe {
            let memory = &*memory.definition;
            std::slice::from_raw_parts(memory.base, memory.current_length)
        }
    }

    /// Same as above, just `_mut`
    fn memory_mut<'a>(&self, store: &'a mut StoreOpaque) -> &'a mut [u8] {
        let memory = match &store[self.0].options.intrinsics {
            Some(Intrinsics { memory, .. }) => memory.clone(),
            None => unreachable!(),
        };

        unsafe {
            let memory = &*memory.definition;
            std::slice::from_raw_parts_mut(memory.base, memory.current_length)
        }
    }
}
