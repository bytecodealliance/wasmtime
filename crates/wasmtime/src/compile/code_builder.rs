use crate::Engine;
use crate::prelude::*;
use std::borrow::Cow;
use std::path::Path;

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
    pub fn new(engine: &'a Engine) -> CodeBuilder<'a> {
        CodeBuilder {
            engine,
            wasm: None,
            wasm_path: None,
            dwarf_package: None,
            dwarf_package_path: None,
            unsafe_intrinsics_import: None,
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

    pub(super) fn get_wasm(&self) -> Result<&[u8]> {
        self.wasm
            .as_deref()
            .ok_or_else(|| anyhow!("no wasm bytes have been configured"))
    }

    /// Expose Wasmtime's unsafe intrinsics under the given import name.
    ///
    /// These intrinsics provide native memory loads and stores to Wasm; they
    /// are *extremely* unsafe! If you are not absolutely sure that you need
    /// these unsafe intrinsics, *do not use them!* See the safety section below
    /// for details.
    ///
    /// This functionality is intended to be used when implementing
    /// "compile-time builtins"; that is, satisfying a Wasm import via
    /// special-cased, embedder-specific code at compile time. You should never
    /// use these intrinsics to intentionally subvert the Wasm sandbox. You
    /// should strive to implement safe functions that encapsulate your uses of
    /// these intrinsics such that, regardless of any value given as arguments,
    /// your functions *cannot* result in loading from or storing to invalid
    /// pointers, or any other kind of unsafety. See below for an example of the
    /// intended use cases.
    ///
    /// Wasmtime's unsafe intrinsics can only be exposed to Wasm components, not
    /// core modules, currently.
    ///
    /// # Safety
    ///
    /// Extreme care must be taken when using these intrinsics.
    ///
    /// All memory operated upon by these intrinsics must be reachable from the
    /// `T` in a `Store<T>`. That is, all addresses should be derived, via
    /// pointer arithmetic and transitive loads, from the result of the
    /// `store-data-address` intrinsic.
    ///
    /// Additionally, usage of these intrinsics is inherently tied to a
    /// particular `T` type. It is wildly unsafe to run a Wasm program that uses
    /// unsafe intrinsics to access the store's `T` inside a `Store<U>`. You
    /// must only run Wasm that uses unsafe intrinsics in a `Store<T>` where the
    /// `T` is the type expected by the Wasm's unsafe-intrinsics usage.
    ///
    /// Furthermore, usage of these intrinsics is not only tied to a particular
    /// `T` type, but also to `T`'s layout on the host platform. The size and
    /// alignment of `T`, the offsets of its fields, and those fields' size and
    /// alignment can all vary across not only architecture but also operating
    /// system. With care, you can define your `T` type such that its layout is
    /// identical across the platforms that you run Wasm on, allowing you to
    /// reuse the same Wasm binary and its unsafe-intrinsics usage on all your
    /// platforms. Failing that, you must only run a Wasm program that uses
    /// unsafe intrinsics on the host platform that its unsafe-intrinsic usafe
    /// is specialized to. See the portability section and example below for
    /// more details.
    ///
    /// Finally, every pointer loaded from or stored to must:
    ///
    /// * Be non-null
    ///
    /// * Be aligned to the access type's natural alignment (e.g. 8-byte alignment
    ///   for `u64`, 4-byte alignment for `u32`, etc...)
    ///
    /// * Point to a memory block that is valid to read from (for loads) or
    ///   valid to write to (for stores)
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
    /// You are *strongly* encouraged to add assertions for the layout
    /// properties that your unsafe-intrinsics usage's safety relies upon:
    ///
    /// ```rust
    /// /// This type is used as `wasmtime::Store<MyData>` and accessed by Wasm via
    /// /// unsafe intrinsics.
    /// #[repr(C)]
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
    /// All native load and store intinsics should operate on memory addresses
    /// that are derived from a call to this intrinsic. If you want to expose
    /// data for raw memory access by Wasm, put it inside the `T` in your
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
    /// * Only access `u8`, `u16`, `u32`, and `u64` data via these intrinsics
    ///
    /// * If you need to access other types of data, encode it into those types
    ///   and then access the encoded data from the intrinsics
    ///
    /// * Use `union`s to encode pointers and pointer-sized data as a `u64` and
    ///   then access it via the `u64-native-{load,store}` intrinsics:
    ///
    ///   ```rust
    ///   union ExposedPointer {
    ///       pointer: *mut u8,
    ///       _layout: u64,
    ///   }
    ///
    ///   static _EXPOSED_POINTER_LAYOUT_ASSERTIONS: () = {
    ///       assert!(core::mem::size_of::<ExposedPointer>() == 8);
    ///       assert!(core::mem::align_of::<ExposedPointer>() == 8);
    ///   };
    ///   ```
    ///
    /// # Example
    ///
    /// TODO FITZGEN
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
        let wasm = self.get_wasm()?;
        let dwarf_package = self.get_dwarf_package();
        ensure!(
            self.unsafe_intrinsics_import.is_none(),
            "`CodeBuilder::expose_unsafe_intrinsics` can only be used with components"
        );
        let (v, _) =
            super::build_module_artifacts(self.engine, &wasm, dwarf_package.as_deref(), &())?;
        Ok(v)
    }

    /// Same as [`CodeBuilder::compile_module_serialized`] except that it
    /// compiles a serialized [`Component`](crate::component::Component)
    /// instead of a module.
    #[cfg(feature = "component-model")]
    pub fn compile_component_serialized(&self) -> Result<Vec<u8>> {
        let bytes = self.get_wasm()?;
        let (v, _) = super::build_component_artifacts(
            self.engine,
            &bytes,
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
        let compiler = self.0.compiler();
        compiler.triple().hash(hasher);
        compiler.flags().hash(hasher);
        compiler.isa_flags().hash(hasher);

        // Hash configuration state read for compilation
        let config = self.0.config();
        self.0.tunables().hash(hasher);
        self.0.features().hash(hasher);
        config.wmemcheck.hash(hasher);

        // Catch accidental bugs of reusing across crate versions.
        config.module_version.hash(hasher);
    }
}
