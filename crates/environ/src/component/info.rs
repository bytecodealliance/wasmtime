// General runtime type-information about a component.
//
// ## Optimizing instantiation
//
// One major consideration for the structure of the types in this module is to
// make instantiation as fast as possible. To facilitate this the representation
// here avoids the need to create a `PrimaryMap` during instantiation of a
// component for each index space like the func, global, table, etc, index
// spaces. Instead a component is simply defined by a list of instantiation
// instructions, and arguments to the instantiation of each instance are a list
// of "pointers" into previously created instances. This means that we only need
// to build up one list of instances during instantiation.
//
// Additionally we also try to avoid string lookups wherever possible. In the
// component model instantiation and aliasing theoretically deals with lots of
// string lookups here and there. This is slower than indexing lookup, though,
// and not actually necessary when the structure of a module is statically
// known. This means that `ExportItem` below has two variants where we try to
// use the indexing variant as much as possible, which can be done for
// everything except imported core wasm modules.

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
    ///
    /// NB: at this time recursive components are not supported, and that may
    /// change this somewhat significantly.
    pub initializers: Vec<Initializer>,

    /// The number of runtime instances (maximum `RuntimeInstanceIndex`) created
    /// when instantiating this component.
    pub num_runtime_instances: u32,

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

    /// The number of lowered host functions (maximum `LoweredIndex`) needed to
    /// instantiate this component.
    pub num_lowerings: u32,
}

/// Initializer instructions to get processed when instantiating a component
///
/// The variants of this enum are processed during the instantiation phase of
/// a component in-order from front-to-back. These are otherwise emitted as a
/// component is parsed and read and translated.
///
/// NB: at this time recursive components are not supported, and that may
/// change this somewhat significantly.
///
//
// FIXME(#2639) if processing this list is ever a bottleneck we could
// theoretically use cranelift to compile an initialization function which
// performs all of these duties for us and skips the overhead of interpreting
// all of these instructions.
#[derive(Debug, Serialize, Deserialize)]
pub enum Initializer {
    /// A core was module is being instantiated.
    ///
    /// This will result in a new core wasm instance being created, which may
    /// involve running the `start` function of the instance as well if it's
    /// specified. This largely delegates to the same standard instantiation
    /// process as the rest of the core wasm machinery already uses.
    InstantiateModule {
        /// The instance of the index that's being created.
        ///
        /// This is guaranteed to be the `n`th `InstantiateModule` instruction
        /// if the index is `n`.
        instance: RuntimeInstanceIndex,

        /// The module that's being instantiated, either an "upvar" or an
        /// imported module.
        module: ModuleToInstantiate,

        /// The arguments to instantiation and where they're loaded from.
        ///
        /// Note that this is a flat list. For "upvars" this list is sorted by
        /// the actual concrete imports needed by the upvar so the items can be
        /// passed directly to instantiation. For imports this list is sorted
        /// by the order of the import names on the type of the module
        /// declaration in this component.
        ///
        /// Each argument is a `CoreDef` which represents that it's either, at
        /// this time, a lowered imported function or a core wasm item from
        /// another previously instantiated instance.
        args: Box<[CoreDef]>,
    },

    /// A host function is being lowered, creating a core wasm function.
    ///
    /// This initializer entry is intended to be used to fill out the
    /// `VMComponentContext` and information about this lowering such as the
    /// cranelift-compiled trampoline function pointer, the host function
    /// pointer the trampline calls, and the canonical ABI options.
    LowerImport(LowerImport),

    /// A core wasm linear memory is going to be saved into the
    /// `VMComponentContext`.
    ///
    /// This instruction indicates that the `index`th core wasm linear memory
    /// needs to be extracted from the `export` specified, a pointer to a
    /// previously created module instance, and stored into the
    /// `VMComponentContext` at the `index` specified. This lowering is then
    /// used in the future by pointers from `CanonicalOptions`.
    ExtractMemory {
        /// The index of the memory we're storing.
        ///
        /// This is guaranteed to be the `n`th `ExtractMemory` instruction
        /// if the index is `n`.
        index: RuntimeMemoryIndex,
        /// The source of the memory that is stored.
        export: CoreExport<MemoryIndex>,
    },

    /// Same as `ExtractMemory`, except it's extracting a function pointer to be
    /// used as a `realloc` function.
    ExtractRealloc {
        /// The index of the realloc function we're storing.
        ///
        /// This is guaranteed to be the `n`th `ExtractRealloc` instruction
        /// if the index is `n`.
        index: RuntimeReallocIndex,
        /// The source of the function pointer that is stored.
        def: CoreDef,
    },
}

/// Indicator used to refer to what module is being instantiated when
/// `Initializer::InstantiateModule` is used.
#[derive(Debug, Serialize, Deserialize)]
pub enum ModuleToInstantiate {
    /// An "upvar", or a module defined within a component, is being used.
    ///
    /// The index here is correlated with the `Translation::upvars` map that's
    /// created during translation of a component.
    Upvar(ModuleUpvarIndex),

    /// An imported core wasm module is being instantiated.
    ///
    /// It's guaranteed that this `RuntimeImportIndex` points to a module.
    Import(RuntimeImportIndex),
}

/// Description of a lowered import used in conjunction with
/// `Initializer::LowerImport`.
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
    /// `Initializer::LowerImport` instruction.
    Lowered(LoweredIndex),
}

impl From<CoreExport<EntityIndex>> for CoreDef {
    fn from(export: CoreExport<EntityIndex>) -> CoreDef {
        CoreDef::Export(export)
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
        ty: FuncTypeIndex,
        /// Which core WebAssembly export is being lifted.
        func: CoreExport<FuncIndex>,
        /// Any options, if present, associated with this lifting.
        options: CanonicalOptions,
    },
}

/// Canonical ABI options associated with a lifted or lowered function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalOptions {
    /// The encoding used for strings.
    pub string_encoding: StringEncoding,

    /// The memory used by these options, if specified.
    pub memory: Option<RuntimeMemoryIndex>,

    /// The realloc function used by these options, if specified.
    pub realloc: Option<RuntimeReallocIndex>,
    // TODO: need to represent post-return here as well
}

impl Default for CanonicalOptions {
    fn default() -> CanonicalOptions {
        CanonicalOptions {
            string_encoding: StringEncoding::Utf8,
            memory: None,
            realloc: None,
        }
    }
}

/// Possible encodings of strings within the component model.
//
// Note that the `repr(u8)` is load-bearing here since this is used in an
// `extern "C" fn()` function argument which is called from cranelift-compiled
// code so we must know the representation of this.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
#[repr(u8)]
pub enum StringEncoding {
    Utf8,
    Utf16,
    CompactUtf16,
}
