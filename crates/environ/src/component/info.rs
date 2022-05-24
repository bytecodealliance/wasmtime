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
use crate::{EntityIndex, PrimaryMap};
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
    /// A list of typed values that this component imports, indexed by either
    /// the import's position or the name of the import.
    pub imports: IndexMap<String, TypeDef>,

    /// A list of this component's exports, indexed by either position or name.
    pub exports: IndexMap<String, Export>,

    /// The list of instances that this component creates during instantiation.
    ///
    /// Note that this is flattened/resolved from the original component to
    /// the point where alias annotations and such are not required. Instead
    /// the list of arguments to instantiate each module is provided as exports
    /// of prior instantiations.
    pub instances: PrimaryMap<RuntimeInstanceIndex, Instantiation>,
}

/// Different ways to instantiate a module at runtime.
#[derive(Debug, Serialize, Deserialize)]
pub enum Instantiation {
    /// A module "upvar" is being instantiated which is a closed-over module
    /// that is known at runtime by index.
    ModuleUpvar {
        /// The module index which is being instantiated.
        module: ModuleUpvarIndex,
        /// The flat list of arguments to the module's instantiation.
        args: Box<[CoreExport<EntityIndex>]>,
    },

    /// A module import is being instantiated.
    ///
    /// NB: this is not implemented in the runtime yet so this is a little less
    /// fleshed out than the above case. For example it's not entirely clear how
    /// the import will be referred to here (here a `usize` is used but unsure
    /// if that will work out).
    ModuleImport {
        /// Which module import is being instantiated.
        import_index: usize,
        /// The flat list of arguments to the module's instantiation.
        args: Box<[CoreExport<EntityIndex>]>,
    },
}

/// Identifier of an exported item from a core WebAssembly module instance.
///
/// Note that the `T` here is the index type for exports which can be
/// identified by index. The `T` is monomorphized with types like
/// [`EntityIndex`] or [`FuncIndex`].
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    LiftedFunction(LiftedFunction),
}

/// Description of a lifted function.
///
/// This represents how a function was lifted, what options were used to lift
/// it, and how it's all processed at runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiftedFunction {
    /// The component function type of the function being created.
    pub ty: FuncTypeIndex,
    /// Which core WebAssembly export is being lifted.
    pub func: CoreExport<FuncIndex>,
    /// Any options, if present, associated with this lifting.
    pub options: CanonicalOptions,
}

/// Canonical ABI options associated with a lifted function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalOptions {
    /// The encoding used for strings.
    pub string_encoding: StringEncoding,
    /// Representation of the `into` option where intrinsics are peeled out and
    /// identified from an instance.
    pub intrinsics: Option<Intrinsics>,
}

impl Default for CanonicalOptions {
    fn default() -> CanonicalOptions {
        CanonicalOptions {
            string_encoding: StringEncoding::Utf8,
            intrinsics: None,
        }
    }
}

/// Possible encodings of strings within the component model.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum StringEncoding {
    Utf8,
    Utf16,
    CompactUtf16,
}

/// Intrinsics required with the `(into $instance)` option specified in
/// `canon.lift`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intrinsics {
    /// The linear memory that the module exports which we're reading/writing
    /// from.
    pub memory: CoreExport<MemoryIndex>,

    /// A memory allocation, and reallocation, function.
    pub canonical_abi_realloc: CoreExport<FuncIndex>,

    /// A memory deallocation function.
    ///
    /// NB: this will probably be replaced with a per-export-destructor rather
    /// than a general memory deallocation function.
    pub canonical_abi_free: CoreExport<FuncIndex>,
}
