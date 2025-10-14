//! Wizer: the WebAssembly pre-initializer!
//!
//! See the [`Wizer`] struct for details.

#![deny(missing_docs)]

mod info;
mod instrument;
mod parse;
mod rewrite;
mod snapshot;

/// Re-export wasmtime so users can align with our version. This is
/// especially useful when providing a custom Linker.
pub use wasmtime;

pub use crate::info::ModuleContext;
use anyhow::Context;
use std::collections::{HashMap, HashSet};
use wasmtime::{Extern, Module, Result, Store};

const DEFAULT_KEEP_INIT_FUNC: bool = false;

/// Wizer: the WebAssembly pre-initializer!
///
/// Don't wait for your Wasm module to initialize itself, pre-initialize it!
/// Wizer instantiates your WebAssembly module, executes its initialization
/// function, and then serializes the instance's initialized state out into a
/// new WebAssembly module. Now you can use this new, pre-initialized
/// WebAssembly module to hit the ground running, without making your users wait
/// for that first-time set up code to complete.
///
/// ## Caveats
///
/// * The initialization function may not call any imported functions. Doing so
///   will trigger a trap and `wizer` will exit.
///
/// * The Wasm module may not import globals, tables, or memories.
///
/// * Reference types are not supported yet. This is tricky because it would
///   allow the Wasm module to mutate tables, and we would need to be able to
///   snapshot the new table state, but funcrefs and externrefs don't have
///   identity and aren't comparable in the Wasm spec, which makes snapshotting
///   difficult.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "clap", derive(clap::Parser))]
pub struct Wizer {
    /// The Wasm export name of the function that should be executed to
    /// initialize the Wasm module.
    #[cfg_attr(
        feature = "clap",
        arg(short = 'f', long, default_value = "wizer.initialize")
    )]
    init_func: String,

    /// Any function renamings to perform.
    ///
    /// A renaming specification `dst=src` renames a function export `src` to
    /// `dst`, overwriting any previous `dst` export.
    ///
    /// Multiple renamings can be specified. It is an error to specify more than
    /// one source to rename to a destination name, or to specify more than one
    /// renaming destination for one source.
    ///
    /// This option can be used, for example, to replace a `_start` entry point
    /// in an initialized module with an alternate entry point.
    ///
    /// When module linking is enabled, these renames are only applied to the
    /// outermost module.
    #[cfg_attr(
        feature = "clap",
        arg(
            short = 'r',
            long = "rename-func",
            alias = "func-rename",
            value_name = "dst=src",
            value_parser = parse_rename,
        ),
    )]
    func_renames: Vec<(String, String)>,

    /// After initialization, should the Wasm module still export the
    /// initialization function?
    ///
    /// This is `false` by default, meaning that the initialization function is
    /// no longer exported from the Wasm module.
    #[cfg_attr(
        feature = "clap",
        arg(long, require_equals = true, value_name = "true|false")
    )]
    keep_init_func: Option<Option<bool>>,
}

#[cfg(feature = "clap")]
fn parse_rename(s: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        anyhow::bail!("must contain exactly one equals character ('=')");
    }
    Ok((parts[0].into(), parts[1].into()))
}

struct FuncRenames {
    /// For a given export name that we encounter in the original module, a map
    /// to a new name, if any, to emit in the output module.
    rename_src_to_dst: HashMap<String, String>,
    /// A set of export names that we ignore in the original module (because
    /// they are overwritten by renamings).
    rename_dsts: HashSet<String>,
}

impl FuncRenames {
    fn parse(renames: &[(String, String)]) -> anyhow::Result<FuncRenames> {
        let mut ret = FuncRenames {
            rename_src_to_dst: HashMap::new(),
            rename_dsts: HashSet::new(),
        };
        if renames.is_empty() {
            return Ok(ret);
        }

        for (dst, src) in renames {
            if ret.rename_dsts.contains(dst) {
                anyhow::bail!("Duplicated function rename dst {dst}");
            }
            if ret.rename_src_to_dst.contains_key(src) {
                anyhow::bail!("Duplicated function rename src {src}");
            }
            ret.rename_dsts.insert(dst.clone());
            ret.rename_src_to_dst.insert(src.clone(), dst.clone());
        }

        Ok(ret)
    }
}

impl Wizer {
    /// Construct a new `Wizer` builder.
    pub fn new() -> Self {
        Wizer {
            init_func: "wizer.initialize".into(),
            func_renames: vec![],
            keep_init_func: None,
        }
    }

    /// The export name of the initializer function.
    ///
    /// Defaults to `"wizer.initialize"`.
    pub fn init_func(&mut self, init_func: impl Into<String>) -> &mut Self {
        self.init_func = init_func.into();
        self
    }

    /// Returns the initialization function that will be run for wizer.
    pub fn get_init_func(&self) -> &str {
        &self.init_func
    }

    /// Add a function rename to perform.
    pub fn func_rename(&mut self, new_name: &str, old_name: &str) -> &mut Self {
        self.func_renames
            .push((new_name.to_string(), old_name.to_string()));
        self
    }

    /// After initialization, should the Wasm module still export the
    /// initialization function?
    ///
    /// This is `false` by default, meaning that the initialization function is
    /// no longer exported from the Wasm module.
    pub fn keep_init_func(&mut self, keep: bool) -> &mut Self {
        self.keep_init_func = Some(Some(keep));
        self
    }

    /// Initialize the given Wasm, snapshot it, and return the serialized
    /// snapshot as a new, pre-initialized Wasm module.
    pub fn run<T>(
        &self,
        store: &mut Store<T>,
        wasm: &[u8],
        instantiate: impl FnOnce(&mut Store<T>, &Module) -> Result<wasmtime::Instance>,
    ) -> anyhow::Result<Vec<u8>> {
        let (cx, instrumented_wasm) = self.instrument(wasm)?;

        let engine = store.engine();
        let module = wasmtime::Module::new(engine, &instrumented_wasm)
            .context("failed to compile the Wasm module")?;
        self.validate_init_func(&module)?;

        let instance = instantiate(store, &module)?;
        self.initialize(store, &instance)?;
        self.snapshot(cx, store, &instance)
    }

    /// First half of [`Self::run`] which instruments the provided `wasm` and
    /// produces a new wasm module which should be run by a runtime.
    ///
    /// After the returned wasm is executed the context returned here and the
    /// state of the instance should be passed to [`Self::snapshot`].
    pub fn instrument<'a>(&self, wasm: &'a [u8]) -> anyhow::Result<(ModuleContext<'a>, Vec<u8>)> {
        // Make sure we're given valid Wasm from the get go.
        self.wasm_validate(&wasm)?;

        let cx = parse::parse(wasm)?;

        // When wizening core modules directly some imports aren't supported,
        // so check for those here.
        for import in cx.imports() {
            match import.ty {
                wasmparser::TypeRef::Global(_) => {
                    anyhow::bail!("imported globals are not supported")
                }
                wasmparser::TypeRef::Table(_) => {
                    anyhow::bail!("imported tables are not supported")
                }
                wasmparser::TypeRef::Memory(_) => {
                    anyhow::bail!("imported memories are not supported")
                }
                wasmparser::TypeRef::Func(_) => {}
                wasmparser::TypeRef::Tag(_) => {}
            }
        }

        let instrumented_wasm = instrument::instrument(&cx);

        if cfg!(debug_assertions) {
            if let Err(error) = self.wasm_validate(&instrumented_wasm) {
                #[cfg(feature = "wasmprinter")]
                let wat = wasmprinter::print_bytes(&wasm)
                    .unwrap_or_else(|e| format!("Disassembling to WAT failed: {}", e));
                #[cfg(not(feature = "wasmprinter"))]
                let wat = "`wasmprinter` cargo feature is not enabled".to_string();
                panic!("instrumented Wasm is not valid: {error:?}\n\nWAT:\n{wat}");
            }
        }

        Ok((cx, instrumented_wasm))
    }

    /// Second half of [`Self::run`] which takes the [`ModuleContext`] returned
    /// by [`Self::instrument`] and the state of the `instance` after it has
    /// possibly executed its initialization function.
    ///
    /// This returns a new WebAssembly binary which has all state
    /// pre-initialized.
    pub fn snapshot<T>(
        &self,
        mut cx: ModuleContext<'_>,
        store: &mut Store<T>,
        instance: &wasmtime::Instance,
    ) -> anyhow::Result<Vec<u8>> {
        // Parse rename spec.
        let renames = FuncRenames::parse(&self.func_renames)?;

        let snapshot = snapshot::snapshot(&mut *store, &instance);
        let rewritten_wasm = self.rewrite(&mut cx, store, &snapshot, &renames);

        if cfg!(debug_assertions) {
            if let Err(error) = self.wasm_validate(&rewritten_wasm) {
                #[cfg(feature = "wasmprinter")]
                let wat = wasmprinter::print_bytes(&rewritten_wasm)
                    .unwrap_or_else(|e| format!("Disassembling to WAT failed: {}", e));
                #[cfg(not(feature = "wasmprinter"))]
                let wat = "`wasmprinter` cargo feature is not enabled".to_string();
                panic!("rewritten Wasm is not valid: {error:?}\n\nWAT:\n{wat}");
            }
        }

        Ok(rewritten_wasm)
    }

    fn wasm_validate(&self, wasm: &[u8]) -> anyhow::Result<()> {
        log::debug!("Validating input Wasm");

        wasmparser::Validator::new_with_features(wasmparser::WasmFeatures::all())
            .validate_all(wasm)
            .context("wasm validation failed")?;

        // Reject bulk memory stuff that manipulates state we don't
        // snapshot. See the comment inside `wasm_features`.
        let mut wasm = wasm;
        let mut parsers = vec![wasmparser::Parser::new(0)];
        while !parsers.is_empty() {
            let payload = match parsers.last_mut().unwrap().parse(wasm, true)? {
                wasmparser::Chunk::NeedMoreData(_) => unreachable!(),
                wasmparser::Chunk::Parsed { consumed, payload } => {
                    wasm = &wasm[consumed..];
                    payload
                }
            };
            match payload {
                wasmparser::Payload::CodeSectionEntry(code) => {
                    let mut ops = code.get_operators_reader()?;
                    while !ops.eof() {
                        match ops.read()? {
                            // Table mutations aren't allowed as wizer has no
                            // way to record a snapshot of a table at this time.
                            // The only table mutations allowed are those from
                            // active element segments which can be
                            // deterministically replayed, so disallow all other
                            // forms of mutating a table.
                            //
                            // Ideally Wizer could take a snapshot of a table
                            // post-instantiation and then ensure that after
                            // running initialization the table didn't get
                            // mutated, allowing these instructions, but that's
                            // also not possible at this time.
                            wasmparser::Operator::TableCopy { .. } => {
                                anyhow::bail!("unsupported `table.copy` instruction")
                            }
                            wasmparser::Operator::TableInit { .. } => {
                                anyhow::bail!("unsupported `table.init` instruction")
                            }
                            wasmparser::Operator::TableSet { .. } => {
                                anyhow::bail!("unsupported `table.set` instruction")
                            }
                            wasmparser::Operator::TableGrow { .. } => {
                                anyhow::bail!("unsupported `table.grow` instruction")
                            }
                            wasmparser::Operator::TableFill { .. } => {
                                anyhow::bail!("unsupported `table.fill` instruction")
                            }

                            // Wizer has no way of dynamically determining which
                            // element or data segments were dropped during
                            // execution so instead disallow these instructions
                            // entirely. Like above it'd be nice to allow them
                            // but just forbid their execution during the
                            // initialization function, but that can't be done
                            // easily at this time.
                            wasmparser::Operator::ElemDrop { .. } => {
                                anyhow::bail!("unsupported `elem.drop` instruction")
                            }
                            wasmparser::Operator::DataDrop { .. } => {
                                anyhow::bail!("unsupported `data.drop` instruction")
                            }

                            _ => continue,
                        }
                    }
                }
                wasmparser::Payload::End(_) => {
                    parsers.pop();
                }
                _ => continue,
            }
        }

        Ok(())
    }

    /// Check that the module exports an initialization function, and that the
    /// function has the correct type.
    fn validate_init_func(&self, module: &wasmtime::Module) -> anyhow::Result<()> {
        log::debug!("Validating the exported initialization function");
        match module.get_export(&self.init_func) {
            Some(wasmtime::ExternType::Func(func_ty)) => {
                if func_ty.params().len() != 0 || func_ty.results().len() != 0 {
                    anyhow::bail!(
                        "the Wasm module's `{}` function export does not have type `[] -> []`",
                        &self.init_func
                    );
                }
            }
            Some(_) => anyhow::bail!(
                "the Wasm module's `{}` export is not a function",
                &self.init_func
            ),
            None => anyhow::bail!(
                "the Wasm module does not have a `{}` export",
                &self.init_func
            ),
        }
        Ok(())
    }

    /// Instantiate the module and call its initialization function.
    fn initialize<T>(
        &self,
        store: &mut Store<T>,
        instance: &wasmtime::Instance,
    ) -> anyhow::Result<()> {
        log::debug!("Calling the initialization function");

        if let Some(export) = instance.get_export(&mut *store, "_initialize") {
            if let Extern::Func(func) = export {
                func.typed::<(), ()>(&store)
                    .and_then(|f| f.call(&mut *store, ()))
                    .context("calling the Reactor initialization function")?;

                if self.init_func == "_initialize" {
                    // Don't run `_initialize` twice if the it was explicitly
                    // requested as the init function.
                    return Ok(());
                }
            }
        }

        let init_func = instance
            .get_typed_func::<(), ()>(&mut *store, &self.init_func)
            .expect("checked by `validate_init_func`");
        init_func
            .call(&mut *store, ())
            .with_context(|| format!("the `{}` function trapped", self.init_func))?;

        Ok(())
    }

    fn get_keep_init_func(&self) -> bool {
        match self.keep_init_func {
            Some(keep) => keep.unwrap_or(true),
            None => DEFAULT_KEEP_INIT_FUNC,
        }
    }
}
