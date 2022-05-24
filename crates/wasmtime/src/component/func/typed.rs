use crate::component::Func;
use crate::store::StoreOpaque;
use crate::{AsContextMut, StoreContextMut, ValRaw};
use anyhow::{bail, Result};
use std::borrow::Cow;
use std::convert::Infallible;
use std::marker;
use std::mem::{self, MaybeUninit};
use std::str;
use wasmtime_environ::component::{ComponentTypes, InterfaceType, StringEncoding};

const MAX_STACK_PARAMS: usize = 16;
const MAX_STACK_RESULTS: usize = 1;
const UTF16_TAG: usize = 1 << 31;

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
        let m: &mut MaybeUninit<_> = $maybe_uninit;
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
        mem::transmute::<*mut U, &mut MaybeUninit<U>>(new_ptr)
    }
}

/// A statically-typed version of [`Func`] which takes `Params` as input and
/// returns `Return`.
///
/// This is an efficient way to invoke a WebAssembly component where if the
/// inputs and output are statically known this can eschew the vast majority of
/// machinery and checks when calling WebAssembly. This is the most optimized
/// way to call a WebAssembly component.
///
/// Note that like [`Func`] this is a pointer within a [`Store`](crate::Store)
/// and usage will panic if used with the wrong store.
///
/// This type is primarily created with the [`Func::typed`] API.
pub struct TypedFunc<Params, Return> {
    func: Func,

    // The definition of this field is somewhat subtle and may be surprising.
    // Naively one might expect something like
    //
    //      _marker: marker::PhantomData<fn(Params) -> Return>,
    //
    // Since this is a function pointer after all. The problem with this
    // definition though is that it imposes the wrong variance on `Params` from
    // what we want. Abstractly a `fn(Params)` is able to store `Params` within
    // it meaning you can only give it `Params` that live longer than the
    // function pointer.
    //
    // With a component model function, however, we're always copying data from
    // the host into the guest, so we are never storing pointers to `Params`
    // into the guest outside the duration of a `call`, meaning we can actually
    // accept values in `TypedFunc::call` which live for a shorter duration
    // than the `Params` argument on the struct.
    //
    // This all means that we don't use a phantom function pointer, but instead
    // feign phantom storage here to get the variance desired.
    _marker: marker::PhantomData<(Params, Return)>,
}

impl<Params, Return> Copy for TypedFunc<Params, Return> {}

impl<Params, Return> Clone for TypedFunc<Params, Return> {
    fn clone(&self) -> TypedFunc<Params, Return> {
        *self
    }
}

impl<Params, Return> TypedFunc<Params, Return>
where
    Params: ComponentParams,
    Return: ComponentReturn,
{
    /// Creates a new [`TypedFunc`] from the provided component [`Func`],
    /// unsafely asserting that the underlying function takes `Params` as
    /// input and returns `Return`.
    ///
    /// # Unsafety
    ///
    /// This is an unsafe function because it does not verify that the [`Func`]
    /// provided actually implements this signature. It's up to the caller to
    /// have performed some other sort of check to ensure that the signature is
    /// correct.
    pub unsafe fn new_unchecked(func: Func) -> TypedFunc<Params, Return> {
        TypedFunc {
            _marker: marker::PhantomData,
            func,
        }
    }

    /// Returns the underlying un-typed [`Func`] that this [`TypedFunc`]
    /// references.
    pub fn func(&self) -> &Func {
        &self.func
    }

    /// Calls the underlying WebAssembly component function using the provided
    /// `params` as input.
    ///
    /// This method is used to enter into a component. Execution happens within
    /// the `store` provided. The `params` are copied into WebAssembly memory
    /// as appropriate and a core wasm function is invoked.
    ///
    /// # Errors
    ///
    /// This function can return an error for a number of reasons:
    ///
    /// * If the wasm itself traps during execution.
    /// * If the wasm traps while copying arguments into memory.
    /// * If the wasm provides bad allocation pointers when copying arguments
    ///   into memory.
    /// * If the wasm returns a value which violates the canonical ABI.
    ///
    /// In general there are many ways that things could go wrong when copying
    /// types in and out of a wasm module with the canonical ABI, and certain
    /// error conditions are specific to certain types. For example a
    /// WebAssembly module can't return an invalid `char`. When allocating space
    /// for this host to copy a string into the returned pointer must be
    /// in-bounds in memory.
    ///
    /// If an error happens then the error should contain detailed enough
    /// information to understand which part of the canonical ABI went wrong
    /// and what to inspect.
    ///
    /// # Panics
    ///
    /// This function will panic if `store` does not own this function.
    pub fn call(&self, mut store: impl AsContextMut, params: Params) -> Result<Return> {
        let mut store = store.as_context_mut();
        if <Params::AsTuple as ComponentValue>::flatten_count() <= MAX_STACK_PARAMS {
            self.call_stack_args(&mut store, &params)
        } else {
            self.call_heap_args(&mut store, &params)
        }
    }

    fn call_stack_args<T>(
        &self,
        store: &mut StoreContextMut<'_, T>,
        params: &Params,
    ) -> Result<Return> {
        // Create storage space for both the parameters and the results (stored
        // on top of one another), and initially have it all uninitialized.
        let params_and_results = &mut MaybeUninit::<
            ParamsAndResults<<Params::AsTuple as ComponentValue>::Lower, Return::Lower>,
        >::uninit();

        // In debug assertions mode start with an arbitrary bit-pattern which
        // should be overwritten for anything actually read by the wasm
        // trampoline we'll call later.
        if cfg!(debug_assertions) {
            unsafe {
                const CANON_ABI_UNINIT_PATTERN: u8 = 0xAB;
                params_and_results
                    .as_mut_ptr()
                    .write_bytes(CANON_ABI_UNINIT_PATTERN, 1);
            }
        }

        // Perform the lowering operation for the parameters which will write
        // all of the parameters to the stack. This stack buffer is then passed
        // to core wasm as `*mut ValRaw` which will read the values from the
        // stack and later store the results here as well.
        params.lower(
            store,
            &self.func,
            map_maybe_uninit!(params_and_results.params),
        )?;

        self.call_raw(store, params_and_results)
    }

    fn call_heap_args<T>(
        &self,
        store: &mut StoreContextMut<'_, T>,
        params: &Params,
    ) -> Result<Return> {
        // Memory must exist via validation if the arguments are stored on the
        // heap, so we can create a `Memory` at this point. Afterwards `realloc`
        // is used to allocate space for all the arguments and then they're all
        // stored in linear memory.
        let mut memory = Memory::new(store.as_context_mut(), &self.func);
        let ptr = memory.realloc(0, 0, Params::align(), Params::size())?;
        params.store(&mut memory, ptr)?;

        // Space for the parameters and results are created on the stack here.
        // Note that the parameter here is a single `ValRaw` since the function
        // will only have one parameter which is a pointer into the heap where
        // all of the arguments are stored. The space for the results is
        // reserved by the other field of the union of `ParamsAndResults`.
        //
        // Also note that the pointer here is stored as a 64-bit integer. This
        // allows this to work with either 32 or 64-bit memories. For a 32-bit
        // memory it'll just ignore the upper 32 zero bits, and for 64-bit
        // memories this'll have the full 64-bits. Note that for 32-bit
        // memories the call to `realloc` above guarantees that the `ptr` is
        // in-bounds meaning that we will know that the zero-extended upper
        // bits of `ptr` are guaranteed to be zero.
        //
        // This comment about 64-bit integers is also referred to below with
        // "WRITEPTR64".
        let params_and_results = &mut MaybeUninit::new(ParamsAndResults {
            params: ValRaw {
                i64: (ptr as i64).to_le(),
            },
        });

        self.call_raw(store, params_and_results)
    }

    fn call_raw<T, U>(
        &self,
        store: &mut StoreContextMut<'_, T>,
        space: &mut MaybeUninit<ParamsAndResults<U, Return::Lower>>,
    ) -> Result<Return>
    where
        U: Copy,
    {
        let super::FuncData {
            trampoline, export, ..
        } = store.0[self.func.0];

        // Double-check the size/alignemnt of `space`, just in case.
        //
        // Note that this alone is not enough to guarantee the validity of the
        // `unsafe` block below, but it's definitely required. In any case LLVM
        // should be able to trivially see through these assertions and remove
        // them in release mode.
        let val_size = mem::size_of::<ValRaw>();
        let val_align = mem::align_of::<ValRaw>();
        assert!(mem::size_of_val(space) % val_size == 0);
        assert!(mem::size_of_val(map_maybe_uninit!(space.params)) % val_size == 0);
        assert!(mem::size_of_val(map_maybe_uninit!(space.ret)) % val_size == 0);
        assert!(mem::align_of_val(space) == val_align);
        assert!(mem::align_of_val(map_maybe_uninit!(space.params)) == val_align);
        assert!(mem::align_of_val(map_maybe_uninit!(space.ret)) == val_align);

        unsafe {
            // This is unsafe as we are providing the guarantee that all the
            // inputs are valid. The various pointers passed in for the function
            // are all valid since they're coming from our store, and the
            // `params_and_results` should have the correct layout for the core
            // wasm function we're calling. Note that this latter point relies
            // on the correctness of this module and `ComponentValue`
            // implementations, hence `ComponentValue` being an `unsafe` trait.
            crate::Func::call_unchecked_raw(
                store,
                export.anyfunc,
                trampoline,
                space.as_mut_ptr().cast(),
            )?;

            // Note that `.assume_init_ref()` here is unsafe but we're relying
            // on the correctness of the structure of `params_and_results`, the
            // structure of `Return::Lower`, and the type-checking performed to
            // acquire the `TypedFunc` to make this safe. It should be the case
            // that `Return::Lower` is the exact representation of the return
            // value when interpreted as `[ValRaw]`, and additionally they
            // should have the correct types for the function we just called
            // (which filled in the return values).
            Return::lift(
                store.0,
                &self.func,
                map_maybe_uninit!(space.ret).assume_init_ref(),
            )
        }
    }
}

#[repr(C)]
union ParamsAndResults<Params: Copy, Return: Copy> {
    params: Params,
    ret: Return,
}

/// A trait representing a static list of parameters that can be passed to a
/// [`TypedFunc`].
///
/// This trait is implemented for a number of tuple types and is not expected
/// to be implemented externally. The contents of this trait are hidden as it's
/// intended to be an implementation detail of Wasmtime. The contents of this
/// trait are not covered by Wasmtime's stability guarantees.
///
/// For more information about this trait see [`Func::typed`] and
/// [`TypedFunc`].
//
// Note that this is an `unsafe` trait, and the unsafety means that
// implementations of this trait must be correct or otherwise [`TypedFunc`]
// would not be memory safe. The main reason this is `unsafe` is the
// `typecheck` function which must operate correctly relative to the `AsTuple`
// interpretation of the implementor.
pub unsafe trait ComponentParams {
    /// The tuple type corresponding to this list of parameters if this list is
    /// interpreted as a tuple in the canonical ABI.
    #[doc(hidden)]
    type AsTuple: ComponentValue;

    /// Performs a typecheck to ensure that this `ComponentParams` implementor
    /// matches the types of the types in `params`.
    #[doc(hidden)]
    fn typecheck(params: &[(Option<String>, InterfaceType)], types: &ComponentTypes) -> Result<()>;

    /// Views this instance of `ComponentParams` as a tuple, allowing
    /// delegation to all of the methods in `ComponentValue`.
    #[doc(hidden)]
    fn as_tuple(&self) -> &Self::AsTuple;

    /// Convenience method to `ComponentValue::lower` when viewing this
    /// parameter list as a tuple.
    #[doc(hidden)]
    fn lower<T>(
        &self,
        store: &mut StoreContextMut<T>,
        func: &Func,
        dst: &mut MaybeUninit<<Self::AsTuple as ComponentValue>::Lower>,
    ) -> Result<()> {
        self.as_tuple().lower(store, func, dst)
    }

    /// Convenience method to `ComponentValue::store` when viewing this
    /// parameter list as a tuple.
    #[doc(hidden)]
    fn store<T>(&self, memory: &mut Memory<'_, T>, offset: usize) -> Result<()> {
        self.as_tuple().store(memory, offset)
    }

    /// Convenience function to return the canonical abi alignment of this list
    /// of parameters when viewed as a tuple.
    #[doc(hidden)]
    #[inline]
    fn align() -> u32 {
        Self::AsTuple::align()
    }

    /// Convenience function to return the canonical abi byte size of this list
    /// of parameters when viewed as a tuple.
    #[doc(hidden)]
    #[inline]
    fn size() -> usize {
        Self::AsTuple::size()
    }
}

// Macro to generate an implementation of `ComponentParams` for all supported
// lengths of tuples of types in Wasmtime.
macro_rules! impl_component_params {
    ($n:tt $($t:ident)*) => {paste::paste!{
        #[allow(non_snake_case)]
        unsafe impl<$($t,)*> ComponentParams for ($($t,)*) where $($t: ComponentValue),* {
            type AsTuple = ($($t,)*);

            fn typecheck(
                params: &[(Option<String>, InterfaceType)],
                _types: &ComponentTypes,
            ) -> Result<()> {
                if params.len() != $n {
                    bail!("expected {} types, found {}", $n, params.len());
                }
                let mut params = params.iter().map(|i| &i.1);
                $($t::typecheck(params.next().unwrap(), _types)?;)*
                debug_assert!(params.next().is_none());
                Ok(())
            }

            #[inline]
            fn as_tuple(&self) -> &Self::AsTuple {
                self
            }
        }
    }};
}

for_each_function_signature!(impl_component_params);

/// A trait representing types which can be passed to and read from components
/// with the canonical ABI.
///
/// This trait is implemented for Rust types which can be communicated to
/// components. This is implemented for Rust types which correspond to
/// interface types in the component model of WebAssembly. The [`Func::typed`]
/// and [`TypedFunc`] Rust items are the main consumers of this trait.
///
/// For more information on this trait see the examples in [`Func::typed`].
///
/// The contents of this trait are hidden as it's intended to be an
/// implementation detail of Wasmtime. The contents of this trait are not
/// covered by Wasmtime's stability guarantees.
//
// Note that this is an `unsafe` trait as `TypedFunc`'s safety heavily relies on
// the correctness of the implementations of this trait. Some ways in which this
// trait must be correct to be safe are:
//
// * The `Lower` associated type must be a `ValRaw` sequence. It doesn't have to
//   literally be `[ValRaw; N]` but when laid out in memory it must be adjacent
//   `ValRaw` values and have a multiple of the size of `ValRaw` and the same
//   alignment.
//
// * The `lower` function must initialize the bits within `Lower` that are going
//   to be read by the trampoline that's used to enter core wasm. A trampoline
//   is passed `*mut Lower` and will read the canonical abi arguments in
//   sequence, so all of the bits must be correctly initialized.
//
// * The `size` and `align` functions must be correct for this value stored in
//   the canonical ABI. The `Cursor<T>` iteration of these bytes rely on this
//   for correctness as they otherwise eschew bounds-checking.
//
// There are likely some other correctness issues which aren't documented as
// well, this isn't intended to be an exhaustive list. It suffices to say,
// though, that correctness bugs in this trait implementation are highly likely
// to lead to security bugs, which again leads to the `unsafe` in the trait.
//
// Also note that this trait specifically is not sealed because we'll
// eventually have a proc macro that generates implementations of this trait
// for external types in a `#[derive]`-like fashion.
//
// FIXME: need to write a #[derive(ComponentValue)]
pub unsafe trait ComponentValue {
    /// Representation of the "lowered" form of this component value.
    ///
    /// Lowerings lower into core wasm values which are represented by `ValRaw`.
    /// This `Lower` type must be a list of `ValRaw` as either a literal array
    /// or a struct where every field is a `ValRaw`. This must be `Copy` (as
    /// `ValRaw` is `Copy`) and support all byte patterns. This being correct is
    /// one reason why the trait is unsafe.
    #[doc(hidden)]
    type Lower: Copy;

    /// Representation of the "lifted" form of this component value.
    ///
    /// This is somewhat subtle and is not always what you might expect. This is
    /// only used for values which are actually possible to return by-value in
    /// the canonical ABI. Everything returned indirectly (e.g. takes up two or
    /// more core wasm values to represent) is instead returned as `Value<T>`
    /// and this associated type isn't used.
    ///
    /// For that reason this `Lift` is defined as `Self` for most primitives,
    /// but it's actually `Infallible` (some empty void-like enum) for
    /// strings/lists because those aren't possible to lift from core wasm
    /// values.
    ///
    /// This is also used for ADT-definitions of tuples/options/results since
    /// it's technically possible to return `(u32,)` or something like
    /// `option<()>` which is all an immediate return value as well. In general
    /// this is expected to largely be `Infallible` (or similar) and functions
    /// return `Value<T>` instead at the `TypedFunc` layer.
    #[doc(hidden)]
    type Lift;

    /// Performs a type-check to see whether this comopnent value type matches
    /// the interface type `ty` provided.
    #[doc(hidden)]
    fn typecheck(ty: &InterfaceType, types: &ComponentTypes) -> Result<()>;

    /// Performs the "lower" function in the canonical ABI.
    ///
    /// This method will lower the given value into wasm linear memory. The
    /// `store` and `func` are provided in case memory is needed (e.g. for
    /// strings/lists) so `realloc` can be called. The `dst` is the destination
    /// to store the lowered results.
    ///
    /// Note that `dst` is a pointer to uninitialized memory. It's expected
    /// that `dst` is fully initialized by the time this function returns, hence
    /// the `unsafe` on the trait implementation.
    #[doc(hidden)]
    fn lower<T>(
        &self,
        store: &mut StoreContextMut<T>,
        func: &Func,
        dst: &mut MaybeUninit<Self::Lower>,
    ) -> Result<()>;

    /// Returns the size, in bytes, that this type has in the canonical ABI.
    ///
    /// Note that it's expected that this function is "simple" to be easily
    /// optimizable by LLVM (e.g. inlined and const-evaluated).
    //
    // FIXME: needs some sort of parameter indicating the memory size
    #[doc(hidden)]
    fn size() -> usize;

    /// Returns the alignment, in bytes, that this type has in the canonical
    /// ABI.
    ///
    /// Note that it's expected that this function is "simple" to be easily
    /// optimizable by LLVM (e.g. inlined and const-evaluated).
    #[doc(hidden)]
    fn align() -> u32;

    /// Performs the "store" operation in the canonical ABI.
    ///
    /// This function will store `self` into the linear memory described by
    /// `memory` at the `offset` provided.
    ///
    /// It is expected that `offset` is a valid offset in memory for
    /// `Self::size()` bytes. At this time that's not an unsafe contract as it's
    /// always re-checked on all stores, but this is something that will need to
    /// be improved in the future to remove extra bounds checks. For now this
    /// function will panic if there's a bug and `offset` isn't valid within
    /// memory.
    #[doc(hidden)]
    fn store<T>(&self, memory: &mut Memory<'_, T>, offset: usize) -> Result<()>;

    /// Returns the number of core wasm abi values will be used to represent
    /// this type in its lowered form.
    ///
    /// This divides the size of `Self::Lower` by the size of `ValRaw`.
    #[doc(hidden)]
    fn flatten_count() -> usize {
        assert!(mem::size_of::<Self::Lower>() % mem::size_of::<ValRaw>() == 0);
        assert!(mem::align_of::<Self::Lower>() == mem::align_of::<ValRaw>());
        mem::size_of::<Self::Lower>() / mem::size_of::<ValRaw>()
    }

    /// Performs the "lift" oepration in the canonical ABI.
    ///
    /// Like `Self::Lift` this is somewhat special, it's actually only ever
    /// called if `Self::Lower` is zero or one `ValRaw` instances. If the
    /// lowered representation of this type needs more instances of `ValRaw`
    /// then the value is always returned through memory which means a `Cursor`
    /// is instead used to iterate over the contents.
    ///
    /// This takes the lowered representation as input and returns the
    /// associated `Lift` type for this implementation. For types where `Lift`
    /// is `Infallible` or similar this simply panics as it should never be
    /// called at runtime.
    #[doc(hidden)]
    fn lift(src: &Self::Lower) -> Result<Self::Lift>;
}

/// A helper structure to package up proof-of-memory. This holds a store pointer
/// and a `Func` pointer where the function has the pointers to memory.
///
/// Note that one of the purposes of this type is to make `lower_list`
/// vectorizable by "caching" the last view of memory. CUrrently it doesn't do
/// that, though, because I couldn't get `lower_list::<u8>` to vectorize. I've
/// left this in for convenience in the hope that this can be updated in the
/// future.
#[doc(hidden)]
pub struct Memory<'a, T> {
    store: StoreContextMut<'a, T>,
    func: &'a Func,
}

impl<'a, T> Memory<'a, T> {
    fn new(store: StoreContextMut<'a, T>, func: &'a Func) -> Memory<'a, T> {
        Memory { func, store }
    }

    #[inline]
    fn string_encoding(&self) -> StringEncoding {
        self.store.0[self.func.0].options.string_encoding
    }

    #[inline]
    fn memory(&mut self) -> &mut [u8] {
        self.func.memory_mut(self.store.0)
    }

    fn realloc(
        &mut self,
        old: usize,
        old_size: usize,
        old_align: u32,
        new_size: usize,
    ) -> Result<usize> {
        let ret = self
            .func
            .realloc(&mut self.store, old, old_size, old_align, new_size)
            .map(|(_, ptr)| ptr);
        return ret;
    }

    fn get<const N: usize>(&mut self, offset: usize) -> &mut [u8; N] {
        // FIXME: this bounds check shouldn't actually be necessary, all
        // callers of `ComponentValue::store` have already performed a bounds
        // check so we're guaranteed that `offset..offset+N` is in-bounds. That
        // being said we at least should do bounds checks in debug mode and
        // it's not clear to me how to easily structure this so that it's
        // "statically obvious" the bounds check isn't necessary.
        //
        // For now I figure we can leave in this bounds check and if it becomes
        // an issue we can optimize further later, probably with judicious use
        // of `unsafe`.
        (&mut self.memory()[offset..][..N]).try_into().unwrap()
    }
}

// Macro to help generate "forwarding implementations" of `ComponentValue` to
// another type, used for wrappers in Rust like `&T`, `Box<T>`, etc.
macro_rules! forward_component_param {
    ($(($($generics:tt)*) $a:ty => $b:ty,)*) => ($(
        unsafe impl <$($generics)*> ComponentValue for $a {
            type Lower = <$b as ComponentValue>::Lower;
            type Lift = <$b as ComponentValue>::Lift;

            #[inline]
            fn typecheck(ty: &InterfaceType, types: &ComponentTypes) -> Result<()> {
                <$b as ComponentValue>::typecheck(ty, types)
            }

            fn lower<U>(
                &self,
                store: &mut StoreContextMut<U>,
                func: &Func,
                dst: &mut MaybeUninit<Self::Lower>,
            ) -> Result<()> {
                <$b as ComponentValue>::lower(self, store, func, dst)
            }

            #[inline]
            fn size() -> usize {
                <$b as ComponentValue>::size()
            }

            #[inline]
            fn align() -> u32 {
                <$b as ComponentValue>::align()
            }

            fn store<U>(&self, memory: &mut Memory<'_, U>, offset: usize) -> Result<()> {
                <$b as ComponentValue>::store(self, memory, offset)
            }

            fn lift(src: &Self::Lower) -> Result<Self::Lift> {
                <$b as ComponentValue>::lift(src)
            }
        }
    )*)
}

forward_component_param! {
    (T: ComponentValue + ?Sized) &'_ T => T,
    (T: ComponentValue + ?Sized) Box<T> => T,
    (T: ComponentValue + ?Sized) std::rc::Rc<T> => T,
    (T: ComponentValue + ?Sized) std::sync::Arc<T> => T,
    () String => str,
    (T: ComponentValue) Vec<T> => [T],
}

unsafe impl ComponentValue for () {
    // A 0-sized array is used here to represent that it has zero-size but it
    // still has the alignment of `ValRaw`.
    type Lower = [ValRaw; 0];
    type Lift = ();

    fn typecheck(ty: &InterfaceType, _types: &ComponentTypes) -> Result<()> {
        match ty {
            // FIXME(WebAssembly/component-model#21) this may either want to
            // match more types, not actually exist as a trait impl, or
            // something like that. Figuring out on that issue about the
            // relationship between the 0-tuple, unit, and empty structs.
            InterfaceType::Unit => Ok(()),
            other => bail!("expected `unit` found `{}`", desc(other)),
        }
    }

    #[inline]
    fn lower<T>(
        &self,
        _store: &mut StoreContextMut<T>,
        _func: &Func,
        _dst: &mut MaybeUninit<Self::Lower>,
    ) -> Result<()> {
        Ok(())
    }

    #[inline]
    fn size() -> usize {
        0
    }

    #[inline]
    fn align() -> u32 {
        1
    }

    #[inline]
    fn store<T>(&self, _memory: &mut Memory<'_, T>, _offset: usize) -> Result<()> {
        Ok(())
    }

    #[inline]
    fn lift(_src: &Self::Lower) -> Result<()> {
        Ok(())
    }
}

// Macro to help generate `ComponentValue` implementations for primitive types
// such as integers, char, bool, etc.
macro_rules! integers {
    ($($primitive:ident = $ty:ident in $field:ident $(as $unsigned:ident)?,)*) => ($(
        unsafe impl ComponentValue for $primitive {
            type Lower = ValRaw;
            type Lift = $primitive;

            fn typecheck(ty: &InterfaceType, _types: &ComponentTypes) -> Result<()> {
                match ty {
                    InterfaceType::$ty => Ok(()),
                    other => bail!("expected `{}` found `{}`", desc(&InterfaceType::$ty), desc(other))
                }
            }

            fn lower<T>(
                &self,
                _store: &mut StoreContextMut<T>,
                _func: &Func,
                dst: &mut MaybeUninit<Self::Lower>,
            ) -> Result<()> {
                map_maybe_uninit!(dst.$field)
                    .write((*self $(as $unsigned)? as $field).to_le());
                Ok(())
            }

            #[inline]
            fn size() -> usize { mem::size_of::<$primitive>() }

            // Note that this specifically doesn't use `align_of` as some
            // host platforms have a 4-byte alignment for primitive types but
            // the canonical abi always has the same size/alignment for these
            // types.
            #[inline]
            fn align() -> u32 { mem::size_of::<$primitive>() as u32 }

            fn store<T>(&self, memory: &mut Memory<'_, T>, offset: usize) -> Result<()> {
                *memory.get(offset) = self.to_le_bytes();
                Ok(())
            }

            #[inline]
            fn lift(src: &Self::Lower) -> Result<Self::Lift> {
                // Convert from little-endian and then view the signed storage
                // as an optionally-unsigned type.
                let field = unsafe {
                    $field::from_le(src.$field) $(as $unsigned)?
                };

                // Perform a lossless cast from our field storage to the
                // destination type. Note that `try_from` here is load bearing
                // which rejects conversions like `500u32` to `u8` because
                // that's out-of-bounds for `u8`.
                Ok($primitive::try_from(field)?)
            }
        }

        impl Cursor<'_, $primitive> {
            /// Returns the underlying value that this cursor points to.
            #[inline]
            pub fn get(&self) -> $primitive {
                $primitive::from_le_bytes(self.item_bytes().try_into().unwrap())
            }
        }
    )*)
}

integers! {
    i8 = S8 in i32,
    u8 = U8 in i32 as u32,
    i16 = S16 in i32,
    u16 = U16 in i32 as u32,
    i32 = S32 in i32,
    u32 = U32 in i32 as u32,
    i64 = S64 in i64,
    u64 = U64 in i64 as u64,
}

macro_rules! floats {
    ($($float:ident/$storage:ident = $ty:ident)*) => ($(const _: () = {
        /// All floats in-and-out of the canonical ABI always have their NaN
        /// payloads canonicalized. Conveniently the `NAN` constant in Rust has
        /// the same representation as canonical NAN, so we can use that for the
        /// NAN value.
        #[inline]
        fn canonicalize(float: $float) -> $float {
            if float.is_nan() {
                $float::NAN
            } else {
                float
            }
        }

        unsafe impl ComponentValue for $float {
            type Lower = ValRaw;
            type Lift = $float;

            fn typecheck(ty: &InterfaceType, _types: &ComponentTypes) -> Result<()> {
                match ty {
                    InterfaceType::$ty => Ok(()),
                    other => bail!("expected `{}` found `{}`", desc(&InterfaceType::$ty), desc(other))
                }
            }

            fn lower<T>(
                &self,
                _store: &mut StoreContextMut<T>,
                _func: &Func,
                dst: &mut MaybeUninit<Self::Lower>,
            ) -> Result<()> {
                map_maybe_uninit!(dst.$float)
                    .write(canonicalize(*self).to_bits().to_le());
                Ok(())
            }

            #[inline]
            fn size() -> usize { mem::size_of::<$float>() }

            // Note that like integers size is used here instead of alignment to
            // respect the canonical ABI, not host platforms.
            #[inline]
            fn align() -> u32 { mem::size_of::<$float>() as u32 }

            fn store<T>(&self, memory: &mut Memory<'_, T>, offset: usize) -> Result<()> {
                let ptr = memory.get(offset);
                *ptr = canonicalize(*self).to_bits().to_le_bytes();
                Ok(())
            }

            #[inline]
            fn lift(src: &Self::Lower) -> Result<Self::Lift> {
                let field = $storage::from_le(unsafe { src.$float });
                Ok(canonicalize($float::from_bits(field)))
            }
        }

        impl Cursor<'_, $float> {
            /// Returns the underlying value that this cursor points to.
            ///
            /// Note that NaN values in the component model are canonicalized
            /// so any NaN read is guaranteed to be a "canonical NaN".
            #[inline]
            pub fn get(&self) -> $float {
                canonicalize($float::from_le_bytes(self.item_bytes().try_into().unwrap()))
            }
        }
    };)*)
}

floats! {
    f32/u32 = Float32
    f64/u64 = Float64
}

unsafe impl ComponentValue for bool {
    type Lower = ValRaw;
    type Lift = bool;

    fn typecheck(ty: &InterfaceType, _types: &ComponentTypes) -> Result<()> {
        match ty {
            InterfaceType::Bool => Ok(()),
            other => bail!("expected `bool` found `{}`", desc(other)),
        }
    }

    fn lower<T>(
        &self,
        _store: &mut StoreContextMut<T>,
        _func: &Func,
        dst: &mut MaybeUninit<Self::Lower>,
    ) -> Result<()> {
        map_maybe_uninit!(dst.i32).write((*self as i32).to_le());
        Ok(())
    }

    #[inline]
    fn size() -> usize {
        1
    }

    #[inline]
    fn align() -> u32 {
        1
    }

    fn store<T>(&self, memory: &mut Memory<'_, T>, offset: usize) -> Result<()> {
        memory.get::<1>(offset)[0] = *self as u8;
        Ok(())
    }

    #[inline]
    fn lift(src: &Self::Lower) -> Result<Self::Lift> {
        match i32::from_le(unsafe { src.i32 }) {
            0 => Ok(false),
            1 => Ok(true),
            _ => bail!("invalid boolean value"),
        }
    }
}

impl Cursor<'_, bool> {
    /// Returns the underlying value that this cursor points to.
    ///
    /// # Errors
    ///
    /// Returns an error if the wasm memory does not have the boolean stored in
    /// the correct canonical ABI format.
    #[inline]
    pub fn get(&self) -> Result<bool> {
        match self.item_bytes()[0] {
            0 => Ok(false),
            1 => Ok(true),
            _ => bail!("invalid boolean value"),
        }
    }
}

unsafe impl ComponentValue for char {
    type Lower = ValRaw;
    type Lift = char;

    fn typecheck(ty: &InterfaceType, _types: &ComponentTypes) -> Result<()> {
        match ty {
            InterfaceType::Char => Ok(()),
            other => bail!("expected `char` found `{}`", desc(other)),
        }
    }

    fn lower<T>(
        &self,
        _store: &mut StoreContextMut<T>,
        _func: &Func,
        dst: &mut MaybeUninit<Self::Lower>,
    ) -> Result<()> {
        map_maybe_uninit!(dst.i32).write((u32::from(*self) as i32).to_le());
        Ok(())
    }

    #[inline]
    fn size() -> usize {
        4
    }

    #[inline]
    fn align() -> u32 {
        4
    }

    fn store<T>(&self, memory: &mut Memory<'_, T>, offset: usize) -> Result<()> {
        *memory.get::<4>(offset) = u32::from(*self).to_le_bytes();
        Ok(())
    }

    #[inline]
    fn lift(src: &Self::Lower) -> Result<Self::Lift> {
        let bits = i32::from_le(unsafe { src.i32 }) as u32;
        Ok(char::try_from(bits)?)
    }
}

impl Cursor<'_, char> {
    /// Returns the underlying value that this cursor points to.
    ///
    /// # Errors
    ///
    /// Returns an error if the wasm memory does not have the char stored in
    /// the correct canonical ABI format (e.g it's an invalid char)
    #[inline]
    pub fn get(&self) -> Result<char> {
        let bits = u32::from_le_bytes(self.item_bytes().try_into().unwrap());
        Ok(char::try_from(bits)?)
    }
}

unsafe impl ComponentValue for str {
    type Lower = [ValRaw; 2];
    type Lift = Infallible;

    fn typecheck(ty: &InterfaceType, _types: &ComponentTypes) -> Result<()> {
        match ty {
            InterfaceType::String => Ok(()),
            other => bail!("expected `string` found `{}`", desc(other)),
        }
    }

    fn lower<T>(
        &self,
        store: &mut StoreContextMut<T>,
        func: &Func,
        dst: &mut MaybeUninit<[ValRaw; 2]>,
    ) -> Result<()> {
        let (ptr, len) = lower_string(&mut Memory::new(store.as_context_mut(), func), self)?;
        // See "WRITEPTR64" above for why this is always storing a 64-bit
        // integer.
        map_maybe_uninit!(dst[0].i64).write((ptr as i64).to_le());
        map_maybe_uninit!(dst[1].i64).write((len as i64).to_le());
        Ok(())
    }

    fn size() -> usize {
        8
    }

    fn align() -> u32 {
        4
    }

    fn store<T>(&self, mem: &mut Memory<'_, T>, offset: usize) -> Result<()> {
        let (ptr, len) = lower_string(mem, self)?;
        // FIXME: needs memory64 handling
        *mem.get(offset + 0) = (ptr as i32).to_le_bytes();
        *mem.get(offset + 4) = (len as i32).to_le_bytes();
        Ok(())
    }

    fn lift(_src: &Self::Lower) -> Result<Self::Lift> {
        unreachable!("never lifted, should use `Value<str>` instead")
    }
}

fn lower_string<T>(mem: &mut Memory<'_, T>, string: &str) -> Result<(usize, usize)> {
    match mem.string_encoding() {
        StringEncoding::Utf8 => {
            let ptr = mem.realloc(0, 0, 1, string.len())?;
            mem.memory()[ptr..][..string.len()].copy_from_slice(string.as_bytes());
            Ok((ptr, string.len()))
        }
        StringEncoding::Utf16 => {
            let size = string.len() * 2;
            let mut ptr = mem.realloc(0, 0, 2, size)?;
            let bytes = &mut mem.memory()[ptr..][..size];
            let mut copied = 0;
            for (u, bytes) in string.encode_utf16().zip(bytes.chunks_mut(2)) {
                let u_bytes = u.to_le_bytes();
                bytes[0] = u_bytes[0];
                bytes[1] = u_bytes[1];
                copied += 1;
            }
            if (copied * 2) < size {
                ptr = mem.realloc(ptr, size, 2, copied * 2)?;
            }
            Ok((ptr, copied))
        }
        StringEncoding::CompactUtf16 => {
            unimplemented!("compact-utf-16");
        }
    }
}

impl<'a> Cursor<'a, String> {
    /// Returns the underlying string that this cursor points to.
    ///
    /// Note that this will internally decode the string from the wasm's
    /// encoding to utf-8 and additionally perform validation.
    ///
    /// # Errors
    ///
    /// Returns an error if this string's pointer/length are out of bounds or
    /// if the string wasn't encoded correctly (e.g. invalid utf-8).
    pub fn to_str(&self) -> Result<Cow<'a, str>> {
        let ptr_and_len = self.item_bytes();
        // FIXME: needs memory64 treatment
        let ptr = u32::from_le_bytes(ptr_and_len[..4].try_into().unwrap());
        let len = u32::from_le_bytes(ptr_and_len[4..].try_into().unwrap());
        let (ptr, len) = (usize::try_from(ptr)?, usize::try_from(len)?);
        match self.string_encoding() {
            StringEncoding::Utf8 => self.decode_utf8(ptr, len),
            StringEncoding::Utf16 => self.decode_utf16(ptr, len),
            StringEncoding::CompactUtf16 => {
                if len & UTF16_TAG != 0 {
                    self.decode_utf16(ptr, len ^ UTF16_TAG)
                } else {
                    self.decode_latin1(ptr, len)
                }
            }
        }
    }

    fn decode_utf8(&self, ptr: usize, len: usize) -> Result<Cow<'a, str>> {
        let memory = self.all_memory();
        let memory = memory
            .get(ptr..)
            .and_then(|s| s.get(..len))
            .ok_or_else(|| anyhow::anyhow!("string out of bounds"))?;
        Ok(str::from_utf8(memory)?.into())
    }

    fn decode_utf16(&self, ptr: usize, len: usize) -> Result<Cow<'a, str>> {
        let memory = self.all_memory();
        let memory = len
            .checked_mul(2)
            .and_then(|byte_len| memory.get(ptr..)?.get(..byte_len))
            .ok_or_else(|| anyhow::anyhow!("string out of bounds"))?;
        Ok(std::char::decode_utf16(
            memory
                .chunks(2)
                .map(|chunk| u16::from_le_bytes(chunk.try_into().unwrap())),
        )
        .collect::<Result<String, _>>()?
        .into())
    }

    fn decode_latin1(&self, ptr: usize, len: usize) -> Result<Cow<'a, str>> {
        drop((ptr, len));
        unimplemented!()
    }
}

unsafe impl<T> ComponentValue for [T]
where
    T: ComponentValue,
{
    type Lower = [ValRaw; 2];
    type Lift = Infallible;

    fn typecheck(ty: &InterfaceType, types: &ComponentTypes) -> Result<()> {
        match ty {
            InterfaceType::List(t) => T::typecheck(&types[*t], types),
            other => bail!("expected `list` found `{}`", desc(other)),
        }
    }

    fn lower<U>(
        &self,
        store: &mut StoreContextMut<U>,
        func: &Func,
        dst: &mut MaybeUninit<[ValRaw; 2]>,
    ) -> Result<()> {
        let (ptr, len) = lower_list(&mut Memory::new(store.as_context_mut(), func), self)?;
        // See "WRITEPTR64" above for why this is always storing a 64-bit
        // integer.
        map_maybe_uninit!(dst[0].i64).write((ptr as i64).to_le());
        map_maybe_uninit!(dst[1].i64).write((len as i64).to_le());
        Ok(())
    }

    #[inline]
    fn size() -> usize {
        8
    }

    #[inline]
    fn align() -> u32 {
        4
    }

    fn store<U>(&self, mem: &mut Memory<'_, U>, offset: usize) -> Result<()> {
        let (ptr, len) = lower_list(mem, self)?;
        *mem.get(offset + 0) = (ptr as i32).to_le_bytes();
        *mem.get(offset + 4) = (len as i32).to_le_bytes();
        Ok(())
    }

    fn lift(_src: &Self::Lower) -> Result<Self::Lift> {
        unreachable!("never lifted, should use `Value<[T]>` instead")
    }
}

// FIXME: this is not a memcpy for `T` where `T` is something like `u8`.
//
// Some attempts to fix this have proved not fruitful. In isolation an attempt
// was made where:
//
// * `Memory` stored a `*mut [u8]` as its "last view" of memory to avoid
//   reloading the base pointer constantly. This view is reset on `realloc`.
// * The bounds-checks in `Memory::get` were removed (replaced with unsafe
//   indexing)
//
// Even then though this didn't correctly vectorized for `Vec<u8>`. It's not
// entirely clear why but it appeared that it's related to reloading the base
// pointer fo memory (I guess from `Memory` itself?). Overall I'm not really
// clear on what's happening there, but this is surely going to be a performance
// bottleneck in the future.
fn lower_list<T, U>(mem: &mut Memory<'_, U>, list: &[T]) -> Result<(usize, usize)>
where
    T: ComponentValue,
{
    let elem_size = T::size();
    let size = list
        .len()
        .checked_mul(elem_size)
        .ok_or_else(|| anyhow::anyhow!("size overflow copying a list"))?;
    let ptr = mem.realloc(0, 0, T::align(), size)?;
    let mut cur = ptr;
    for item in list {
        item.store(mem, cur)?;
        cur += elem_size;
    }
    Ok((ptr, list.len()))
}

impl<'a, T: ComponentValue> Cursor<'a, Vec<T>> {
    /// Returns the item length of this vector
    pub fn len(&self) -> usize {
        // FIXME: needs memory64 treatment
        u32::from_le_bytes(self.item_bytes()[4..].try_into().unwrap()) as usize
    }

    /// Returns an iterator over the elements of this vector.
    ///
    /// The returned iterator is an exact-size iterator and is of length
    /// `self.len()`. Note that the iterator is also an iterator of [`Cursor`]
    /// types representing that the desired values all continue to live in wasm
    /// linear memory.
    ///
    /// # Errors
    ///
    /// Returns an error if this list's pointer/length combination is
    /// out-of-bounds, or if the length times the element size is too large to
    /// fit in linear memory.
    pub fn iter(&self) -> Result<impl ExactSizeIterator<Item = Cursor<'a, T>> + '_> {
        let (ptr, len) = {
            let ptr_and_len = self.item_bytes();
            // FIXME: needs memory64 treatment
            let ptr = u32::from_le_bytes(ptr_and_len[..4].try_into().unwrap());
            let len = u32::from_le_bytes(ptr_and_len[4..].try_into().unwrap());
            (usize::try_from(ptr)?, usize::try_from(len)?)
        };
        len.checked_mul(T::size())
            .and_then(|byte_len| self.all_memory().get(ptr..)?.get(..byte_len))
            .ok_or_else(|| anyhow::anyhow!("list out of bounds"))?;

        Ok((0..len).map(move |i| {
            // The `move_to` function is not safe because `Cursor` is a static
            // proof that the offset/length is in-bounds. This bounds-check,
            // however, was just performed above so we know that the offset is
            // indeed valid, meaning this `unsafe` should be ok.
            unsafe { self.move_to(ptr + T::size() * i) }
        }))
    }
}

impl<'a> Cursor<'a, Vec<u8>> {
    /// Get access to the raw underlying memory for this byte slice.
    ///
    /// Note that this is specifically only implemented for a `(list u8)` type
    /// since it's known to be valid in terms of alignment and representation
    /// validity.
    ///
    /// # Errors
    ///
    /// Returns an error if the pointer or of this slice point outside of linear
    /// memory.
    pub fn as_slice(&self) -> Result<&'a [u8]> {
        let (ptr, len) = {
            let ptr_and_len = self.item_bytes();
            // FIXME: needs memory64 treatment
            let ptr = u32::from_le_bytes(ptr_and_len[..4].try_into().unwrap());
            let len = u32::from_le_bytes(ptr_and_len[4..].try_into().unwrap());
            (usize::try_from(ptr)?, usize::try_from(len)?)
        };
        self.all_memory()
            .get(ptr..)
            .and_then(|m| m.get(..len))
            .ok_or_else(|| anyhow::anyhow!("list out of bounds"))
    }
}

#[inline]
const fn align_to(a: usize, align: u32) -> usize {
    debug_assert!(align.is_power_of_two());
    let align = align as usize;
    (a + (align - 1)) & !(align - 1)
}

unsafe impl<T> ComponentValue for Option<T>
where
    T: ComponentValue,
{
    type Lower = TupleLower2<<u32 as ComponentValue>::Lower, T::Lower>;
    type Lift = Option<T::Lift>;

    fn typecheck(ty: &InterfaceType, types: &ComponentTypes) -> Result<()> {
        match ty {
            InterfaceType::Option(t) => T::typecheck(&types[*t], types),
            other => bail!("expected `option` found `{}`", desc(other)),
        }
    }

    fn lower<U>(
        &self,
        store: &mut StoreContextMut<U>,
        func: &Func,
        dst: &mut MaybeUninit<Self::Lower>,
    ) -> Result<()> {
        match self {
            None => {
                map_maybe_uninit!(dst.A1.i32).write(0_i32.to_le());
                // Note that this is unsafe as we're writing an arbitrary
                // bit-pattern to an arbitrary type, but part of the unsafe
                // contract of the `ComponentValue` trait is that we can assign
                // any bit-pattern. By writing all zeros here we're ensuring
                // that the core wasm arguments this translates to will all be
                // zeros (as the canonical ABI requires).
                unsafe {
                    map_maybe_uninit!(dst.A2).as_mut_ptr().write_bytes(0u8, 1);
                }
            }
            Some(val) => {
                map_maybe_uninit!(dst.A1.i32).write(1_i32.to_le());
                val.lower(store, func, map_maybe_uninit!(dst.A2))?;
            }
        }
        Ok(())
    }

    #[inline]
    fn size() -> usize {
        align_to(1, T::align()) + T::size()
    }

    #[inline]
    fn align() -> u32 {
        T::align()
    }

    fn store<U>(&self, mem: &mut Memory<'_, U>, offset: usize) -> Result<()> {
        match self {
            None => {
                mem.get::<1>(offset)[0] = 0;
            }
            Some(val) => {
                mem.get::<1>(offset)[0] = 1;
                val.store(mem, offset + align_to(1, T::align()))?;
            }
        }
        Ok(())
    }

    fn lift(src: &Self::Lower) -> Result<Self::Lift> {
        Ok(match i32::from_le(unsafe { src.A1.i32 }) {
            0 => None,
            1 => Some(T::lift(&src.A2)?),
            _ => bail!("invalid option discriminant"),
        })
    }
}

impl<'a, T: ComponentValue> Cursor<'a, Option<T>> {
    /// Returns the underlying value for this `Option<T>`
    ///
    /// Note that the payload of the `Option` returned is itself a cursor as it
    /// still points into linear memory.
    ///
    /// # Errors
    ///
    /// Returns an error if the discriminant for this `Option<T>` in linear
    /// memory is invalid.
    #[inline]
    pub fn get(&self) -> Result<Option<Cursor<'a, T>>> {
        match self.item_bytes()[0] {
            0 => Ok(None),
            1 => Ok(Some(self.bump(align_to(1, T::align())))),
            _ => bail!("invalid option discriminant"),
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct ResultLower<T: Copy, E: Copy> {
    tag: ValRaw,
    payload: ResultLowerPayload<T, E>,
}

#[derive(Clone, Copy)]
#[repr(C)]
union ResultLowerPayload<T: Copy, E: Copy> {
    ok: T,
    err: E,
}

unsafe impl<T, E> ComponentValue for Result<T, E>
where
    T: ComponentValue,
    E: ComponentValue,
{
    type Lower = ResultLower<T::Lower, E::Lower>;
    type Lift = Result<T::Lift, E::Lift>;

    fn typecheck(ty: &InterfaceType, types: &ComponentTypes) -> Result<()> {
        match ty {
            InterfaceType::Expected(r) => {
                let expected = &types[*r];
                T::typecheck(&expected.ok, types)?;
                E::typecheck(&expected.err, types)?;
                Ok(())
            }
            other => bail!("expected `expected` found `{}`", desc(other)),
        }
    }

    fn lower<U>(
        &self,
        store: &mut StoreContextMut<U>,
        func: &Func,
        dst: &mut MaybeUninit<Self::Lower>,
    ) -> Result<()> {
        // Start out by zeroing out the payload. This will ensure that if either
        // arm doesn't initialize some values then everything is still
        // deterministically set.
        //
        // Additionally, this initialization of zero means that the specific
        // types written by each `lower` call below on each arm still has the
        // correct value even when "joined" with the other arm.
        //
        // Finally note that this is required by the canonical ABI to some
        // degree where if the `Ok` arm initializes fewer values than the `Err`
        // arm then all the remaining values must be initialized to zero, and
        // that's what this does.
        unsafe {
            map_maybe_uninit!(dst.payload)
                .as_mut_ptr()
                .write_bytes(0u8, 1);
        }

        match self {
            Ok(e) => {
                map_maybe_uninit!(dst.tag.i32).write(0_i32.to_le());
                e.lower(store, func, map_maybe_uninit!(dst.payload.ok))?;
            }
            Err(e) => {
                map_maybe_uninit!(dst.tag.i32).write(1_i32.to_le());
                e.lower(store, func, map_maybe_uninit!(dst.payload.err))?;
            }
        }
        Ok(())
    }

    #[inline]
    fn size() -> usize {
        align_to(1, Self::align()) + T::size().max(E::size())
    }

    #[inline]
    fn align() -> u32 {
        T::align().max(E::align())
    }

    fn store<U>(&self, mem: &mut Memory<'_, U>, offset: usize) -> Result<()> {
        match self {
            Ok(e) => {
                mem.get::<1>(offset)[0] = 0;
                e.store(mem, offset + align_to(1, Self::align()))?;
            }
            Err(e) => {
                mem.get::<1>(offset)[0] = 1;
                e.store(mem, offset + align_to(1, Self::align()))?;
            }
        }
        Ok(())
    }

    fn lift(src: &Self::Lower) -> Result<Self::Lift> {
        // This implementation is not correct if there's actually information in
        // the payload. This doesn't validate that if `payload` has a nonzero
        // size that the "extended" bits are all zero. For example if
        // `Result<i32, i64>` is returned then that's represented as `i32 i64`
        // and `0 i64::MAX` is an invalid return value. This implementation,
        // however, would consider that valid since it would not try to read the
        // upper bits of the i64.
        //
        // For now this is ok because `lift` is only called for types where
        // `Lower` is at most one `ValRaw`. A `Result<T, E>` always takes up at
        // least one `ValRaw` for the discriminant so we know that if this is
        // being used then both `T` and `E` have zero size.
        assert!(mem::size_of_val(&src.payload) == 0);

        Ok(match i32::from_le(unsafe { src.tag.i32 }) {
            0 => Ok(unsafe { T::lift(&src.payload.ok)? }),
            1 => Err(unsafe { E::lift(&src.payload.err)? }),
            _ => bail!("invalid expected discriminant"),
        })
    }
}

impl<'a, T, E> Cursor<'a, Result<T, E>>
where
    T: ComponentValue,
    E: ComponentValue,
{
    /// Returns the underlying value for this `Result<T, E>`
    ///
    /// Note that the payloads of the `Result` returned are themselves cursors
    /// as they still point into linear memory.
    ///
    /// # Errors
    ///
    /// Returns an error if the discriminant for this `Result` in linear
    /// memory is invalid.
    #[inline]
    pub fn get(&self) -> Result<Result<Cursor<'a, T>, Cursor<'a, E>>> {
        let align = <Result<T, E> as ComponentValue>::align();
        match self.item_bytes()[0] {
            0 => Ok(Ok(self.bump(align_to(1, align)))),
            1 => Ok(Err(self.bump(align_to(1, align)))),
            _ => bail!("invalid expected discriminant"),
        }
    }
}

macro_rules! impl_component_ty_for_tuples {
    // the unit tuple goes to the `Unit` type, not the `Tuple` type
    //
    // FIXME(WebAssembly/component-model#21) there's some active discussion on
    // the relationship between the 0-tuple and the unit type in the component
    // model.
    (0) => {};

    ($n:tt $($t:ident)*) => {paste::paste!{
        #[allow(non_snake_case)]
        #[doc(hidden)]
        #[derive(Clone, Copy)]
        #[repr(C)]
        pub struct [<TupleLower$n>]<$($t),*> {
            $($t: $t,)*
        }

        #[allow(non_snake_case)]
        unsafe impl<$($t,)*> ComponentValue for ($($t,)*)
        where $($t: ComponentValue),*
        {
            type Lower = [<TupleLower$n>]<$($t::Lower),*>;
            type Lift = ($($t::Lift,)*);

            fn typecheck(
                ty: &InterfaceType,
                types: &ComponentTypes,
            ) -> Result<()> {
                match ty {
                    InterfaceType::Tuple(t) => {
                        let tuple = &types[*t];
                        if tuple.types.len() != $n {
                            bail!("expected {}-tuple, found {}-tuple", $n, tuple.types.len());
                        }
                        let mut tuple = tuple.types.iter();
                        $($t::typecheck(tuple.next().unwrap(), types)?;)*
                        debug_assert!(tuple.next().is_none());
                        Ok(())
                    }
                    other => bail!("expected `tuple` found `{}`", desc(other)),
                }
            }

            fn lower<U>(
                &self,
                store: &mut StoreContextMut<U>,
                func: &Func,
                dst: &mut MaybeUninit<Self::Lower>,
            ) -> Result<()> {
                let ($($t,)*) = self;
                $($t.lower(store, func, map_maybe_uninit!(dst.$t))?;)*
                Ok(())
            }

            #[inline]
            fn size() -> usize {
                let mut size = 0;
                $(size = align_to(size, $t::align()) + $t::size();)*
                size
            }

            #[inline]
            fn align() -> u32 {
                let mut align = 1;
                $(align = align.max($t::align());)*
                align
            }

            fn store<U>(&self, memory: &mut Memory<'_, U>, mut offset: usize) -> Result<()> {
                let ($($t,)*) = self;
                // TODO: this requires that `offset` is aligned which we may not
                // want to do
                $(
                    offset = align_to(offset, $t::align());
                    $t.store(memory, offset)?;
                    offset += $t::size();
                )*
                drop(offset); // silence warning about last assignment
                Ok(())
            }

            #[inline]
            fn lift(src: &Self::Lower) -> Result<Self::Lift> {
                Ok(($($t::lift(&src.$t)?,)*))
            }
        }

        impl<'a, $($t),*> Cursor<'a, ($($t,)*)>
        where
            $($t: ComponentValue),*
        {
            fn start_offset(&self) -> usize {
                0
            }

            define_tuple_cursor_accessors!(start_offset $($t)*);
        }
    }};
}

macro_rules! define_tuple_cursor_accessors {
    ($offset:ident) => {};
    ($offset:ident $t:ident $($u:ident)*) => {
        paste::paste! {
            /// Returns a pointer to the `n`th field of the tuple contained
            /// within this cursor.
            #[inline]
            pub fn [<$t:lower>](&self) -> Cursor<'a, $t> {
                self.bump(align_to(self.$offset(), $t::align()))
            }

            #[allow(dead_code)]
            #[inline]
            fn [<$t:lower _end>](&self) -> usize {
                align_to(self.$offset(), $t::align()) + $t::size()
            }

            define_tuple_cursor_accessors!([<$t:lower _end>] $($u)*);
        }
    };
}

for_each_function_signature!(impl_component_ty_for_tuples);

fn desc(ty: &InterfaceType) -> &'static str {
    match ty {
        InterfaceType::U8 => "u8",
        InterfaceType::S8 => "s8",
        InterfaceType::U16 => "u16",
        InterfaceType::S16 => "s16",
        InterfaceType::U32 => "u32",
        InterfaceType::S32 => "s32",
        InterfaceType::U64 => "u64",
        InterfaceType::S64 => "s64",
        InterfaceType::Float32 => "f32",
        InterfaceType::Float64 => "f64",
        InterfaceType::Unit => "unit",
        InterfaceType::Bool => "bool",
        InterfaceType::Char => "char",
        InterfaceType::String => "string",
        InterfaceType::List(_) => "list",
        InterfaceType::Tuple(_) => "tuple",
        InterfaceType::Option(_) => "option",
        InterfaceType::Expected(_) => "expected",

        InterfaceType::Record(_) => "record",
        InterfaceType::Variant(_) => "variant",
        InterfaceType::Flags(_) => "flags",
        InterfaceType::Enum(_) => "enum",
        InterfaceType::Union(_) => "union",
    }
}

/// A trait representing values which can be returned from a [`TypedFunc`].
///
/// For all values which implement the [`ComponentValue`] trait this is
/// implemented for either `T` or [`Value<T>`]. For more information on which
/// to use see the documentation at [`Func::typed`].
///
/// The contents of this trait are hidden as it's intended to be an
/// implementation detail of Wasmtime. The contents of this trait are not
/// covered by Wasmtime's stability guarantees.
//
// Note that this is an `unsafe` trait because the safety of `TypedFunc` relies
// on `typecheck` being correct relative to `Lower`, among other things.
//
// Also note that this trait specifically is not sealed because we'll
// eventually have a proc macro that generates implementations of this trait
// for external types in a `#[derive]`-like fashion.
pub unsafe trait ComponentReturn: Sized {
    /// The core wasm lowered value used to interpret this return value.
    ///
    /// This is `T::Lower` in the case of `ComponentReturn for T` and this is
    /// otherwise a singular `ValRaw` for `Value<T>` to store the i32 return
    /// value.
    #[doc(hidden)]
    type Lower: Copy;

    /// Performs a type-check to ensure that this `ComponentReturn` value
    /// matches the interface type specified.
    ///
    /// Note that even if `Self` matches the `ty` specified this function will
    /// also perform a check to ensure that `Lower` is suitable for returning
    /// `Self` in the core wasm ABI. For example `Value<u8>` has the type
    /// `InterfaceType::U8` but is not suitable as a return type since
    /// `Value<u8>` represents an indirect return value and `u8` is a direct
    /// return. That check is done by this function.
    #[doc(hidden)]
    fn typecheck(ty: &InterfaceType, types: &ComponentTypes) -> Result<()>;

    /// Performs the lifting operation from the core wasm return value into
    /// `Self`.
    ///
    /// Note that this can fail in the case that an indirect pointer was
    /// returned and the indirect pointer is out-of-bounds.
    #[doc(hidden)]
    fn lift(store: &StoreOpaque, func: &Func, src: &Self::Lower) -> Result<Self>;
}

// Note that the trait bound here requires that the lifted value of `T` is
// itself. This is true for primitives and ADTs above and is required to
// implement the `lift` function. That also means that implementations of
// `ComponentValue` for strings/lists statically can't use this impl because
// their `Lift` is not themselves.
unsafe impl<T: ComponentValue<Lift = T>> ComponentReturn for T {
    type Lower = T::Lower;

    fn typecheck(ty: &InterfaceType, types: &ComponentTypes) -> Result<()> {
        // Perform a check that the size of the return value is indeed at most
        // one core wasm abi value. If there is more than one core wasm abi
        // return value then the `Value<T>` type must be used instead.
        if T::flatten_count() > MAX_STACK_RESULTS {
            let name = std::any::type_name::<T>();
            bail!(
                "cannot use `{name}` as a return value as it is \
                 returned indirectly, use `Value<{name}>` instead"
            );
        }

        // ... and if the ABI is appropriate then we can otherwise delegate to
        // a normal type-check.
        T::typecheck(ty, types)
    }

    fn lift(_store: &StoreOpaque, _func: &Func, src: &Self::Lower) -> Result<Self> {
        <T as ComponentValue>::lift(src)
    }
}

unsafe impl<T: ComponentValue> ComponentReturn for Value<T> {
    type Lower = ValRaw;

    fn typecheck(ty: &InterfaceType, types: &ComponentTypes) -> Result<()> {
        // Similar to the impl above, except this is the reverse. When using
        // `Value<T>` that means the return value is expected to be indirectly
        // returned in linear memory. That means we need to verify that the
        // canonical ABI indeed return `T` indirectly by double-checking that
        // the core wasm abi makeup of the type requires more than one value.
        if T::flatten_count() <= MAX_STACK_RESULTS {
            let name = std::any::type_name::<T>();
            bail!(
                "cannot use `Value<{name}>` as a return value as it is not \
                 returned indirectly, use `{name}` instead"
            );
        }

        // ... and like above if the abi lines up then delegate to `T` for
        // further type-checking.
        T::typecheck(ty, types)
    }

    fn lift(store: &StoreOpaque, func: &Func, src: &Self::Lower) -> Result<Self> {
        // FIXME: needs to read an i64 for memory64
        let ptr = u32::from_le(unsafe { src.i32 as u32 }) as usize;
        Value::new(store, func, ptr)
    }
}

pub use self::value::*;

/// The `Value` and `Cursor` types have internal variants that are important to
/// uphold so they're defined in a small submodule here to statically prevent
/// access to their private internals by the surrounding module.
mod value {
    use super::*;
    use crate::StoreContext;

    /// A pointer to a type which is stored in WebAssembly linear memory.
    ///
    /// This structure is used as the return value from [`TypedFunc`] at this
    /// time to represent a function that returns its value through linear
    /// memory instead of directly through return values.
    ///
    /// A [`Value<T>`] represents a valid chunk of WebAssembly linear memory.
    /// From a [`Value<T>`] a [`Cursor<T>`] can be created which is used to
    /// actually inspect the contents of WebAssembly linear memory.
    //
    // As an implementation note the `Value` type has an unsafe contract where
    // `pointer` is valid for `T::size()` bytes within the memory pointed to by
    // the `origin` function specified. This `Value` itself does not pin the
    // memory as its purpose is to not pin the store. The pinning of the store
    // happens later.
    pub struct Value<T> {
        pointer: usize,
        origin: Func,
        _marker: marker::PhantomData<T>,
    }

    /// A type which is used to inspect the contents of `T` as it resides in
    /// WebAssembly linear memory.
    ///
    /// The [`Cursor<T>`] type is created by the [`Value::cursor`] method which
    /// holds a shared borrow onto the [`Store`](crate::Store). This does
    /// not necessarily represent that `T` itself is stored in linear memory,
    /// for example `Cursor<String>` doesn't mean that a host `String` type
    /// is stored in linear memory but rather a canonical ABI string is stored
    /// in linear memory. The [`Cursor<T>`] has per-`T` methods on it to access
    /// the contents of wasm linear memory.
    ///
    /// The existence of [`Cursor<T>`] means that the pointer that the cursor
    /// has is valid for `T::size()` bytes of linear memory. The actual memory
    /// it points to may have invalid contents, but that's left for each
    /// method-of-interpretation to determine.
    //
    // As an implementation detail, like `Value`, the existence of a `Cursor`
    // is static proof that `offset` within `all_memory` is valid for
    // `T::size()` bytes. This enables the `item_bytes` method to use unchecked
    // indexing.
    pub struct Cursor<'a, T> {
        offset: usize,
        all_memory: &'a [u8],
        string_encoding: StringEncoding,
        _marker: marker::PhantomData<T>,
    }

    impl<T> Value<T>
    where
        T: ComponentValue,
    {
        pub(super) fn new(store: &StoreOpaque, origin: &Func, pointer: usize) -> Result<Value<T>> {
            // Construction of a `Value` indicates proof that the `pointer` is
            // valid, so the check is performed here to ensure that it's safe
            // to construct the `Value`.
            origin
                .memory(store)
                .get(pointer..)
                .and_then(|s| s.get(..T::size()))
                .ok_or_else(|| anyhow::anyhow!("pointer out of bounds of memory"))?;
            Ok(Value {
                pointer,
                origin: *origin,
                _marker: marker::PhantomData,
            })
        }

        /// Returns a [`Cursor<T>`] that can be used to read linear memory.
        ///
        /// This method will borrow the `store` provided to get access to wasm
        /// linear memory and the returned [`Cursor<T>`] is used to iterate
        /// over the wasm linear memory using accessor methods specific to
        /// the type `T`.
        ///
        /// # Panics
        ///
        /// This function will panic if `store` doesn't own the wasm linear
        /// memory that this `Value` points to.
        pub fn cursor<'a, U: 'a>(&self, store: impl Into<StoreContext<'a, U>>) -> Cursor<'a, T> {
            let store = store.into();
            let all_memory = self.origin.memory(store.0);

            // Note that construction of a `Cursor` is static proof that the
            // `offset` is valid. This should be ok here because this `Value`
            // was already validated and memory cannot shrink, so after the
            // `Value` was created the memory should still be of an appropriate
            // size.
            Cursor {
                offset: self.pointer,
                all_memory,
                string_encoding: store.0[self.origin.0].options.string_encoding,
                _marker: marker::PhantomData,
            }
        }
    }

    impl<'a, T: ComponentValue> Cursor<'a, T> {
        /// Returns the bytes that `T` is stored within.
        #[inline]
        pub(super) fn item_bytes(&self) -> &[u8] {
            // The existence of `Cursor<T>` as a wrapper type is intended to
            // serve as proof that this `unsafe` block is indeed safe. The
            // unchecked indexing here is possible due to the bounds checks
            // that happen during construction of a `Cursor`.
            //
            // ... but in debug mode we double-check just to be sure.
            unsafe {
                if cfg!(debug_assertions) {
                    drop(&self.all_memory[self.offset..][..T::size()]);
                }
                self.all_memory
                    .get_unchecked(self.offset..)
                    .get_unchecked(..T::size())
            }
        }

        /// Returns all of linear memory, useful for strings/lists which have
        /// indirect pointers.
        #[inline]
        pub(super) fn all_memory(&self) -> &'a [u8] {
            self.all_memory
        }

        /// Returns the string encoding in use.
        pub(super) fn string_encoding(&self) -> StringEncoding {
            self.string_encoding
        }

        /// Increments this `Cursor` forward by `offset` bytes to point to a
        /// `U` that is contained within `T`.
        ///
        /// # Panics
        ///
        /// Panics if `offset + U::size()` is larger than `T::size()`.
        #[inline]
        pub(super) fn bump<U>(&self, offset: usize) -> Cursor<'a, U>
        where
            U: ComponentValue,
        {
            // Perform a bounds check that if we increase `self.offset` by
            // `offset` and point to `U` that the result is still contained
            // within this `Cursor<T>`. After doing so it's safe to call
            // `move_to` as the bounds check has been performed.
            //
            // Note that it's expected that this bounds-check can be optimized
            // out in most cases. The `offset` argument is typically a constant
            // thing like a field or payload offset, and then `{T,U}::size()`
            // are also trivially const-evaluatable by LLVM. If this shows up
            // in profiles more functions may need `#[inline]`.
            assert!(offset + U::size() <= T::size());
            unsafe { self.move_to(self.offset + offset) }
        }

        /// An unsafe method to construct a new `Cursor` pointing to within
        /// the same linear memory that this `Cursor` points to but for a
        /// different type `U` and at a different `offset`.
        ///
        /// # Unsafety
        ///
        /// This function is unsafe because `Cursor` is a static proof that the
        /// `offset` is valid for `U::size()` bytes within linear memory.
        /// Callers must uphold this invariant themselves and perform a bounds
        /// check before being able to safely call this method.
        #[inline]
        pub(super) unsafe fn move_to<U>(&self, offset: usize) -> Cursor<'a, U>
        where
            U: ComponentValue,
        {
            Cursor {
                offset,
                all_memory: self.all_memory,
                string_encoding: self.string_encoding,
                _marker: marker::PhantomData,
            }
        }
    }
}
