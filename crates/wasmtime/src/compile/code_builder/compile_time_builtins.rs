use super::*;
use crate::ToWasmtimeResult as _;
use wasm_compose::graph::{CompositionGraph, EncodeOptions, ExportIndex, ImportIndex, InstanceId};

/// Returns the instance imports of `component`, in declaration order.
fn instance_imports<'c>(
    component: &'c wasm_compose::graph::Component<'_>,
) -> impl Iterator<Item = (ImportIndex, &'c str)> {
    component.imports().filter_map(|(index, name, ty)| {
        matches!(ty, wasmparser::ComponentTypeRef::Instance(_)).then_some((index, name))
    })
}

/// Composes `main_wasm` with the compile-time builtins it imports.
///
/// Each imported builtin is instantiated once and satisfies every instance
/// import named after it, including other builtins' imports. Other imports,
/// such as the unsafe intrinsics, remain imports, merged by name.
fn compose(
    main_wasm: &[u8],
    builtins: &HashMap<Cow<'_, str>, Cow<'_, [u8]>>,
    features: wasmparser::WasmFeatures,
) -> Result<Vec<u8>> {
    let mut validator = wasmparser::Validator::new_with_features(features);
    let mut graph = CompositionGraph::new();

    let main = wasm_compose::graph::Component::from_bytes(&mut validator, "main", main_wasm)
        .to_wasmtime_result()?;
    let main_imports: Vec<String> = instance_imports(&main)
        .map(|(_, name)| name.to_owned())
        .collect();
    let main = graph.add_component(main).to_wasmtime_result()?;
    let main = graph.instantiate(main).to_wasmtime_result()?;

    // Instantiate in import order, so the output doesn't depend on hash map
    // order.
    let mut instances: Vec<(&str, InstanceId)> = Vec::new();
    for name in &main_imports {
        let Some((name, bytes)) = builtins.get_key_value(name.as_str()) else {
            continue;
        };

        // Graph labels must be unique and valid but aren't encoded, so number
        // them rather than reuse import names, which could clash with `main`.
        let label = format!("builtin{}", instances.len());

        // wasm-compose's errors use the label, so name the builtin here.
        let component = wasm_compose::graph::Component::from_bytes(&mut validator, label, &**bytes)
            .to_wasmtime_result()
            .with_context(|| format!("invalid compile-time builtin `{name}`"))?;
        let component = graph.add_component(component).to_wasmtime_result()?;

        instances.push((name, graph.instantiate(component).to_wasmtime_result()?));
    }

    // Collect the connections first, since connecting needs the graph
    // mutably.
    let mut connections: Vec<(InstanceId, InstanceId, ImportIndex)> = Vec::new();
    for target in core::iter::once(main).chain(instances.iter().map(|&(_, instance)| instance)) {
        let (_, component) = graph.get_component_of_instance(target).unwrap();
        for (import, name) in instance_imports(component) {
            if let Some(&(_, source)) = instances.iter().find(|&&(builtin, _)| builtin == name) {
                connections.push((source, target, import));
            }
        }
    }

    for (source, target, import) in connections {
        graph
            .connect(source, None::<ExportIndex>, target, import)
            .to_wasmtime_result()?;
    }

    graph
        .encode(EncodeOptions {
            define_components: true,
            export: Some(main),
            validate: false,
        })
        .to_wasmtime_result()
}

impl<'a> CodeBuilder<'a> {
    pub(crate) fn get_compile_time_builtins(&self) -> &HashMap<Cow<'a, str>, Cow<'a, [u8]>> {
        &self.compile_time_builtins
    }

    pub(super) fn compose_compile_time_builtins<'b>(
        &self,
        main_wasm: &'b [u8],
    ) -> Result<Cow<'b, [u8]>> {
        if self.get_compile_time_builtins().is_empty() {
            return Ok(main_wasm.into());
        }

        let imports = self.check_imports_for_compile_time_builtins(&main_wasm)?;
        let builtins = self.get_compile_time_builtins();

        // Nothing to compose if the main Wasm imports none of the builtins.
        if !imports.iter().any(|import| builtins.contains_key(*import)) {
            drop(imports);
            return Ok(main_wasm.into());
        }

        Ok(compose(main_wasm, builtins, self.engine.features())?.into())
    }

    /// Check that the main Wasm doesn't import unsafe intrinsics, keeping the
    /// TCB to just the compile-time builtins' implementation.
    ///
    /// Returns the Wasm's top-level instance imports, so that composition can
    /// be skipped when none of them is a compile-time builtin.
    fn check_imports_for_compile_time_builtins<'b>(
        &self,
        main_wasm: &'b [u8],
    ) -> Result<crate::hash_set::HashSet<&'b str>, Error> {
        let intrinsics_import = self.unsafe_intrinsics_import.as_deref().ok_or_else(|| {
            format_err!(
                "must configure the unsafe-intrinsics import when using compile-time builtins"
            )
        })?;

        let mut instance_imports = crate::hash_set::HashSet::new();
        let parser = wasmparser::Parser::new(0);
        let mut level = 0;

        for payload in parser.parse_all(main_wasm) {
            match payload? {
                wasmparser::Payload::Version { .. } => {
                    level += 1;
                }
                wasmparser::Payload::End(_) => {
                    level -= 1;
                }
                wasmparser::Payload::ComponentImportSection(imports) if level == 1 => {
                    for imp in imports.into_iter() {
                        let imp = imp?;
                        // Ideally we would simply choose a new import name that
                        // doesn't conflict with the main Wasm's imports and
                        // plumb that through to the compile-time builtins
                        // regardless of the import name that they use, but
                        // unfortunately the `wasm-compose` API is not powerful
                        // enough for us to do all that.
                        ensure!(
                            imp.name.name != intrinsics_import,
                            "main Wasm cannot import the unsafe intrinsics (`{intrinsics_import}`) \
                             when using compile-time builtins"
                        );

                        if let wasmparser::ComponentTypeRef::Instance(_) = imp.ty {
                            instance_imports.insert(imp.name.name);
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(instance_imports)
    }

    /// Define a compile-time builtin component, via its Wasm bytes.
    ///
    /// Compile-time builtins enable you to build safe, zero-copy, and (with
    /// [inlining][crate::Config::compiler_inlining])
    /// zero-function-call-overhead Wasm APIs for accessing host data, buffers,
    /// and objects.
    ///
    /// A compile-time builtin is a component that is
    ///
    /// * authored by the host (Wasmtime embedder),
    ///
    /// * whose implementation (though not necessarily its interface!) is
    ///   host-specific,
    ///
    /// * has access to unsafe intrinsics (and is therefore part of the host's
    ///   [trusted compute base]), and
    ///
    /// * is linked into guest Wasm programs at compile-time.
    ///
    /// Any imports satisfied by a compile-time builtin during compilation will
    /// not show up in the resulting component's
    /// [imports][crate::component::types::Component::imports], and they can no
    /// longer be customized by a [`Linker`][crate::component::Linker]
    /// definition at instantiation time.[^0]
    ///
    /// [^0]: If linking compile-time builtins into a component at compile-time
    /// reminds you of [component composition], that is not a coincidence:
    /// component composition is used under the covers as part of compile-time
    /// builtins' implementation.
    ///
    /// Comparing compile-time builtins with
    /// [`Linker`][crate::component::Linker]s is informative:
    ///
    /// * Both mechanisms define APIs to satisfy a Wasm program's imports.
    ///
    /// * A `Linker` satisfies those imports at instantiation-time, while
    ///   compile-time builtins do it during compilation.
    ///
    /// * APIs defined by a `Linker` are implemented in Rust, and hosts can
    ///   build safe, sandboxed Wasm APIs on top of raw, un-sandboxed primitives
    ///   via Rust's `unsafe`. APIs defined by compile-time builtins are
    ///   implemented as Wasm components, and hosts can build safe, sandboxed
    ///   Wasm APIs on top of raw, un-sandboxed primitives via [unsafe
    ///   intrinsics][CodeBuilder::expose_unsafe_intrinsics].
    ///
    /// * Imports satisfied via `Linker`-defined APIs are implemented with
    ///   [PLT/GOT]-style function table lookups and indirect calls in the
    ///   Wasm's compiled native code. On the other hand, Wasmtime implements
    ///   calls to imports satisfied via compile-time builtins with direct calls
    ///   in the Wasm's compiled native code. Wasmtime's compiler can also
    ///   [inline][crate::Config::compiler_inlining] these direct calls,
    ///   removing function call overheads and enabling further, cascading
    ///   compiler optimizations.
    ///
    /// If you are familiar with Wasm on the Web, you can think of compile-time
    /// builtins as the rough equivalent of [the `js-string-builtins` proposal]
    /// but for arbitrary host-defined APIs in a Wasmtime embedding environment
    /// rather than JS string APIs in a Web browser environment.
    ///
    /// [trusted compute base]: https://en.wikipedia.org/wiki/Trusted_computing_base
    /// [the `js-string-builtins` proposal]: https://github.com/WebAssembly/js-string-builtins/blob/main/proposals/js-string-builtins/Overview.md
    /// [component composition]: https://component-model.bytecodealliance.org/composing-and-distributing/composing.html
    /// [PLT/GOT]: https://reverseengineering.stackexchange.com/a/1993
    ///
    /// # Safety
    ///
    /// Compile-time builtins are part of your [trusted compute base] and should
    /// be authored by trusted, first-party developers with extreme care. You
    /// should never use compile-time builtins authored by untrusted,
    /// third-party developers.
    ///
    /// Compile-time builtins are given access to Wasmtime's [unsafe
    /// intrinsics][CodeBuilder::expose_unsafe_intrinsics], and the same safety
    /// invariants and portability concerns apply. However, when compile-time
    /// builtins are defined on a `CodeBuilder`, unsafe intrinsics are *only*
    /// exposed to the compile-time builtins, and they are *not* exposed to the
    /// main guest Wasm program. This means that — assuming your compile-time
    /// builtins only exposing safe APIs, encapsulating the intrinsics'
    /// unsafety, and modulo bugs in your implementation of those safe APIs —
    /// that the main guest Wasm program is not part of your trusted compute
    /// base.
    ///
    /// # Example
    ///
    /// See the example in [CodeBuilder::expose_unsafe_intrinsics].
    pub unsafe fn compile_time_builtins_binary(
        &mut self,
        name: impl Into<Cow<'a, str>>,
        wasm_bytes: impl Into<Cow<'a, [u8]>>,
    ) -> &mut Self {
        self.compile_time_builtins
            .insert(name.into(), wasm_bytes.into());
        self
    }

    /// Equivalent of [`CodeBuilder::compile_time_builtins_binary`] that also
    /// accepts the WebAssembly text format.
    ///
    /// This method will configure the WebAssembly binary to be compiled and
    /// used to satisfy the `name` instance import. The input `wasm_bytes` may
    /// either be the wasm text format or the binary format. If the `wat` crate
    /// feature is enabled, which is enabled by default, then the text format
    /// will automatically be converted to the binary format.
    ///
    /// # Errors
    ///
    /// This method will also return an error if `wasm_bytes` is the wasm text
    /// format and the text syntax is not valid.
    ///
    /// # Safety
    ///
    /// See [`CodeBuilder::compile_time_builtins_binary`].
    ///
    /// # Example
    ///
    /// See the example in [CodeBuilder::expose_unsafe_intrinsics], which uses
    /// compile-time builtins.
    #[allow(unused_variables, reason = "`wasm_path` only used with `wat` feature")]
    pub unsafe fn compile_time_builtins_binary_or_text(
        &mut self,
        name: impl Into<Cow<'a, str>>,
        wasm_bytes: impl Into<Cow<'a, [u8]>>,
        wasm_path: Option<&Path>,
    ) -> Result<&mut Self> {
        let wasm_bytes = wasm_bytes.into();

        #[cfg(feature = "wat")]
        if let Cow::Owned(wasm_bytes) = wat::parse_bytes(&wasm_bytes).map_err(|mut e| {
            if let Some(path) = wasm_path {
                e.set_path(path);
            }
            e
        })? {
            // SAFETY: Same as our unsafe contract.
            return Ok(unsafe { self.compile_time_builtins_binary(name, wasm_bytes) });
        }

        // SAFETY: Same as our unsafe contract.
        Ok(unsafe { self.compile_time_builtins_binary(name, wasm_bytes) })
    }

    /// Like [`CodeBuilder::compile_time_builtins_binary`], but reads the `file`
    /// specified for the bytes that will define the compile-time builtin.
    ///
    /// # Safety
    ///
    /// See [`CodeBuilder::compile_time_builtins_binary`].
    ///
    /// # Example
    ///
    /// See the example in [CodeBuilder::expose_unsafe_intrinsics], which uses
    /// compile-time builtins.
    pub unsafe fn compile_time_builtins_binary_file(
        &mut self,
        name: impl Into<Cow<'a, str>>,
        file: &Path,
    ) -> Result<&mut Self> {
        let wasm_bytes = std::fs::read(file)
            .with_context(|| format!("failed to read file: {}", file.display()))?;
        // SAFETY: Same as our unsafe contract.
        Ok(unsafe { self.compile_time_builtins_binary(name, wasm_bytes) })
    }

    /// Equivalent of [`CodeBuilder::compile_time_builtins_binary_file`] that
    /// also accepts the WebAssembly text format.
    ///
    /// This method is will read the file at the given path and interpret the
    /// contents to determine if it's the Wasm text format or binary format. The
    /// file extension is not consulted. The text format is automatically
    /// converted to the binary format if the crate feature `wat` is active.
    ///
    /// # Errors
    ///
    /// In addition to the errors returned by
    /// [`CodeBuilder::compile_time_builtins_binary_file`] this may also fail if
    /// the text format is read and the syntax is invalid.
    ///
    /// # Safety
    ///
    /// See [`CodeBuilder::compile_time_builtins_binary`].
    ///
    /// # Example
    ///
    /// See the example in [CodeBuilder::expose_unsafe_intrinsics], which uses
    /// compile-time builtins.
    pub unsafe fn compile_time_builtins_binary_or_text_file(
        &mut self,
        name: impl Into<Cow<'a, str>>,
        file: &Path,
    ) -> Result<&mut Self> {
        #[cfg(feature = "wat")]
        {
            let wasm = wat::parse_file(file)
                .with_context(|| format!("error parsing file: {}", file.display()))?;
            // SAFETY: Same as our unsafe contract.
            Ok(unsafe { self.compile_time_builtins_binary(name, wasm) })
        }

        #[cfg(not(feature = "wat"))]
        {
            // SAFETY: Same as our unsafe contract.
            unsafe { self.compile_time_builtins_binary_file(name, file) }
        }
    }
}
