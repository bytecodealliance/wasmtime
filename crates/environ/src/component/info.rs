// General runtime type-information about a component.
//
// Compared to the `Module` structure for core wasm this type is pretty
// significantly different. The core wasm `Module` corresponds roughly 1-to-1
// with the structure of the wasm module itself, but instead a `Component` is
// more of a "compiled" representation where the original structure is thrown
// away in favor of a more optimized representation. The considerations for this
// are:
//
// * This representation of a `Component` avoids the need to create a
//   `PrimaryMap` of some form for each of the index spaces within a component.
//   This is less so an issue about allocations and moreso that this information
//   generally just isn't needed any time after instantiation. Avoiding creating
//   these altogether helps components be lighter weight at runtime and
//   additionally accelerates instantiation.
//
// * Components can have arbitrary nesting and internally do instantiations via
//   string-based matching. At instantiation-time, though, we want to do as few
//   string-lookups in hash maps as much as we can since they're significantly
//   slower than index-based lookups. Furthermore while the imports of a
//   component are not statically known the rest of the structure of the
//   component is statically known which enables the ability to track precisely
//   what matches up where and do all the string lookups at compile time instead
//   of instantiation time.
//
// * Finally by performing this sort of dataflow analysis we are capable of
//   identifying what adapters need trampolines for compilation or fusion. For
//   example this tracks when host functions are lowered which enables us to
//   enumerate what trampolines are required to enter into a component.
//   Additionally (eventually) this will track all of the "fused" adapter
//   functions where a function from one component instance is lifted and then
//   lowered into another component instance. Altogether this enables Wasmtime's
//   AOT-compilation where the artifact from compilation is suitable for use in
//   running the component without the support of a compiler at runtime.
//
// Note, however, that the current design of `Component` has fundamental
// limitations which it was not designed for. For example there is no feasible
// way to implement either importing or exporting a component itself from the
// root component. Currently we rely on the ability to have static knowledge of
// what's coming from the host which at this point can only be either functions
// or core wasm modules. Additionally one flat list of initializers for a
// component are produced instead of initializers-per-component which would
// otherwise be required to export a component from a component.
//
// For now this tradeoff is made as it aligns well with the intended use case
// for components in an embedding. This may need to be revisited though if the
// requirements of embeddings change over time.

use crate::component::*;
use crate::{EntityIndex, PrimaryMap, SignatureIndex};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Run-time-type-information about a `Component`, its structure, and how to
/// instantiate it.
///
/// This type is intended to mirror the `Module` type in this crate which
/// provides all the runtime information about the structure of a module and
/// how it works.
///
/// NB: Lots of the component model is not yet implemented in the runtime so
/// this is going to undergo a lot of churn.
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Component {
    /// A list of typed values that this component imports.
    ///
    /// Note that each name is given an `ImportIndex` here for the next map to
    /// refer back to.
    pub import_types: PrimaryMap<ImportIndex, (String, TypeDef)>,

    /// A list of "flattened" imports that are used by this instance.
    ///
    /// This import map represents extracting imports, as necessary, from the
    /// general imported types by this component. The flattening here refers to
    /// extracting items from instances. Currently the flat imports are either a
    /// host function or a core wasm module.
    ///
    /// For example if `ImportIndex(0)` pointed to an instance then this import
    /// map represent extracting names from that map, for example extracting an
    /// exported module or an exported function.
    ///
    /// Each import item is keyed by a `RuntimeImportIndex` which is referred to
    /// by types below whenever something refers to an import. The value for
    /// each `RuntimeImportIndex` in this map is the `ImportIndex` for where
    /// this items comes from (which can be associated with a name above in the
    /// `import_types` array) as well as the list of export names if
    /// `ImportIndex` refers to an instance. The export names array represents
    /// recursively fetching names within an instance.
    //
    // TODO: this is probably a lot of `String` storage and may be something
    // that needs optimization in the future. For example instead of lots of
    // different `String` allocations this could instead be a pointer/length
    // into one large string allocation for the entire component. Alternatively
    // strings could otherwise be globally intern'd via some other mechanism to
    // avoid `Linker`-specific intern-ing plus intern-ing here. Unsure what the
    // best route is or whether such an optimization is even necessary here.
    pub imports: PrimaryMap<RuntimeImportIndex, (ImportIndex, Vec<String>)>,

    /// A list of this component's exports, indexed by either position or name.
    pub exports: IndexMap<String, Export>,

    /// Initializers that must be processed when instantiating this component.
    ///
    /// This list of initializers does not correspond directly to the component
    /// itself. The general goal with this is that the recursive nature of
    /// components is "flattened" with an array like this which is a linear
    /// sequence of instructions of how to instantiate a component. This will
    /// have instantiations, for example, in addition to entries which
    /// initialize `VMComponentContext` fields with previously instantiated
    /// instances.
    pub initializers: Vec<GlobalInitializer>,

    /// The number of runtime instances (maximum `RuntimeInstanceIndex`) created
    /// when instantiating this component.
    pub num_runtime_instances: u32,

    /// Same as `num_runtime_instances`, but for `RuntimeComponentInstanceIndex`
    /// instead.
    pub num_runtime_component_instances: u32,

    /// The number of runtime memories (maximum `RuntimeMemoryIndex`) needed to
    /// instantiate this component.
    ///
    /// Note that this many memories will be stored in the `VMComponentContext`
    /// and each memory is intended to be unique (e.g. the same memory isn't
    /// stored in two different locations).
    pub num_runtime_memories: u32,

    /// The number of runtime reallocs (maximum `RuntimeReallocIndex`) needed to
    /// instantiate this component.
    ///
    /// Note that this many function pointers will be stored in the
    /// `VMComponentContext`.
    pub num_runtime_reallocs: u32,

    /// Same as `num_runtime_reallocs`, but for post-return functions.
    pub num_runtime_post_returns: u32,

    /// The number of lowered host functions (maximum `LoweredIndex`) needed to
    /// instantiate this component.
    pub num_lowerings: u32,

    /// The number of modules that are required to be saved within an instance
    /// at runtime, or effectively the number of exported modules.
    pub num_runtime_modules: u32,

    /// The number of functions which "always trap" used to implement
    /// `canon.lower` of `canon.lift`'d functions within the same component.
    pub num_always_trap: u32,

    /// The number of host transcoder functions needed for strings in adapter
    /// modules.
    pub num_transcoders: u32,
}

/// GlobalInitializer instructions to get processed when instantiating a component
///
/// The variants of this enum are processed during the instantiation phase of
/// a component in-order from front-to-back. These are otherwise emitted as a
/// component is parsed and read and translated.
//
// FIXME(#2639) if processing this list is ever a bottleneck we could
// theoretically use cranelift to compile an initialization function which
// performs all of these duties for us and skips the overhead of interpreting
// all of these instructions.
#[derive(Debug, Serialize, Deserialize)]
pub enum GlobalInitializer {
    /// A core wasm module is being instantiated.
    ///
    /// This will result in a new core wasm instance being created, which may
    /// involve running the `start` function of the instance as well if it's
    /// specified. This largely delegates to the same standard instantiation
    /// process as the rest of the core wasm machinery already uses.
    InstantiateModule(InstantiateModule),

    /// A host function is being lowered, creating a core wasm function.
    ///
    /// This initializer entry is intended to be used to fill out the
    /// `VMComponentContext` and information about this lowering such as the
    /// cranelift-compiled trampoline function pointer, the host function
    /// pointer the trampoline calls, and the canonical ABI options.
    LowerImport(LowerImport),

    /// A core wasm function was "generated" via `canon lower` of a function
    /// that was `canon lift`'d in the same component, meaning that the function
    /// always traps. This is recorded within the `VMComponentContext` as a new
    /// `VMCallerCheckedAnyfunc` that's available for use.
    AlwaysTrap(AlwaysTrap),

    /// A core wasm linear memory is going to be saved into the
    /// `VMComponentContext`.
    ///
    /// This instruction indicates that the `index`th core wasm linear memory
    /// needs to be extracted from the `export` specified, a pointer to a
    /// previously created module instance, and stored into the
    /// `VMComponentContext` at the `index` specified. This lowering is then
    /// used in the future by pointers from `CanonicalOptions`.
    ExtractMemory(ExtractMemory),

    /// Same as `ExtractMemory`, except it's extracting a function pointer to be
    /// used as a `realloc` function.
    ExtractRealloc(ExtractRealloc),

    /// Same as `ExtractMemory`, except it's extracting a function pointer to be
    /// used as a `post-return` function.
    ExtractPostReturn(ExtractPostReturn),

    /// The `module` specified is saved into the runtime state at the next
    /// `RuntimeModuleIndex`, referred to later by `Export` definitions.
    SaveStaticModule(StaticModuleIndex),

    /// Same as `SaveModuleUpvar`, but for imports.
    SaveModuleImport(RuntimeImportIndex),

    /// Similar to `ExtractMemory` and friends and indicates that a
    /// `VMCallerCheckedAnyfunc` needs to be initialized for a transcoder
    /// function and this will later be used to instantiate an adapter module.
    Transcoder(Transcoder),
}

/// Metadata for extraction of a memory of what's being extracted and where it's
/// going.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractMemory {
    /// The index of the memory being defined.
    pub index: RuntimeMemoryIndex,
    /// Where this memory is being extracted from.
    pub export: CoreExport<MemoryIndex>,
}

/// Same as `ExtractMemory` but for the `realloc` canonical option.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractRealloc {
    /// The index of the realloc being defined.
    pub index: RuntimeReallocIndex,
    /// Where this realloc is being extracted from.
    pub def: CoreDef,
}

/// Same as `ExtractMemory` but for the `post-return` canonical option.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractPostReturn {
    /// The index of the post-return being defined.
    pub index: RuntimePostReturnIndex,
    /// Where this post-return is being extracted from.
    pub def: CoreDef,
}

/// Different methods of instantiating a core wasm module.
#[derive(Debug, Serialize, Deserialize)]
pub enum InstantiateModule {
    /// A module defined within this component is being instantiated.
    ///
    /// Note that this is distinct from the case of imported modules because the
    /// order of imports required is statically known and can be pre-calculated
    /// to avoid string lookups related to names at runtime, represented by the
    /// flat list of arguments here.
    Static(StaticModuleIndex, Box<[CoreDef]>),

    /// An imported module is being instantiated.
    ///
    /// This is similar to `Upvar` but notably the imports are provided as a
    /// two-level named map since import resolution order needs to happen at
    /// runtime.
    Import(
        RuntimeImportIndex,
        IndexMap<String, IndexMap<String, CoreDef>>,
    ),
}

/// Description of a lowered import used in conjunction with
/// `GlobalInitializer::LowerImport`.
#[derive(Debug, Serialize, Deserialize)]
pub struct LowerImport {
    /// The index of the lowered function that's being created.
    ///
    /// This is guaranteed to be the `n`th `LowerImport` instruction
    /// if the index is `n`.
    pub index: LoweredIndex,

    /// The index of the imported host function that is being lowered.
    ///
    /// It's guaranteed that this `RuntimeImportIndex` points to a function.
    pub import: RuntimeImportIndex,

    /// The core wasm signature of the function that's being created.
    pub canonical_abi: SignatureIndex,

    /// The canonical ABI options used when lowering this function specified in
    /// the original component.
    pub options: CanonicalOptions,
}

/// Description of what to initialize when a `GlobalInitializer::AlwaysTrap` is
/// encountered.
#[derive(Debug, Serialize, Deserialize)]
pub struct AlwaysTrap {
    /// The index of the function that is being initialized in the
    /// `VMComponentContext`.
    pub index: RuntimeAlwaysTrapIndex,
    /// The core wasm signature of the function that's inserted.
    pub canonical_abi: SignatureIndex,
}

/// Definition of a core wasm item and where it can come from within a
/// component.
///
/// Note that this is sort of a result of data-flow-like analysis on a component
/// during compile time of the component itself. References to core wasm items
/// are "compiled" to either referring to a previous instance or to some sort of
/// lowered host import.
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum CoreDef {
    /// This item refers to an export of a previously instantiated core wasm
    /// instance.
    Export(CoreExport<EntityIndex>),
    /// This item is a core wasm function with the index specified here. Note
    /// that this `LoweredIndex` corresponds to the nth
    /// `GlobalInitializer::LowerImport` instruction.
    Lowered(LoweredIndex),
    /// This is used to represent a degenerate case of where a `canon lift`'d
    /// function is immediately `canon lower`'d in the same instance. Such a
    /// function always traps at runtime.
    AlwaysTrap(RuntimeAlwaysTrapIndex),
    /// This is a reference to a wasm global which represents the
    /// runtime-managed flags for a wasm instance.
    InstanceFlags(RuntimeComponentInstanceIndex),
    /// This refers to a cranelift-generated trampoline which calls to a
    /// host-defined transcoding function.
    Transcoder(RuntimeTranscoderIndex),
}

impl<T> From<CoreExport<T>> for CoreDef
where
    EntityIndex: From<T>,
{
    fn from(export: CoreExport<T>) -> CoreDef {
        CoreDef::Export(export.map_index(|i| i.into()))
    }
}

/// Identifier of an exported item from a core WebAssembly module instance.
///
/// Note that the `T` here is the index type for exports which can be
/// identified by index. The `T` is monomorphized with types like
/// [`EntityIndex`] or [`FuncIndex`].
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct CoreExport<T> {
    /// The instance that this item is located within.
    ///
    /// Note that this is intended to index the `instances` map within a
    /// component. It's validated ahead of time that all instance pointers
    /// refer only to previously-created instances.
    pub instance: RuntimeInstanceIndex,

    /// The item that this export is referencing, either by name or by index.
    pub item: ExportItem<T>,
}

impl<T> CoreExport<T> {
    /// Maps the index type `T` to another type `U` if this export item indeed
    /// refers to an index `T`.
    pub fn map_index<U>(self, f: impl FnOnce(T) -> U) -> CoreExport<U> {
        CoreExport {
            instance: self.instance,
            item: match self.item {
                ExportItem::Index(i) => ExportItem::Index(f(i)),
                ExportItem::Name(s) => ExportItem::Name(s),
            },
        }
    }
}

/// An index at which to find an item within a runtime instance.
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum ExportItem<T> {
    /// An exact index that the target can be found at.
    ///
    /// This is used where possible to avoid name lookups at runtime during the
    /// instantiation process. This can only be used on instances where the
    /// module was statically known at compile time, however.
    Index(T),

    /// An item which is identified by a name, so at runtime we need to
    /// perform a name lookup to determine the index that the item is located
    /// at.
    ///
    /// This is used for instantiations of imported modules, for example, since
    /// the precise shape of the module is not known.
    Name(String),
}

/// Possible exports from a component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Export {
    /// A lifted function being exported which is an adaptation of a core wasm
    /// function.
    LiftedFunction {
        /// The component function type of the function being created.
        ty: TypeFuncIndex,
        /// Which core WebAssembly export is being lifted.
        func: CoreDef,
        /// Any options, if present, associated with this lifting.
        options: CanonicalOptions,
    },
    /// A module defined within this component is exported.
    ///
    /// The module index here indexes a module recorded with
    /// `GlobalInitializer::SaveModule` above.
    Module(RuntimeModuleIndex),
    /// A nested instance is being exported which has recursively defined
    /// `Export` items.
    Instance(IndexMap<String, Export>),
    /// An exported type from a component or instance, currently only
    /// informational.
    Type(TypeDef),
}

/// Canonical ABI options associated with a lifted or lowered function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalOptions {
    /// The component instance that this bundle was associated with.
    pub instance: RuntimeComponentInstanceIndex,

    /// The encoding used for strings.
    pub string_encoding: StringEncoding,

    /// The memory used by these options, if specified.
    pub memory: Option<RuntimeMemoryIndex>,

    /// The realloc function used by these options, if specified.
    pub realloc: Option<RuntimeReallocIndex>,

    /// The post-return function used by these options, if specified.
    pub post_return: Option<RuntimePostReturnIndex>,
}

/// Possible encodings of strings within the component model.
//
// Note that the `repr(u8)` is load-bearing here since this is used in an
// `extern "C" fn()` function argument which is called from cranelift-compiled
// code so we must know the representation of this.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
#[repr(u8)]
pub enum StringEncoding {
    Utf8,
    Utf16,
    CompactUtf16,
}

/// Information about a string transcoding function required by an adapter
/// module.
///
/// A transcoder is used when strings are passed between adapter modules,
/// optionally changing string encodings at the same time. The transcoder is
/// implemented in a few different layers:
///
/// * Each generated adapter module has some glue around invoking the transcoder
///   represented by this item. This involves bounds-checks and handling
///   `realloc` for example.
/// * Each transcoder gets a cranelift-generated trampoline which has the
///   appropriate signature for the adapter module in question. Existence of
///   this initializer indicates that this should be compiled by Cranelift.
/// * The cranelift-generated trampoline will invoke a "transcoder libcall"
///   which is implemented natively in Rust that has a signature independent of
///   memory64 configuration options for example.
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct Transcoder {
    /// The index of the transcoder being defined and initialized.
    ///
    /// This indicates which `VMCallerCheckedAnyfunc` slot is written to in a
    /// `VMComponentContext`.
    pub index: RuntimeTranscoderIndex,
    /// The transcoding operation being performed.
    pub op: Transcode,
    /// The linear memory that the string is being read from.
    pub from: RuntimeMemoryIndex,
    /// Whether or not the source linear memory is 64-bit or not.
    pub from64: bool,
    /// The linear memory that the string is being written to.
    pub to: RuntimeMemoryIndex,
    /// Whether or not the destination linear memory is 64-bit or not.
    pub to64: bool,
    /// The wasm signature of the cranelift-generated trampoline.
    pub signature: SignatureIndex,
}

pub use crate::fact::{FixedEncoding, Transcode};
