use crate::Engine;
use crate::prelude::*;
use std::borrow::Cow;
use std::path::Path;

#[cfg(feature = "compile-time-builtins")]
use crate::hash_map::HashMap;

#[cfg(feature = "compile-time-builtins")]
mod compile_time_builtins;

/// Builder-style structure used to create a [`Module`](crate::module::Module) or
/// pre-compile a module to a serialized list of bytes.
///
/// This structure can be used for more advanced configuration when compiling a
/// WebAssembly module. Most configuration can use simpler constructors such as:
///
/// * [`Module::new`](crate::Module::new)
/// * [`Module::from_file`](crate::Module::from_file)
/// * [`Module::from_binary`](crate::Module::from_binary)
///
/// Note that a [`CodeBuilder`] always involves compiling WebAssembly bytes
/// to machine code. To deserialize a list of bytes use
/// [`Module::deserialize`](crate::Module::deserialize) instead.
///
/// A [`CodeBuilder`] requires a source of WebAssembly bytes to be configured
/// before calling [`compile_module_serialized`] or [`compile_module`]. This can
/// be provided with either the [`wasm_binary`] or [`wasm_binary_file`] method.
/// Note that only a single source of bytes can be provided.
///
/// # WebAssembly Text Format
///
/// This builder supports the WebAssembly Text Format (`*.wat` files) through
/// the [`CodeBuilder::wasm_binary_or_text`] and
/// [`CodeBuilder::wasm_binary_or_text_file`] methods. These methods
/// automatically convert WebAssembly text files to binary. Note though that
/// this behavior is disabled if the `wat` crate feature is not enabled.
///
/// [`compile_module_serialized`]: CodeBuilder::compile_module_serialized
/// [`compile_module`]: CodeBuilder::compile_module
/// [`wasm_binary`]: CodeBuilder::wasm_binary
/// [`wasm_binary_file`]: CodeBuilder::wasm_binary_file
pub struct CodeBuilder<'a> {
    pub(super) engine: &'a Engine,
    wasm: Option<Cow<'a, [u8]>>,
    wasm_path: Option<Cow<'a, Path>>,
    dwarf_package: Option<Cow<'a, [u8]>>,
    dwarf_package_path: Option<Cow<'a, Path>>,
    unsafe_intrinsics_import: Option<String>,

    /// A map from import name to the Wasm bytes of the associated compile-time
    /// builtin and its file path, if any.
    //
    // XXX: we can't use `'a` here without forcing us to change a bunch of
    // callers because `HashMap` has a `Drop` implementation, so `dropck` thinks
    // that all `'a` borrows could be used by `CodeBuilder`'s `Drop`, which
    // means that a bunch of the existing calls to `.wasm_bytes` and such would
    // need to be reordered with the `CodeBuilder`'s construction.
    #[cfg(feature = "compile-time-builtins")]
    compile_time_builtins: HashMap<Cow<'a, str>, Cow<'a, [u8]>>,
}

/// Return value of [`CodeBuilder::hint`]
pub enum CodeHint {
    /// Hint that the code being compiled is a module.
    Module,
    /// Hint that the code being compiled is a component.
    Component,
}

impl<'a> CodeBuilder<'a> {
    /// Creates a new builder which will insert modules into the specified
    /// [`Engine`].
    pub fn new(engine: &'a Engine) -> Self {
        CodeBuilder {
            engine,
            wasm: None,
            wasm_path: None,
            dwarf_package: None,
            dwarf_package_path: None,
            unsafe_intrinsics_import: None,
            #[cfg(feature = "compile-time-builtins")]
            compile_time_builtins: HashMap::default(),
        }
    }

    /// Configures the WebAssembly binary that is being compiled.
    ///
    /// The `wasm_bytes` parameter must be a binary WebAssembly file.
    /// This will be stored within the [`CodeBuilder`] for processing later when
    /// compilation is finalized.
    ///
    /// The optional `wasm_path` parameter is the path to the `wasm_bytes` on
    /// disk, if any. This may be used for diagnostics and other
    /// debugging-related purposes, but this method will not read the path
    /// specified.
    ///
    /// # Errors
    ///
    /// This method will return an error if WebAssembly bytes have already been
    /// configured.
    pub fn wasm_binary(
        &mut self,
        wasm_bytes: impl Into<Cow<'a, [u8]>>,
        wasm_path: Option<&'a Path>,
    ) -> Result<&mut Self> {
        if self.wasm.is_some() {
            bail!("cannot configure wasm bytes twice");
        }
        self.wasm = Some(wasm_bytes.into());
        self.wasm_path = wasm_path.map(|p| p.into());

        if self.wasm_path.is_some() {
            self.dwarf_package_from_wasm_path()?;
        }

        Ok(self)
    }

    /// Equivalent of [`CodeBuilder::wasm_binary`] that also accepts the
    /// WebAssembly text format.
    ///
    /// This method will configure the WebAssembly binary to be compiled. The
    /// input `wasm_bytes` may either be the wasm text format or the binary
    /// format. If the `wat` crate feature is enabled, which is enabled by
    /// default, then the text format will automatically be converted to the
    /// binary format.
    ///
    /// # Errors
    ///
    /// This method will return an error if WebAssembly bytes have already been
    /// configured. This method will also return an error if `wasm_bytes` is the
    /// wasm text format and the text syntax is not valid.
    pub fn wasm_binary_or_text(
        &mut self,
        wasm_bytes: &'a [u8],
        wasm_path: Option<&'a Path>,
    ) -> Result<&mut Self> {
        #[cfg(feature = "wat")]
        let wasm_bytes = wat::parse_bytes(wasm_bytes).map_err(|mut e| {
            if let Some(path) = wasm_path {
                e.set_path(path);
            }
            e
        })?;
        self.wasm_binary(wasm_bytes, wasm_path)
    }

    /// Reads the `file` specified for the WebAssembly bytes that are going to
    /// be compiled.
    ///
    /// This method will read `file` from the filesystem and interpret it
    /// as a WebAssembly binary.
    ///
    /// A DWARF package file will be probed using the root of `file` and with a
    /// `.dwp` extension. If found, it will be loaded and DWARF fusion
    /// performed.
    ///
    /// # Errors
    ///
    /// This method will return an error if WebAssembly bytes have already been
    /// configured.
    ///
    /// If `file` can't be read or an error happens reading it then that will
    /// also be returned.
    ///
    /// If DWARF fusion is performed and the DWARF packaged file cannot be read
    /// then an error will be returned.
    pub fn wasm_binary_file(&mut self, file: &'a Path) -> Result<&mut Self> {
        let wasm = std::fs::read(file)
            .with_context(|| format!("failed to read input file: {}", file.display()))?;
        self.wasm_binary(wasm, Some(file))
    }

    /// Equivalent of [`CodeBuilder::wasm_binary_file`] that also accepts the
    /// WebAssembly text format.
    ///
    /// This method is will read the file at `path` and interpret the contents
    /// to determine if it's the wasm text format or binary format. The file
    /// extension of `file` is not consulted. The text format is automatically
    /// converted to the binary format if the crate feature `wat` is active.
    ///
    /// # Errors
    ///
    /// In addition to the errors returned by [`CodeBuilder::wasm_binary_file`]
    /// this may also fail if the text format is read and the syntax is invalid.
    pub fn wasm_binary_or_text_file(&mut self, file: &'a Path) -> Result<&mut Self> {
        #[cfg(feature = "wat")]
        {
            let wasm = wat::parse_file(file)?;
            self.wasm_binary(wasm, Some(file))
        }
        #[cfg(not(feature = "wat"))]
        {
            self.wasm_binary_file(file)
        }
    }

    /// Get the Wasm to be compiled.
    ///
    /// When using compile-time builtins, compose the builtins and the guest
    /// Wasm together, before returning the Wasm.
    pub(super) fn get_wasm(&self) -> Result<Cow<'_, [u8]>> {
        let wasm = self
            .wasm
            .as_deref()
            .ok_or_else(|| format_err!("no wasm bytes have been configured"))?;

        #[cfg(not(feature = "compile-time-builtins"))]
        {
            Ok(wasm.into())
        }

        #[cfg(feature = "compile-time-builtins")]
        {
            self.compose_compile_time_builtins(wasm)
                .context("failed to compose compile-time builtins with the main Wasm")
        }
    }

    /// Expose Wasmtime's unsafe intrinsics under the given import name.
    ///
    /// These intrinsics provide native memory loads and stores to Wasm; they
    /// are *extremely* unsafe! If you are not absolutely sure that you need
    /// these unsafe intrinsics, *do not use them!* See the safety section below
    /// for details.
    ///
    /// This functionality is intended to be used when implementing
    /// [compile-time builtins][CodeBuilder::compile_time_builtins_binary]; that
    /// is, satisfying a Wasm import via special-cased, embedder-specific code
    /// at compile time. You should never use these intrinsics to intentionally
    /// subvert the Wasm sandbox. You should strive to implement safe functions
    /// that encapsulate your uses of these intrinsics such that, regardless of
    /// any value given as arguments, your functions *cannot* result in loading
    /// from or storing to invalid pointers, or any other kind of unsafety. See
    /// below for an example of the intended use cases.
    ///
    /// Wasmtime's unsafe intrinsics can only be exposed to Wasm components, not
    /// core modules, currently.
    ///
    /// Note that when compile-time builtins are defined on a `CodeBuilder`,
    /// only the compile-time builtins can import the unsafe intrinsics, and the
    /// main guest program cannot import them.
    ///
    /// # Safety
    ///
    /// Extreme care must be taken when using these intrinsics.
    ///
    /// All loads of or stores to pointers derived from `store-data-address` are
    /// inherently tied to a particular `T` type in a `Store<T>`. It is wildly
    /// unsafe to run a Wasm program that uses unsafe intrinsics to access the
    /// store's `T` inside a `Store<U>`. You must only run Wasm that uses unsafe
    /// intrinsics in a `Store<T>` where the `T` is the type expected by the
    /// Wasm's unsafe-intrinsics usage.
    ///
    /// Furthermore, usage of these intrinsics is not only tied to a particular
    /// `T` type, but also to `T`'s layout on the host platform. The size and
    /// alignment of `T`, the offsets of its fields, and those fields' size and
    /// alignment can all vary across not only architecture but also operating
    /// system. With care, you can define your `T` type such that its layout is
    /// identical across the platforms that you run Wasm on, allowing you to
    /// reuse the same Wasm binary and its unsafe-intrinsics usage on all your
    /// platforms. Failing that, you must only run a Wasm program that uses
    /// unsafe intrinsics on the host platform that its unsafe-intrinsic usage
    /// is specialized to. See the portability section and example below for
    /// more details.
    ///
    /// You are *strongly* encouraged to add assertions for the layout
    /// properties that your unsafe-intrinsic usage's safety relies upon:
    ///
    /// ```rust
    /// /// This type is used as `wasmtime::Store<MyData>` and accessed by Wasm via
    /// /// unsafe intrinsics.
    /// #[repr(C, align(8))]
    /// struct MyData {
    ///     id: u64,
    ///     counter: u32,
    ///     buf: [u8; 4],
    /// }
    ///
    /// // Assert that the layout is what our Wasm's unsafe-intrinsics usage expects.
    /// static _MY_DATA_LAYOUT_ASSERTIONS: () = {
    ///     assert!(core::mem::size_of::<MyData>() == 16);
    ///     assert!(core::mem::align_of::<MyData>() == 8);
    ///     assert!(core::mem::offset_of!(MyData, id) == 0);
    ///     assert!(core::mem::offset_of!(MyData, counter) == 8);
    ///     assert!(core::mem::offset_of!(MyData, buf) == 12);
    /// };
    /// ```
    ///
    /// Finally, every pointer loaded from or stored to must:
    ///
    /// * Be non-null
    ///
    /// * Be aligned to the access type's natural alignment (e.g. 8-byte alignment
    ///   for `u64`, 4-byte alignment for `u32`, etc...)
    ///
    /// * Point to a memory block that is valid to read from (for loads) or
    ///   valid to write to (for stores) under Rust's pointer provenance rules
    ///
    /// * Point to a memory block that is at least as large as the access type's
    ///   natural size (e.g. 1 byte for `u8`, 2 bytes for `u16`, etc...)
    ///
    /// * Point to a memory block that is not accessed concurrently by any other
    ///   threads
    ///
    /// Failure to uphold any of these invariants will lead to unsafety,
    /// undefined behavior, and/or data races.
    ///
    /// # Intrinsics
    ///
    /// | Name                 | Parameters   | Results |
    /// |----------------------|--------------|---------|
    /// | `u8-native-load`     | `u64`        | `u8`    |
    /// | `u16-native-load`    | `u64`        | `u16`   |
    /// | `u32-native-load`    | `u64`        | `u32`   |
    /// | `u64-native-load`    | `u64`        | `u64`   |
    /// | `u8-native-store`    | `u64`, `u8`  | -       |
    /// | `u16-native-load`    | `u64`, `u16` | -       |
    /// | `u32-native-load`    | `u64`, `u32` | -       |
    /// | `u64-native-load`    | `u64`, `u64` | -       |
    /// | `store-data-address` | -            | `u64`   |
    ///
    /// ## `*-native-load`
    ///
    /// These intrinsics perform an unsandboxed, unsynchronized load from native
    /// memory, using the native endianness.
    ///
    /// ## `*-native-store`
    ///
    /// These intrinsics perform an unsandboxed, unsynchronized store to native
    /// memory, using the native endianness.
    ///
    /// ## `store-data-address`
    ///
    /// This intrinsic function returns the pointer to the embedder's `T` data
    /// inside a `Store<T>`.
    ///
    /// In general, all native load and store intinsics should operate on memory
    /// addresses that are derived from a call to this intrinsic. If you want to
    /// expose data for raw memory access by Wasm, put it inside the `T` in your
    /// `Store<T>` and Wasm's access to that data should derive from this
    /// intrinsic.
    ///
    /// # Portability
    ///
    /// Loads and stores are always performed using the architecture's native
    /// endianness.
    ///
    /// Addresses passed to and returned from these intrinsics are always
    /// 64-bits large. The upper half of the value is simply ignored on 32-bit
    /// architectures.
    ///
    /// With care, you can design your store's `T` type such that accessing it
    /// via these intrinsics is portable, and you can reuse a single Wasm binary
    /// (and its set of intrinsic calls) across all of the platforms, with the
    /// following rules of thumb:
    ///
    /// * Only access `u8`, `u16`, `u32`, and `u64` data via these intrinsics.
    ///
    /// * If you need to access other types of data, encode it into those types
    ///   and then access the encoded data from the intrinsics.
    ///
    /// * Use `union`s to encode pointers and pointer-sized data as a `u64` and
    ///   then access it via the `u64-native-{load,store}` intrinsics. See
    ///   `ExposedPointer` in the example below.
    ///
    /// # Example
    ///
    /// The following example shows how you can use unsafe intrinsics and
    /// compile-time builtins to give Wasm direct zero-copy access to a host
    /// buffer.
    ///
    /// ```rust
    /// use std::mem;
    /// use wasmtime::*;
    ///
    /// // A `*mut u8` pointer that is exposed directly to Wasm via unsafe intrinsics.
    /// #[repr(align(8))]
    /// union ExposedPointer {
    ///     pointer: *mut u8,
    ///     padding: u64,
    /// }
    ///
    /// static _EXPOSED_POINTER_LAYOUT_ASSERTIONS: () = {
    ///     assert!(mem::size_of::<ExposedPointer>() == 8);
    ///     assert!(mem::align_of::<ExposedPointer>() == 8);
    /// };
    ///
    /// impl ExposedPointer {
    ///     /// Wrap the given pointer into an `ExposedPointer`.
    ///     fn new(pointer: *mut u8) -> Self {
    ///         // NB: Zero-initialize to avoid potential footguns with accessing
    ///         // undefined bytes.
    ///         let mut p = Self { padding: 0 };
    ///         p.pointer = pointer;
    ///         p
    ///     }
    ///
    ///     /// Get the wrapped pointer.
    ///     fn get(&self) -> *mut u8 {
    ///         unsafe { self.pointer }
    ///     }
    /// }
    ///
    /// /// This is the `T` type we will put inside our
    /// /// `wasmtime::Store<T>`s. It contains a pointer to a heap-allocated buffer
    /// /// in host memory, which we will give Wasm zero-copy access to via unsafe
    /// /// intrinsics.
    /// #[repr(C)]
    /// struct StoreData {
    ///     buf_ptr: ExposedPointer,
    ///     buf_len: u64,
    /// }
    ///
    /// static _STORE_DATA_LAYOUT_ASSERTIONS: () = {
    ///     assert!(mem::size_of::<StoreData>() == 16);
    ///     assert!(mem::align_of::<StoreData>() == 8);
    ///     assert!(mem::offset_of!(StoreData, buf_ptr) == 0);
    ///     assert!(mem::offset_of!(StoreData, buf_len) == 8);
    /// };
    ///
    /// impl Drop for StoreData {
    ///     fn drop(&mut self) {
    ///         let len = usize::try_from(self.buf_len).unwrap();
    ///         let ptr = std::ptr::slice_from_raw_parts_mut(self.buf_ptr.get(), len);
    ///         unsafe {
    ///             let _ = Box::from_raw(ptr);
    ///         }
    ///     }
    /// }
    ///
    /// impl StoreData {
    ///     /// Create a new `StoreData`, allocating an inner buffer containing
    ///     /// `bytes`.
    ///     fn new(bytes: impl IntoIterator<Item = u8>) -> Self {
    ///         let buf: Box<[u8]> = bytes.into_iter().collect();
    ///         let ptr = Box::into_raw(buf);
    ///         Self {
    ///             buf_ptr: ExposedPointer::new(ptr.cast::<u8>()),
    ///             buf_len: u64::try_from(ptr.len()).unwrap(),
    ///         }
    ///     }
    ///
    ///     /// Get the inner buffer as a shared slice.
    ///     fn buf(&self) -> &[u8] {
    ///         let ptr = self.buf_ptr.get().cast_const();
    ///         let len = usize::try_from(self.buf_len).unwrap();
    ///         unsafe {
    ///             std::slice::from_raw_parts(ptr, len)
    ///         }
    ///     }
    /// }
    ///
    /// # fn main() -> Result<()> {
    /// // Enable function inlining during compilation. If you are using unsafe intrinsics, you
    /// // almost assuredly want them inlined to avoid function call overheads.
    /// let mut config = Config::new();
    /// config.compiler_inlining(true);
    ///
    /// let engine = Engine::new(&config)?;
    /// let linker = wasmtime::component::Linker::new(&engine);
    ///
    /// // Create a new builder for configuring a Wasm compilation.
    /// let mut builder = CodeBuilder::new(&engine);
    ///
    /// // Allow the code we are building to use Wasmtime's unsafe intrinsics.
    /// //
    /// // SAFETY: we wrap all usage of the intrinsics in safe APIs and only instantiate the code
    /// // within a `Store<T>` where `T = StoreData`, as the code expects.
    /// unsafe {
    ///     builder.expose_unsafe_intrinsics("unsafe-intrinsics");
    /// }
    ///
    /// // Define the compile-time builtin that encapsulates the
    /// // intrinsics' unsafety and builds a safe API on top of them.
    /// unsafe {
    ///     builder.compile_time_builtins_binary_or_text(
    ///         "safe-api",
    ///         r#"
    ///             (component
    ///                 (import "unsafe-intrinsics"
    ///                     (instance $intrinsics
    ///                         (export "store-data-address" (func (result u64)))
    ///                         (export "u64-native-load" (func (param "pointer" u64) (result u64)))
    ///                         (export "u8-native-load" (func (param "pointer" u64) (result u8)))
    ///                         (export "u8-native-store" (func (param "pointer" u64) (param "value" u8)))
    ///                     )
    ///                 )
    ///
    ///                 ;; The core Wasm module that implements the safe API.
    ///                 (core module $safe-api-impl
    ///                     (import "" "store-data-address" (func $store-data-address (result i64)))
    ///                     (import "" "u64-native-load" (func $u64-native-load (param i64) (result i64)))
    ///                     (import "" "u8-native-load" (func $u8-native-load (param i64) (result i32)))
    ///                     (import "" "u8-native-store" (func $u8-native-store (param i64 i32)))
    ///
    ///                     ;; Load the `StoreData::buf_ptr` field
    ///                     (func $get-buf-ptr (result i64)
    ///                         (call $u64-native-load (i64.add (call $store-data-address) (i64.const 0)))
    ///                     )
    ///
    ///                     ;; Load the `StoreData::buf_len` field
    ///                     (func $get-buf-len (result i64)
    ///                         (call $u64-native-load (i64.add (call $store-data-address) (i64.const 8)))
    ///                     )
    ///
    ///                     ;; Check that `$i` is within `StoreData` buffer's bounds, raising a trap
    ///                     ;; otherwise.
    ///                     (func $bounds-check (param $i i64)
    ///                         (if (i64.lt_u (local.get $i) (call $get-buf-len))
    ///                             (then (return))
    ///                             (else (unreachable))
    ///                         )
    ///                     )
    ///
    ///                     ;; A safe function to get the `i`th byte from `StoreData`'s buffer,
    ///                     ;; raising a trap on out-of-bounds accesses.
    ///                     (func (export "get") (param $i i64) (result i32)
    ///                         (call $bounds-check (local.get $i))
    ///                         (call $u8-native-load (i64.add (call $get-buf-ptr) (local.get $i)))
    ///                     )
    ///
    ///                     ;; A safe function to set the `i`th byte in `StoreData`'s buffer,
    ///                     ;; raising a trap on out-of-bounds accesses.
    ///                     (func (export "set") (param $i i64) (param $value i32)
    ///                         (call $bounds-check (local.get $i))
    ///                         (call $u8-native-store (i64.add (call $get-buf-ptr) (local.get $i))
    ///                                                (local.get $value))
    ///                     )
    ///
    ///                     ;; A safe function to get the length of the `StoreData` buffer.
    ///                     (func (export "len") (result i64)
    ///                         (call $get-buf-len)
    ///                     )
    ///                 )
    ///
    ///                 ;; Lower the imported intrinsics from component functions to core functions.
    ///                 (core func $store-data-address' (canon lower (func $intrinsics "store-data-address")))
    ///                 (core func $u64-native-load' (canon lower (func $intrinsics "u64-native-load")))
    ///                 (core func $u8-native-load' (canon lower (func $intrinsics "u8-native-load")))
    ///                 (core func $u8-native-store' (canon lower (func $intrinsics "u8-native-store")))
    ///
    ///                 ;; Instantiate our safe API implementation, passing in the lowered unsafe
    ///                 ;; intrinsics as its imports.
    ///                 (core instance $instance
    ///                     (instantiate $safe-api-impl
    ///                         (with "" (instance
    ///                             (export "store-data-address" (func $store-data-address'))
    ///                             (export "u64-native-load" (func $u64-native-load'))
    ///                             (export "u8-native-load" (func $u8-native-load'))
    ///                             (export "u8-native-store" (func $u8-native-store'))
    ///                         ))
    ///                     )
    ///                 )
    ///
    ///                 ;; Lift the safe API's exports from core functions to component functions
    ///                 ;; and export them.
    ///                 (func (export "get") (param "i" u64) (result u8)
    ///                     (canon lift (core func $instance "get"))
    ///                 )
    ///                 (func (export "set") (param "i" u64) (param "value" u8)
    ///                     (canon lift (core func $instance "set"))
    ///                 )
    ///                 (func (export "len") (result u64)
    ///                     (canon lift (core func $instance "len"))
    ///                 )
    ///             )
    ///         "#.as_bytes(),
    ///         None,
    ///     )?;
    /// }
    ///
    /// // Provide the guest Wasm that we are compiling, which uses the safe API we
    /// // implemented as a compile-time builtin.
    /// builder.wasm_binary_or_text(
    ///     r#"
    ///         (component
    ///             ;; Import the safe API.
    ///             (import "safe-api"
    ///                 (instance $safe-api
    ///                     (export "get" (func (param "i" u64) (result u8)))
    ///                     (export "set" (func (param "i" u64) (param "value" u8)))
    ///                     (export "len" (func (result u64)))
    ///                 )
    ///             )
    ///
    ///             ;; Define this component's core module implementation.
    ///             (core module $main-impl
    ///                 (import "" "get" (func $get (param i64) (result i32)))
    ///                 (import "" "set" (func $set (param i64 i32)))
    ///                 (import "" "len" (func $len (result i64)))
    ///
    ///                 (func (export "main")
    ///                     (local $i i64)
    ///                     (local $n i64)
    ///
    ///                     (local.set $i (i64.const 0))
    ///                     (local.set $n (call $len))
    ///
    ///                     (loop $loop
    ///                         ;; When we have iterated over every byte in the
    ///                         ;; buffer, exit.
    ///                         (if (i64.ge_u (local.get $i) (local.get $n))
    ///                             (then (return)))
    ///
    ///                         ;; Increment the `i`th byte in the buffer.
    ///                         (call $set (local.get $i)
    ///                                    (i32.add (call $get (local.get $i))
    ///                                             (i32.const 1)))
    ///
    ///                         ;; Increment `i` and continue to the next iteration
    ///                         ;; of the loop.
    ///                         (local.set $i (i64.add (local.get $i) (i64.const 1)))
    ///                         (br $loop)
    ///                     )
    ///                 )
    ///             )
    ///
    ///             ;; Lower the imported safe APIs from component functions to core functions.
    ///             (core func $get' (canon lower (func $safe-api "get")))
    ///             (core func $set' (canon lower (func $safe-api "set")))
    ///             (core func $len' (canon lower (func $safe-api "len")))
    ///
    ///             ;; Instantiate our module, providing the lowered safe APIs as imports.
    ///             (core instance $instance
    ///                 (instantiate $main-impl
    ///                     (with "" (instance
    ///                         (export "get" (func $get'))
    ///                         (export "set" (func $set'))
    ///                         (export "len" (func $len'))
    ///                     ))
    ///                 )
    ///             )
    ///
    ///             ;; Lift the implementation's `main` from a core function to a component function
    ///             ;; and export it!
    ///             (func (export "main")
    ///                 (canon lift (core func $instance "main"))
    ///             )
    ///         )
    ///     "#.as_bytes(),
    ///     None,
    /// )?;
    ///
    /// // Finish the builder and compile the component.
    /// let component = builder.compile_component()?;
    ///
    /// // Create a new `Store<StoreData>`, wrapping a buffer of the given elements.
    /// let mut store = Store::new(&engine, StoreData::new([0, 10, 20, 30, 40, 50]));
    ///
    /// // Instantiate our component into the store.
    /// let instance = linker.instantiate(&mut store, &component)?;
    ///
    /// // Get the instance's exported `main` function and call it.
    /// instance
    ///     .get_typed_func::<(), ()>(&mut store, "main")?
    ///     .call(&mut store, ())?;
    ///
    /// // Our `StoreData`'s buffer had each element incremented directly from Wasm!
    /// assert_eq!(store.data().buf(), &[1, 11, 21, 31, 41, 51]);
    /// # Ok(())
    /// # }
    /// ```
    pub unsafe fn expose_unsafe_intrinsics(&mut self, import_name: impl Into<String>) -> &mut Self {
        self.unsafe_intrinsics_import = Some(import_name.into());
        self
    }

    /// Explicitly specify DWARF `.dwp` path.
    ///
    /// # Errors
    ///
    /// This method will return an error if the `.dwp` file has already been set
    /// through [`CodeBuilder::dwarf_package`] or auto-detection in
    /// [`CodeBuilder::wasm_binary_file`].
    ///
    /// This method will also return an error if `file` cannot be read.
    pub fn dwarf_package_file(&mut self, file: &Path) -> Result<&mut Self> {
        if self.dwarf_package.is_some() {
            bail!("cannot call `dwarf_package` or `dwarf_package_file` twice");
        }

        let dwarf_package = std::fs::read(file)
            .with_context(|| format!("failed to read dwarf input file: {}", file.display()))?;
        self.dwarf_package_path = Some(Cow::Owned(file.to_owned()));
        self.dwarf_package = Some(dwarf_package.into());

        Ok(self)
    }

    fn dwarf_package_from_wasm_path(&mut self) -> Result<&mut Self> {
        let dwarf_package_path_buf = self.wasm_path.as_ref().unwrap().with_extension("dwp");
        if dwarf_package_path_buf.exists() {
            return self.dwarf_package_file(dwarf_package_path_buf.as_path());
        }

        Ok(self)
    }

    /// Gets the DWARF package.
    pub(super) fn get_dwarf_package(&self) -> Option<&[u8]> {
        self.dwarf_package.as_deref()
    }

    /// Set the DWARF package binary.
    ///
    /// Initializes `dwarf_package` from `dwp_bytes` in preparation for
    /// DWARF fusion. Allows the DWARF package to be supplied as a byte array
    /// when the file probing performed in `wasm_file` is not appropriate.
    ///
    /// # Errors
    ///
    /// Returns an error if the `*.dwp` file is already set via auto-probing in
    /// [`CodeBuilder::wasm_binary_file`] or explicitly via
    /// [`CodeBuilder::dwarf_package_file`].
    pub fn dwarf_package(&mut self, dwp_bytes: &'a [u8]) -> Result<&mut Self> {
        if self.dwarf_package.is_some() {
            bail!("cannot call `dwarf_package` or `dwarf_package_file` twice");
        }
        self.dwarf_package = Some(dwp_bytes.into());
        Ok(self)
    }

    /// Returns a hint, if possible, of what the provided bytes are.
    ///
    /// This method can be use to detect what the previously supplied bytes to
    /// methods such as [`CodeBuilder::wasm_binary_or_text`] are. This will
    /// return whether a module or a component was found in the provided bytes.
    ///
    /// This method will return `None` if wasm bytes have not been configured
    /// or if the provided bytes don't look like either a component or a
    /// module.
    pub fn hint(&self) -> Option<CodeHint> {
        let wasm = self.wasm.as_ref()?;
        if wasmparser::Parser::is_component(wasm) {
            Some(CodeHint::Component)
        } else if wasmparser::Parser::is_core_wasm(wasm) {
            Some(CodeHint::Module)
        } else {
            None
        }
    }

    /// Finishes this compilation and produces a serialized list of bytes.
    ///
    /// This method requires that either [`CodeBuilder::wasm_binary`] or
    /// related methods were invoked prior to indicate what is being compiled.
    ///
    /// This method will block the current thread until compilation has
    /// finished, and when done the serialized artifact will be returned.
    ///
    /// Note that this method will never cache compilations, even if the
    /// `cache` feature is enabled.
    ///
    /// # Errors
    ///
    /// This can fail if the input wasm module was not valid or if another
    /// compilation-related error is encountered.
    pub fn compile_module_serialized(&self) -> Result<Vec<u8>> {
        ensure!(
            self.unsafe_intrinsics_import.is_none(),
            "`CodeBuilder::expose_unsafe_intrinsics` can only be used with components"
        );

        #[cfg(feature = "compile-time-builtins")]
        ensure!(
            self.get_compile_time_builtins().is_empty(),
            "compile-time builtins can only be used with components"
        );

        let wasm = self.get_wasm()?;
        let dwarf_package = self.get_dwarf_package();
        let (v, _) =
            super::build_module_artifacts(self.engine, &wasm, dwarf_package.as_deref(), &())?;
        Ok(v)
    }

    /// Same as [`CodeBuilder::compile_module_serialized`] except that it
    /// compiles a serialized [`Component`](crate::component::Component)
    /// instead of a module.
    #[cfg(feature = "component-model")]
    pub fn compile_component_serialized(&self) -> Result<Vec<u8>> {
        let wasm = self.get_wasm()?;
        let (v, _) = super::build_component_artifacts(
            self.engine,
            &wasm,
            None,
            self.get_unsafe_intrinsics_import(),
            &(),
        )?;
        Ok(v)
    }

    pub(super) fn get_unsafe_intrinsics_import(&self) -> Option<&str> {
        self.unsafe_intrinsics_import.as_deref()
    }
}

/// This is a helper struct used when caching to hash the state of an `Engine`
/// used for module compilation.
///
/// The hash computed for this structure is used to key the global wasmtime
/// cache and dictates whether artifacts are reused. Consequently the contents
/// of this hash dictate when artifacts are or aren't re-used.
pub struct HashedEngineCompileEnv<'a>(pub &'a Engine);

impl std::hash::Hash for HashedEngineCompileEnv<'_> {
    fn hash<H: std::hash::Hasher>(&self, hasher: &mut H) {
        // Hash the compiler's state based on its target and configuration.
        if let Some(compiler) = self.0.compiler() {
            compiler.triple().hash(hasher);
            compiler.flags().hash(hasher);
            compiler.isa_flags().hash(hasher);
        }

        // Hash configuration state read for compilation
        let config = self.0.config();
        self.0.tunables().hash(hasher);
        self.0.features().hash(hasher);
        config.wmemcheck.hash(hasher);

        // Catch accidental bugs of reusing across crate versions.
        config.module_version.hash(hasher);
    }
}
