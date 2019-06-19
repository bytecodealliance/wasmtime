#![warn(missing_docs)]

//! A host module creation DSL for Wasmtime. This allows you to easily export functions and globals to a
//! Wasm module and have the complexities handled for you. It also gives better type safety,
//! unlike when creating the module manually it's impossible for you to get the type signature of
//! functions wrong or cause a runtime error by instantiating a global with a constant of the wrong type.
//!
//! Using the macro looks like so:
//!
//! ```rust
//! # use wasmtime_hostmodule::{
//! #     exports,
//! #     BindArgType,
//! #     Func,
//! #     Global,
//! #     Instantiate,
//! #     Memory,
//! #     Table,
//! #     TableElementType
//! # };
//! #
//! fn hello_world() {
//!     println!("Hello, world!");
//! }
//!
//! fn print_and_return(val: i32) -> i32 {
//!     println!("{}", val);
//!     val
//! }
//!
//! let mut counter = 100;
//! let my_closure = move |inc: u32| -> u32 {
//!     counter += inc;
//!     counter
//! };
//!
//! let my_module = exports! {
//!     do_thing: Func(hello_world.bind::<()>()),
//!     print_and_return: Func(print_and_return.bind::<(i32,)>()),
//!     counting_func: Func(my_closure.bind::<(u32,)>()),
//!     my_glob: Global(100u64, Default::default()),
//!     memory: Memory {
//!         minimum: 1,
//!         maximum: Some(2),
//!         shared: false,
//!     },
//!     table: Table {
//!         ty: TableElementType::Func,
//!         minimum: 10,
//!         maximum: Some(20),
//!     },
//! };
//!
//! # my_module.instantiate().unwrap();
//! ```
//!
//! You can see the different kinds of host exports that you can create in that example. Functions must
//! have their argument list declared - Rust types can implement multiple different function traits
//! whereas WebAssembly functions must be monomorphic, so you have to specify which arguments the function
//! takes. Rust's functions must have their output type uniquely defined by their input types, however, and
//! so you don't need to specify the return type. To specify a function's arguments, you simply call
//! `BindArgType::bind` on the function. You can also add closures as exports as long as they live for the
//! `'static` lifetime (usually this means making them a `move` closure).
//!
//! You don't need to call `.bind` on function pointers, since they are monomorphic.
//!
//! Here's an example:
//!
//! ```rust
//! # use wasmtime_hostmodule::{
//! #     exports,
//! #     BindArgType,
//! #     Func,
//! #     Instantiate,
//! # };
//! #
//! let mut call_foo_counter = 0;
//! let foo = move || {
//!     call_foo_counter += 1;
//!     println!("`foo` called {} times", call_foo_counter);
//! };
//! fn bar(a: i32) -> i32 { a }
//! fn baz(b: f32, c: f32) -> f32 { b + c }
//!
//! let pointer: fn(_, _) -> _ = baz;
//!
//! let my_module = exports! {
//!     foo: Func(foo.bind::<()>()),
//!     bar: Func(bar.bind::<(i32,)>()),
//!     baz: Func(baz.bind::<(f32, f32)>()),
//!     baz_again: Func(pointer),
//! };
//! #
//! # my_module.instantiate().unwrap();
//! ```
//!
//! Globals can either be a regular Rust number type that Wasm supports (`i32`, `i64`, `u32`, `u64`, `f32`
//! and `f64`), or you can supply a `Value`, which allows you to specify the bit pattern of floats and also
//! allows you to choose different types at runtime if necessary. They can also be either immutable (the
//! default) or mutable:
//!
//! ```rust
//! # use wasmtime_hostmodule::{
//! #     exports,
//! #     Global,
//! #     Value,
//! #     Mutability,
//! #     Instantiate,
//! # };
//! #
//! let foo = 10u32;
//! let bar = 3.14159f32;
//! let baz = Value::F32(0b11111111111111111111111111111111);
//!
//! let my_module = exports! {
//!     foo: Global(foo, Default::default()),
//!     bar: Global(bar, Mutability::Immutable),
//!     baz: Global(baz, Mutability::Mutable),
//! };
//! #
//! # my_module.instantiate().unwrap();
//! ```
//!
//! Memory sections and tables are much simpler, you just use the types reexported from `cranelift_wasm`.
//!
//! ```rust
//! # use wasmtime_hostmodule::{
//! #     exports,
//! #     Memory,
//! #     Table,
//! #     TableElementType,
//! #     Instantiate,
//! # };
//! #
//! let my_module = exports! {
//!     memory: Memory {
//!         minimum: 1,
//!         maximum: Some(2),
//!         shared: false,
//!     },
//!     table: Table {
//!         ty: TableElementType::Func,
//!         minimum: 10,
//!         maximum: Some(20),
//!     },
//! };
//! #
//! # my_module.instantiate().unwrap();
//! ```
//!
//! # Instantiation
//!
//! Just building this module doesn't actually do anything though, if you want to import it
//! into another wasm module you need to instantiate it. There are two ways of doing so. The
//! simple way (which is recommended) and the advanced way, which allows you to handle the
//! `InstanceHandle` creation yourself and so can give you more control if necessary.
//!
//! ```rust
//! # use wasmtime_hostmodule::{
//! #     hlist::{Cons, Nil},
//! #     exports,
//! #     Builder,
//! #     Exports,
//! #     Instantiate,
//! #     InstantiationError,
//! # };
//! # use cranelift_entity::PrimaryMap;
//! # use std::{rc::Rc, cell::RefCell, collections::HashMap};
//! # use wasmtime_runtime::{Imports, InstanceHandle};
//! #
//! # let my_module = exports! { };
//! #
//! // Simple way:
//! let handle = my_module.instantiate()?;
//!
//! // Advanced way:
//! let mut builder: Builder = Default::default();
//! let host_data = my_module.build(&mut builder);
//! let handle = builder.instantiate_module(host_data)?;
//! #
//! # let builder = Builder::default();
//! # let some_existing_module = builder.module;
//! # let some_existing_function_map = builder.functions;
//! # let some_triple = builder.triple;
//! # let my_imports = Imports::none();
//! # let my_data_initializers = Vec::new();
//! # let my_signatures = PrimaryMap::new();
//!
//! // Really advanced way:
//! let mut builder: Builder = Builder {
//!     module: some_existing_module,
//!     functions: some_existing_function_map,
//!     triple: some_triple,
//! };
//! let host_data = my_module.build(&mut builder);
//!
//! let handle = InstanceHandle::new(
//!     Rc::new(builder.module),
//!     Rc::new(RefCell::new(HashMap::new())),
//!     builder.functions.into_boxed_slice(),
//!     my_imports,
//!     &my_data_initializers,
//!     my_signatures.into_boxed_slice(),
//!     None,
//!     Box::new(host_data),
//! )?;
//! #
//! # Ok::<(), InstantiationError>(())
//! ```
//!
//! It should be noted that you _cannot_ call `.build` twice on the same builder using different exports
//! - the `host_data` that is returned must be passed into `InstanceHandle::new` in order for functions
//! to work properly. If you want to combine two export definition lists and instantiate the results,
//! use the `+` operator:
//!
//! ```rust
//! # use wasmtime_hostmodule::{
//! #     exports,
//! #     Global,
//! #     Instantiate,
//! #     InstantiationError,
//! # };
//! #
//! let first_module = exports! {
//!     foo: Global(1u64, Default::default()),
//! };
//! let second_module = exports! {
//!     bar: Global(2u64, Default::default()),
//! };
//!
//! let handle = (first_module + second_module).instantiate()?;
//! #
//! # Ok::<(), InstantiationError>(())
//! ```
//!
//! # Under the hood
//!
//! The `export` macro isn't magic, creating an export definition using the macro is precisely equivalent
//! to manually creating a cons chain such as in the following example:
//!
//! ```rust
//! # use wasmtime_hostmodule::{
//! #     hlist::{Cons, Nil},
//! #     ExportDef,
//! #     exports,
//! #     BindArgType,
//! #     Func,
//! #     Global,
//! #     Instantiate,
//! #     Memory,
//! #     Table,
//! #     TableElementType
//! # };
//! #
//! # fn hello_world() {
//! #     println!("Hello, world!");
//! # }
//! #
//! # fn print_and_return(val: i32) -> i32 {
//! #     println!("{}", val);
//! #     val
//! # }
//! #
//! # let mut counter = 100;
//! # let my_closure = move |inc: u32| -> u32 {
//! #     counter += inc;
//! #     counter
//! # };
//! #
//! let my_module = Cons(
//!     ExportDef {
//!         name: "do_thing".to_owned(),
//!         val: Func(hello_world.bind::<()>()),
//!     },
//!     Cons(
//!         ExportDef {
//!             name: "print_and_return".to_owned(),
//!             val: Func(print_and_return.bind::<(i32,)>()),
//!         },
//!         Cons(
//!             ExportDef {
//!                 name: "counting_func".to_owned(),
//!                 val: Func(my_closure.bind::<(u32,)>()),
//!             },
//!             Cons(
//!                 ExportDef {
//!                     name: "my_glob".to_owned(),
//!                     val: Global(100u64, Default::default()),
//!                 },
//!                 Cons(
//!                     ExportDef {
//!                         name: "memory".to_owned(),
//!                         val: Memory {
//!                             minimum: 1,
//!                             maximum: Some(2),
//!                             shared: false,
//!                         },
//!                     },
//!                     Cons(
//!                         ExportDef {
//!                             name: "table".to_owned(),
//!                             val: Table {
//!                                 ty: TableElementType::Func,
//!                                 minimum: 10,
//!                                 maximum: Some(20),
//!                             },
//!                         },
//!                         Nil,
//!                     ),
//!                 ),
//!             ),
//!         ),
//!     ),
//! );
//!
//! # my_module.instantiate().unwrap();
//! ```
//!
//! You can even mix and match:
//!
//! ```rust
//! # use wasmtime_hostmodule::{
//! #     hlist::{Cons, Nil},
//! #     ExportDef,
//! #     exports,
//! #     BindArgType,
//! #     Func,
//! #     Global,
//! #     Instantiate,
//! #     Memory,
//! #     Table,
//! #     TableElementType
//! # };
//! #
//! # fn hello_world() {
//! #     println!("Hello, world!");
//! # }
//! #
//! # fn print_and_return(val: i32) -> i32 {
//! #     println!("{}", val);
//! #     val
//! # }
//! #
//! # let mut counter = 100;
//! # let my_closure = move |inc: u32| -> u32 {
//! #     counter += inc;
//! #     counter
//! # };
//! #
//! let my_module = Cons(
//!     ExportDef {
//!         name: "do_thing".to_owned(),
//!         val: Func(hello_world.bind::<()>()),
//!     },
//!     Cons(
//!         ExportDef {
//!             name: "print_and_return".to_owned(),
//!             val: Func(print_and_return.bind::<(i32,)>()),
//!         },
//!         exports! {
//!             counting_func: Func(my_closure.bind::<(u32,)>()),
//!             my_glob: Global(100u64, Default::default()),
//!             memory: Memory {
//!                 minimum: 1,
//!                 maximum: Some(2),
//!                 shared: false,
//!             },
//!             table: Table {
//!                 ty: TableElementType::Func,
//!                 minimum: 10,
//!                 maximum: Some(20),
//!             },
//!         },
//!     ),
//! );
//!
//! # my_module.instantiate().unwrap();
//! ```

use cranelift_codegen::ir::types;
use cranelift_codegen::{ir, isa};
use cranelift_entity::PrimaryMap;
use cranelift_wasm::{DefinedFuncIndex, Global as WasmGlobal, GlobalInit};
use hlist::{Cons, Here, Nil, There};
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;
use target_lexicon::HOST;
use wasmtime_environ::{translate_signature, Export, MemoryPlan, Module, TablePlan};
use wasmtime_jit::target_tunables;
use wasmtime_runtime::{Imports, InstanceHandle, VMContext, VMFunctionBody};

pub use cranelift_wasm::{Memory, Table, TableElementType};
pub use hlist;
pub use wasmtime_runtime::InstantiationError;

/// A host module builder, containing the types we need to create a host module. This doesn't
/// handle instantiation, which can be done by extracting these values and passing them to
/// `InstanceHandle::new`.
pub struct Builder<'a> {
    /// The module metadata
    pub module: Module,
    /// Pointers to the module's functions
    pub functions: PrimaryMap<DefinedFuncIndex, *const VMFunctionBody>,
    /// The target triple. There's no reason for this not to be `target_lexicon::HOST`, since
    /// you're presumably creating this module with in-program function pointers, but there's
    /// also no reason not to allow it to be configured.
    pub triple: &'a target_lexicon::Triple,
}

impl Default for Builder<'static> {
    fn default() -> Self {
        Builder::new(&HOST)
    }
}

impl<'a> Builder<'a> {
    /// Create a new builder, initializing an empty module and function map. You probably want to
    /// use `Builder`'s `Default::default` implementation, which sets the target triple to `HOST`.
    pub fn new(triple: &'a target_lexicon::Triple) -> Self {
        let module = Module::new();
        let functions: PrimaryMap<DefinedFuncIndex, *const VMFunctionBody> = PrimaryMap::new();

        Builder {
            module,
            functions,
            triple,
        }
    }

    /// Finalize the module and create a new instance handle.
    pub fn instantiate_module<T: Any>(
        self,
        host_data: T,
    ) -> Result<InstanceHandle, InstantiationError> {
        let imports = Imports::none();
        let data_initializers = Vec::new();
        let signatures = PrimaryMap::new();

        InstanceHandle::new(
            Rc::new(self.module),
            Rc::new(RefCell::new(HashMap::new())),
            self.functions.into_boxed_slice(),
            imports,
            &data_initializers,
            signatures.into_boxed_slice(),
            None,
            Box::new(host_data),
        )
    }
}

/// A polymorphic WebAssembly value
pub enum Value {
    /// An `i32`
    I32(u32),
    /// An `i64`
    I64(u64),
    /// An `f32` (specified bitwise)
    F32(u32),
    /// An `f64` (specified bitwise)
    F64(u64),
}

impl From<f64> for Value {
    fn from(other: f64) -> Self {
        Value::F64(other.to_bits())
    }
}
impl From<f32> for Value {
    fn from(other: f32) -> Self {
        Value::F32(other.to_bits())
    }
}
impl From<i32> for Value {
    fn from(other: i32) -> Self {
        Value::I32(other as _)
    }
}
impl From<i64> for Value {
    fn from(other: i64) -> Self {
        Value::I64(other as _)
    }
}
impl From<u32> for Value {
    fn from(other: u32) -> Self {
        Value::I32(other)
    }
}
impl From<u64> for Value {
    fn from(other: u64) -> Self {
        Value::I64(other)
    }
}

/// Internal trait for `Exports`, handling the recursion. You probably never want to call this directly.
pub trait ExportsElement<Counter, HostData>: HasHostData {
    /// Add the exports to the `Builder`, returning the required host data.
    fn build_element(self, builder: &mut Builder<'_>) -> Self::HostData;
}

/// Add the exported elements to a builder and return the required host data. This should only be called
/// once on each builder, since you can't combine `HostData`s - they respond to `+` but if you try to use
/// the result of this `+` as your host data your closures will fail at runtime. If you want to build
/// two different export lists, simply combine them with `+` and call `.build` on the result of that.
pub trait Exports: ExportsElement<Here, <Self as HasHostData>::HostData> {
    /// Add the exports to the `Builder`, returning the required host data.
    fn build(self, builder: &mut Builder<'_>) -> Self::HostData;
}

impl<T> Exports for T
where
    T: ExportsElement<Here, <T as HasHostData>::HostData>,
{
    fn build(self, builder: &mut Builder<'_>) -> Self::HostData {
        self.build_element(builder)
    }
}

/// A single exported element. The `T` is expected to be of type `Func`, `Global`, `Memory` or `Table`.
pub struct ExportDef<T> {
    /// The name of the export.
    pub name: String,
    /// The value of the export, either a `Func`, `Global`, `Memory` or `Table`.
    pub val: T,
}

/// Whether a global is mutable or not. You can call `Default::default()` to get `Mutability::Immutable`.
#[derive(Debug, PartialEq, Eq)]
pub enum Mutability {
    /// The global is mutable
    Mutable,
    /// The global is immutable (this is the default)
    Immutable,
}

impl Default for Mutability {
    fn default() -> Self {
        Mutability::Immutable
    }
}

/// A function. The value wrapped should _not_ be a function, but should be a `BindArgs`. You can create a
/// `BindArgs` from a function or closure by using `BindArgType::bind` passing the argument type list as a
/// tuple. This is because Rust functions can implement the `Fn*` traits multiple times with different
/// argument lists, as long as the output type is uniquely identified by the input types.
pub struct Func<T>(pub T);
/// A global, the `T` is expected to be an `impl Into<Value>`, so a `u32`, `u64`, `i32`, `i64`, `f32`, `f64`
/// or (of course) `Value`. You can also implement `Into<Value>` for your own types and use them here, if
/// you so wish.
pub struct Global<T>(pub T, pub Mutability);

/// A struct to monomorphise function argument types using a `PhantomData`. This can then be used with
/// `CallMut`.
pub struct BindArgs<F, A>(pub F, pub PhantomData<A>);

/// Monomorphise a function's arguments so that it can only be called with a single set of
/// argument types.
pub trait BindArgType: Sized {
    /// Bind the argument types. This should be called with a tuple, even if the function
    /// only takes a single argument you should pass a one-element tuple.
    fn bind<A>(self) -> BindArgs<Self, A>;
}

impl<T> BindArgType for T {
    fn bind<A>(self) -> BindArgs<Self, A> {
        BindArgs(self, PhantomData)
    }
}

/// Trait to convert a tuple of argument types into a
/// Cranelift type list for signatures
pub trait IntoSigTypes {
    /// Create the type list
    fn sig_types() -> Vec<ir::AbiParam>;
}

/// Statically create a cranelift `Type` from this type.
pub trait IntoSigType {
    /// The `Type` that represents this Rust type.
    const SIG_TYPE: types::Type;
}

impl IntoSigType for i32 {
    const SIG_TYPE: types::Type = types::I32;
}

impl IntoSigType for u32 {
    const SIG_TYPE: types::Type = types::I32;
}

impl IntoSigType for i64 {
    const SIG_TYPE: types::Type = types::I64;
}

impl IntoSigType for u64 {
    const SIG_TYPE: types::Type = types::I64;
}

impl IntoSigType for f32 {
    const SIG_TYPE: types::Type = types::F32;
}

impl IntoSigType for f64 {
    const SIG_TYPE: types::Type = types::F64;
}

/// Internal trait to convert a `CallMut` into a function that can be called by WebAssembly.
/// This essentially just removes the `VMContext` parameter (for now, this might be changed
/// in the future) and extracts any closure data from the host data where appropriate.
pub trait MkTrampoline<A, O> {
    /// Build the trampoline function.
    fn make_trampoline<F, C, HostData>() -> *const VMFunctionBody
    where
        F: CallMut<Args = A, Output = O>,
        HostData: hlist::Find<Func<F>, C> + 'static;
}

/// Version of `FnMut` that constrains the arguments to only a single set of types
pub trait CallMut {
    /// The list of arguments as a tuple
    type Args;
    /// The list of returns as a tuple
    type Output;

    /// Do the call, this is equivalent to `FnMut::call_mut`
    fn call_mut(&mut self, args: Self::Args) -> Self::Output;
}

macro_rules! impl_functionlike_traits {
    ($first_a:ident $(, $rest_a:ident)*) => {
        impl<$first_a $(, $rest_a)*, __O> MkTrampoline<($first_a, $($rest_a),*), __O> for
            (($first_a, $($rest_a),*), __O)
        {
            fn make_trampoline<__F, __C, HostData>() -> *const VMFunctionBody
            where
                __F: CallMut<Args = ($first_a, $($rest_a),*), Output = __O>,
                HostData: hlist::Find<Func<__F>, __C> + 'static,
            {
                use std::{mem, ptr::NonNull};

                (|vmctx: &mut VMContext, $first_a: $first_a $(, $rest_a: $rest_a)*| {
                    let mut dangling = NonNull::dangling();
                    let func = if mem::size_of::<__F>() == 0 {
                        unsafe { dangling.as_mut() }
                    } else {
                        &mut hlist::Find::<Func<__F>, __C>::get_mut(
                            unsafe { vmctx.host_state() }
                                .downcast_mut::<HostData>()
                                .expect("Programmer error: Invalid host data"),
                        )
                        .0
                    };
                    func.call_mut(($first_a, $($rest_a),*))
                }) as fn(&mut VMContext, $first_a, $($rest_a),*) -> __F::Output as _
            }
        }

        impl<__F, $first_a $(, $rest_a)*, __O> CallMut for BindArgs<__F, ($first_a, $($rest_a),*)>
        where
            __F: FnMut($first_a, $($rest_a),*) -> __O,
        {
            type Args = ($first_a, $($rest_a),*);
            type Output = __O;

            fn call_mut(&mut self, args: Self::Args) -> Self::Output {
                let ($first_a, $($rest_a),*) = args;
                (self.0)($first_a $(, $rest_a)*)
            }
        }

        impl<$first_a $(, $rest_a)*, __O> CallMut for fn($first_a, $($rest_a),*) -> __O
        {
            type Args = ($first_a, $($rest_a),*);
            type Output = __O;

            fn call_mut(&mut self, args: Self::Args) -> Self::Output {
                let ($first_a, $($rest_a),*) = args;
                self($first_a $(, $rest_a)*)
            }
        }

        impl<$first_a $(, $rest_a)*> IntoSigTypes for ($first_a, $($rest_a),*)
        where
            $first_a: IntoSigType
            $(, $rest_a: IntoSigType)*
        {
            fn sig_types() -> Vec<ir::AbiParam> {
                vec![ir::AbiParam::new($first_a::SIG_TYPE)$(, ir::AbiParam::new($rest_a::SIG_TYPE))*]
            }
        }

        impl_functionlike_traits!($($rest_a),*);
    };
    () => {
        impl<__O> MkTrampoline<(), __O> for ((), __O) {
            fn make_trampoline<__F, __C, HostData>() -> *const VMFunctionBody
            where
                __F: CallMut<Args = (), Output = __O>,
                HostData: hlist::Find<Func<__F>, __C> + 'static,
            {
                use std::{mem, ptr::NonNull};

                (|vmctx: &mut VMContext| {
                    let mut dangling = NonNull::dangling();
                    let func = if mem::size_of::<__F>() == 0 {
                        unsafe { dangling.as_mut() }
                    } else {
                        &mut hlist::Find::<Func<__F>, __C>::get_mut(
                            unsafe { vmctx.host_state() }
                                .downcast_mut::<HostData>()
                                .expect("Programmer error: Invalid host data"),
                        )
                        .0
                    };
                    func.call_mut(())
                }) as fn(&mut VMContext) -> __F::Output as _
            }
        }

        impl<__F, __O> CallMut for BindArgs<__F, ()>
        where
            __F: FnMut() -> __O,
        {
            type Args = ();
            type Output = __O;

            fn call_mut(&mut self, _args: ()) -> Self::Output {
                (self.0)()
            }
        }

        impl<__O> CallMut for fn() -> __O {
            type Args = ();
            type Output = __O;

            fn call_mut(&mut self, _args: ()) -> Self::Output {
                self()
            }
        }

        impl IntoSigTypes for () {
            fn sig_types() -> Vec<ir::AbiParam> {
                vec![]
            }
        }
    };
}

impl<T> IntoSigTypes for T
where
    T: IntoSigType,
{
    fn sig_types() -> Vec<ir::AbiParam> {
        vec![ir::AbiParam::new(T::SIG_TYPE)]
    }
}

/// We create a module here so we can easily add required `allow` annotations
#[allow(non_snake_case)]
mod dummy {
    use super::*;

    impl_functionlike_traits!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
}

/// Separate trait for defining the `HostData` type that this requires, since adding
/// this type to `ExportsElement` would cause a circular dependency to all the
/// implementations.
pub trait HasHostData {
    /// The required host data for this part (only required for closures)
    type HostData;
}

impl<F, Rest> HasHostData for Cons<ExportDef<Func<F>>, Rest>
where
    Rest: HasHostData,
{
    type HostData = Cons<Func<F>, <Rest as HasHostData>::HostData>;
}

impl<T, Rest> HasHostData for Cons<ExportDef<Global<T>>, Rest>
where
    Rest: HasHostData,
{
    type HostData = <Rest as HasHostData>::HostData;
}

impl<Rest> HasHostData for Cons<ExportDef<Memory>, Rest>
where
    Rest: HasHostData,
{
    type HostData = <Rest as HasHostData>::HostData;
}

impl<Rest> HasHostData for Cons<ExportDef<Table>, Rest>
where
    Rest: HasHostData,
{
    type HostData = <Rest as HasHostData>::HostData;
}

impl HasHostData for Nil {
    type HostData = Nil;
}

impl<F, Rest, Counter, HostData> ExportsElement<Counter, HostData>
    for Cons<ExportDef<Func<F>>, Rest>
where
    Cons<ExportDef<Func<F>>, Rest>:
        HasHostData<HostData = Cons<Func<F>, <Rest as HasHostData>::HostData>>,
    F: CallMut,
    F::Args: IntoSigTypes,
    F::Output: IntoSigTypes,
    (F::Args, F::Output): MkTrampoline<F::Args, F::Output>,
    Rest: ExportsElement<There<Counter>, HostData>,
    HostData: hlist::Find<Func<F>, Counter> + 'static,
{
    fn build_element(self, builder: &mut Builder<'_>) -> Self::HostData {
        let calling_convention = isa::CallConv::triple_default(&builder.triple);
        let pointer_type = types::Type::triple_pointer_type(&builder.triple);
        let sig = builder.module.signatures.push(translate_signature(
            ir::Signature {
                params: F::Args::sig_types(),
                returns: F::Output::sig_types(),
                call_conv: calling_convention,
            },
            pointer_type,
        ));

        let func = builder.module.functions.push(sig);
        builder
            .module
            .exports
            .insert(self.0.name, Export::Function(func));
        builder
            .functions
            .push(<(F::Args, F::Output)>::make_trampoline::<
                F,
                Counter,
                HostData,
            >());

        Cons(self.0.val, self.1.build_element(builder))
    }
}

impl<T, Rest, Counter, HostData> ExportsElement<Counter, HostData>
    for Cons<ExportDef<Global<T>>, Rest>
where
    Cons<ExportDef<Global<T>>, Rest>: HasHostData<HostData = <Rest as HasHostData>::HostData>,
    Rest: ExportsElement<Counter, HostData>,
    T: Into<Value>,
{
    fn build_element(self, builder: &mut Builder<'_>) -> Self::HostData {
        let Global(value, mutability) = self.0.val;
        let global = match value.into() {
            Value::F32(val) => builder.module.globals.push(WasmGlobal {
                ty: types::F32,
                mutability: mutability == Mutability::Mutable,
                initializer: GlobalInit::F32Const(val as _),
            }),
            Value::F64(val) => builder.module.globals.push(WasmGlobal {
                ty: types::F64,
                mutability: mutability == Mutability::Mutable,
                initializer: GlobalInit::F64Const(val as _),
            }),
            Value::I32(val) => builder.module.globals.push(WasmGlobal {
                ty: types::I32,
                mutability: mutability == Mutability::Mutable,
                initializer: GlobalInit::I32Const(val as _),
            }),
            Value::I64(val) => builder.module.globals.push(WasmGlobal {
                ty: types::I64,
                mutability: mutability == Mutability::Mutable,
                initializer: GlobalInit::I64Const(val as _),
            }),
        };
        builder
            .module
            .exports
            .insert(self.0.name, Export::Global(global));

        self.1.build_element(builder)
    }
}

impl<Rest, Counter, HostData> ExportsElement<Counter, HostData> for Cons<ExportDef<Memory>, Rest>
where
    Cons<ExportDef<Memory>, Rest>: HasHostData<HostData = <Rest as HasHostData>::HostData>,
    Rest: ExportsElement<Counter, HostData>,
{
    fn build_element(self, builder: &mut Builder<'_>) -> Self::HostData {
        let tunables = target_tunables(builder.triple);
        let memory = builder
            .module
            .memory_plans
            .push(MemoryPlan::for_memory(self.0.val, &tunables));
        builder
            .module
            .exports
            .insert(self.0.name, Export::Memory(memory));

        self.1.build_element(builder)
    }
}

impl<Rest, Counter, HostData> ExportsElement<Counter, HostData> for Cons<ExportDef<Table>, Rest>
where
    Cons<ExportDef<Table>, Rest>: HasHostData<HostData = <Rest as HasHostData>::HostData>,
    Rest: ExportsElement<Counter, HostData>,
{
    fn build_element(self, builder: &mut Builder<'_>) -> Self::HostData {
        let tunables = target_tunables(builder.triple);
        let memory = builder
            .module
            .table_plans
            .push(TablePlan::for_table(self.0.val, &tunables));
        builder
            .module
            .exports
            .insert(self.0.name, Export::Table(memory));

        self.1.build_element(builder)
    }
}

impl<Counter, HostData> ExportsElement<Counter, HostData> for Nil {
    fn build_element(self, _builder: &mut Builder<'_>) -> Self::HostData {
        Nil
    }
}

/// Convenience trait to do all the work required to turn an export list into an
/// `InstanceHandle`.
pub trait Instantiate {
    /// Do the instantiation, consuming the export list. If you need more control, consider using
    /// `Export::build` manually (see the crate root documentation for an example).
    fn instantiate(self) -> Result<InstanceHandle, InstantiationError>;
}

impl<T> Instantiate for T
where
    T: HasHostData,
    <T as HasHostData>::HostData: 'static,
    T: Exports,
{
    fn instantiate(self) -> Result<InstanceHandle, InstantiationError> {
        let mut builder = Builder::new(&HOST);
        let data = self.build(&mut builder);

        builder.instantiate_module(data)
    }
}

/// Convenience macro for creating export lists (but not instantiating them). The result of this
/// macro is a `hlist::HList`, so you can call `.push` to add one extra `ExportDef` and use the
/// `+` operator to combine two lists.
///
/// For an example of how to use this, see the documentation at the crate root.
#[macro_export]
macro_rules! exports {
    ($name:ident: $val:expr $(, $k:ident: $v:expr)* $(,)*) => {{
        $crate::hlist::Cons($crate::ExportDef {
            name: stringify!($name).to_owned(),
            val: $val,
        }, exports!($($k:$v),*))
    }};
    () => {
        $crate::hlist::Nil
    }
}
