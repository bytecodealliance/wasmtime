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
//   This is less so an issue about allocations and more so that this information
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
use crate::prelude::*;
use crate::{EntityIndex, ModuleInternedTypeIndex, PrimaryMap, WasmValType};
use serde_derive::{Deserialize, Serialize};

/// Metadata as a result of compiling a component.
pub struct ComponentTranslation {
    /// Serializable information that will be emitted into the final artifact.
    pub component: Component,

    /// Metadata about required trampolines and what they're supposed to do.
    pub trampolines: PrimaryMap<TrampolineIndex, Trampoline>,
}

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

    /// This component's own root exports from the component itself.
    pub exports: NameMap<String, ExportIndex>,

    /// All exports of this component and exported instances of this component.
    ///
    /// This is indexed by `ExportIndex` for fast lookup and `Export::Instance`
    /// will refer back into this list.
    pub export_items: PrimaryMap<ExportIndex, Export>,

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

    /// WebAssembly type signature of all trampolines.
    pub trampolines: PrimaryMap<TrampolineIndex, ModuleInternedTypeIndex>,

    /// The number of lowered host functions (maximum `LoweredIndex`) needed to
    /// instantiate this component.
    pub num_lowerings: u32,

    /// Maximal number of tables that required at runtime for resource-related
    /// information in this component.
    pub num_resource_tables: usize,

    /// Total number of resources both imported and defined within this
    /// component.
    pub num_resources: u32,

    /// Metadata about imported resources and where they are within the runtime
    /// imports array.
    ///
    /// This map is only as large as the number of imported resources.
    pub imported_resources: PrimaryMap<ResourceIndex, RuntimeImportIndex>,

    /// Metadata about which component instances defined each resource within
    /// this component.
    ///
    /// This is used to determine which set of instance flags are inspected when
    /// testing reentrance.
    pub defined_resource_instances: PrimaryMap<DefinedResourceIndex, RuntimeComponentInstanceIndex>,
}

impl Component {
    /// Attempts to convert a resource index into a defined index.
    ///
    /// Returns `None` if `idx` is for an imported resource in this component or
    /// `Some` if it's a locally defined resource.
    pub fn defined_resource_index(&self, idx: ResourceIndex) -> Option<DefinedResourceIndex> {
        let idx = idx
            .as_u32()
            .checked_sub(self.imported_resources.len() as u32)?;
        Some(DefinedResourceIndex::from_u32(idx))
    }

    /// Converts a defined resource index to a component-local resource index
    /// which includes all imports.
    pub fn resource_index(&self, idx: DefinedResourceIndex) -> ResourceIndex {
        ResourceIndex::from_u32(self.imported_resources.len() as u32 + idx.as_u32())
    }
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
    LowerImport {
        /// The index of the lowered function that's being created.
        ///
        /// This is guaranteed to be the `n`th `LowerImport` instruction
        /// if the index is `n`.
        index: LoweredIndex,

        /// The index of the imported host function that is being lowered.
        ///
        /// It's guaranteed that this `RuntimeImportIndex` points to a function.
        import: RuntimeImportIndex,
    },

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

    /// Declares a new defined resource within this component.
    ///
    /// Contains information about the destructor, for example.
    Resource(Resource),
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
    /// This is a reference to a wasm global which represents the
    /// runtime-managed flags for a wasm instance.
    InstanceFlags(RuntimeComponentInstanceIndex),
    /// This is a reference to a Cranelift-generated trampoline which is
    /// described in the `trampolines` array.
    Trampoline(TrampolineIndex),
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
    ModuleStatic {
        /// The type of this module
        ty: TypeModuleIndex,
        /// Which module this is referring to.
        index: StaticModuleIndex,
    },
    /// A module imported into this component is exported.
    ModuleImport {
        /// Module type index
        ty: TypeModuleIndex,
        /// Module runtime import index
        import: RuntimeImportIndex,
    },
    /// A nested instance is being exported which has recursively defined
    /// `Export` items.
    Instance {
        /// Instance type index, if such is assigned
        ty: TypeComponentInstanceIndex,
        /// Instance export map
        exports: NameMap<String, ExportIndex>,
    },
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[allow(missing_docs, reason = "self-describing variants")]
pub enum StringEncoding {
    Utf8,
    Utf16,
    CompactUtf16,
}

impl StringEncoding {
    /// Decodes the `u8` provided back into a `StringEncoding`, if it's valid.
    pub fn from_u8(val: u8) -> Option<StringEncoding> {
        if val == StringEncoding::Utf8 as u8 {
            return Some(StringEncoding::Utf8);
        }
        if val == StringEncoding::Utf16 as u8 {
            return Some(StringEncoding::Utf16);
        }
        if val == StringEncoding::CompactUtf16 as u8 {
            return Some(StringEncoding::CompactUtf16);
        }
        None
    }
}

/// Possible transcoding operations that must be provided by the host.
///
/// Note that each transcoding operation may have a unique signature depending
/// on the precise operation.
#[allow(missing_docs, reason = "self-describing variants")]
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum Transcode {
    Copy(FixedEncoding),
    Latin1ToUtf16,
    Latin1ToUtf8,
    Utf16ToCompactProbablyUtf16,
    Utf16ToCompactUtf16,
    Utf16ToLatin1,
    Utf16ToUtf8,
    Utf8ToCompactUtf16,
    Utf8ToLatin1,
    Utf8ToUtf16,
}

impl Transcode {
    /// Get this transcoding's symbol fragment.
    pub fn symbol_fragment(&self) -> &'static str {
        match self {
            Transcode::Copy(x) => match x {
                FixedEncoding::Utf8 => "copy_utf8",
                FixedEncoding::Utf16 => "copy_utf16",
                FixedEncoding::Latin1 => "copy_latin1",
            },
            Transcode::Latin1ToUtf16 => "latin1_to_utf16",
            Transcode::Latin1ToUtf8 => "latin1_to_utf8",
            Transcode::Utf16ToCompactProbablyUtf16 => "utf16_to_compact_probably_utf16",
            Transcode::Utf16ToCompactUtf16 => "utf16_to_compact_utf16",
            Transcode::Utf16ToLatin1 => "utf16_to_latin1",
            Transcode::Utf16ToUtf8 => "utf16_to_utf8",
            Transcode::Utf8ToCompactUtf16 => "utf8_to_compact_utf16",
            Transcode::Utf8ToLatin1 => "utf8_to_latin1",
            Transcode::Utf8ToUtf16 => "utf8_to_utf16",
        }
    }

    /// Returns a human-readable description for this transcoding operation.
    pub fn desc(&self) -> &'static str {
        match self {
            Transcode::Copy(FixedEncoding::Utf8) => "utf8-to-utf8",
            Transcode::Copy(FixedEncoding::Utf16) => "utf16-to-utf16",
            Transcode::Copy(FixedEncoding::Latin1) => "latin1-to-latin1",
            Transcode::Latin1ToUtf16 => "latin1-to-utf16",
            Transcode::Latin1ToUtf8 => "latin1-to-utf8",
            Transcode::Utf16ToCompactProbablyUtf16 => "utf16-to-compact-probably-utf16",
            Transcode::Utf16ToCompactUtf16 => "utf16-to-compact-utf16",
            Transcode::Utf16ToLatin1 => "utf16-to-latin1",
            Transcode::Utf16ToUtf8 => "utf16-to-utf8",
            Transcode::Utf8ToCompactUtf16 => "utf8-to-compact-utf16",
            Transcode::Utf8ToLatin1 => "utf8-to-latin1",
            Transcode::Utf8ToUtf16 => "utf8-to-utf16",
        }
    }
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[allow(missing_docs, reason = "self-describing variants")]
pub enum FixedEncoding {
    Utf8,
    Utf16,
    Latin1,
}

impl FixedEncoding {
    /// Returns the byte width of unit loads/stores for this encoding, for
    /// example the unit length is multiplied by this return value to get the
    /// byte width of a string.
    pub fn width(&self) -> u8 {
        match self {
            FixedEncoding::Utf8 => 1,
            FixedEncoding::Utf16 => 2,
            FixedEncoding::Latin1 => 1,
        }
    }
}

/// Description of a new resource declared in a `GlobalInitializer::Resource`
/// variant.
///
/// This will have the effect of initializing runtime state for this resource,
/// namely the destructor is fetched and stored.
#[derive(Debug, Serialize, Deserialize)]
pub struct Resource {
    /// The local index of the resource being defined.
    pub index: DefinedResourceIndex,
    /// Core wasm representation of this resource.
    pub rep: WasmValType,
    /// Optionally-specified destructor and where it comes from.
    pub dtor: Option<CoreDef>,
    /// Which component instance this resource logically belongs to.
    pub instance: RuntimeComponentInstanceIndex,
}

/// A list of all possible trampolines that may be required to compile a
/// component completely.
///
/// These trampolines are used often as core wasm definitions and require
/// Cranelift support to generate these functions. Each trampoline serves a
/// different purpose for implementing bits and pieces of the component model.
///
/// All trampolines have a core wasm function signature associated with them
/// which is stored in the `Component::trampolines` array.
///
/// Note that this type does not implement `Serialize` or `Deserialize` and
/// that's intentional as this isn't stored in the final compilation artifact.
pub enum Trampoline {
    /// Description of a lowered import used in conjunction with
    /// `GlobalInitializer::LowerImport`.
    LowerImport {
        /// The runtime lowering state that this trampoline will access.
        index: LoweredIndex,

        /// The type of the function that is being lowered, as perceived by the
        /// component doing the lowering.
        lower_ty: TypeFuncIndex,

        /// The canonical ABI options used when lowering this function specified
        /// in the original component.
        options: CanonicalOptions,
    },

    /// Information about a string transcoding function required by an adapter
    /// module.
    ///
    /// A transcoder is used when strings are passed between adapter modules,
    /// optionally changing string encodings at the same time. The transcoder is
    /// implemented in a few different layers:
    ///
    /// * Each generated adapter module has some glue around invoking the
    ///   transcoder represented by this item. This involves bounds-checks and
    ///   handling `realloc` for example.
    /// * Each transcoder gets a cranelift-generated trampoline which has the
    ///   appropriate signature for the adapter module in question. Existence of
    ///   this initializer indicates that this should be compiled by Cranelift.
    /// * The cranelift-generated trampoline will invoke a "transcoder libcall"
    ///   which is implemented natively in Rust that has a signature independent
    ///   of memory64 configuration options for example.
    Transcoder {
        /// The transcoding operation being performed.
        op: Transcode,
        /// The linear memory that the string is being read from.
        from: RuntimeMemoryIndex,
        /// Whether or not the source linear memory is 64-bit or not.
        from64: bool,
        /// The linear memory that the string is being written to.
        to: RuntimeMemoryIndex,
        /// Whether or not the destination linear memory is 64-bit or not.
        to64: bool,
    },

    /// A small adapter which simply traps, used for degenerate lift/lower
    /// combinations.
    AlwaysTrap,

    /// A `resource.new` intrinsic which will inject a new resource into the
    /// table specified.
    ResourceNew(TypeResourceTableIndex),

    /// Same as `ResourceNew`, but for the `resource.rep` intrinsic.
    ResourceRep(TypeResourceTableIndex),

    /// Same as `ResourceNew`, but for the `resource.drop` intrinsic.
    ResourceDrop(TypeResourceTableIndex),

    /// An intrinsic used by FACT-generated modules which will transfer an owned
    /// resource from one table to another. Used in component-to-component
    /// adapter trampolines.
    ResourceTransferOwn,

    /// Same as `ResourceTransferOwn` but for borrows.
    ResourceTransferBorrow,

    /// An intrinsic used by FACT-generated modules which indicates that a call
    /// is being entered and resource-related metadata needs to be configured.
    ///
    /// Note that this is currently only invoked when borrowed resources are
    /// detected, otherwise this is "optimized out".
    ResourceEnterCall,

    /// Same as `ResourceEnterCall` except for when exiting a call.
    ResourceExitCall,
}

impl Trampoline {
    /// Returns the name to use for the symbol of this trampoline in the final
    /// compiled artifact
    pub fn symbol_name(&self) -> String {
        use Trampoline::*;
        match self {
            LowerImport { index, .. } => {
                format!("component-lower-import[{}]", index.as_u32())
            }
            Transcoder {
                op, from64, to64, ..
            } => {
                let op = op.symbol_fragment();
                let from = if *from64 { "64" } else { "32" };
                let to = if *to64 { "64" } else { "32" };
                format!("component-transcode-{op}-m{from}-m{to}")
            }
            AlwaysTrap => format!("component-always-trap"),
            ResourceNew(i) => format!("component-resource-new[{}]", i.as_u32()),
            ResourceRep(i) => format!("component-resource-rep[{}]", i.as_u32()),
            ResourceDrop(i) => format!("component-resource-drop[{}]", i.as_u32()),
            ResourceTransferOwn => format!("component-resource-transfer-own"),
            ResourceTransferBorrow => format!("component-resource-transfer-borrow"),
            ResourceEnterCall => format!("component-resource-enter-call"),
            ResourceExitCall => format!("component-resource-exit-call"),
        }
    }
}
