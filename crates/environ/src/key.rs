//! Keys for identifying functions during compilation, in call graphs, and when
//! resolving relocations.

#[cfg(feature = "component-model")]
use crate::component;
use crate::{
    BuiltinFunctionIndex, DefinedFuncIndex, HostCall, ModuleInternedTypeIndex, StaticModuleIndex,
};
use core::{cmp, fmt};
use serde_derive::{Deserialize, Serialize};

/// The kind of a function that is being compiled, linked, or otherwise
/// referenced.
///
/// This is like a `FuncKey` but without any payload values.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(test, derive(arbitrary::Arbitrary))]
pub enum FuncKeyKind {
    /// A Wasm-defined function.
    DefinedWasmFunction = FuncKey::new_kind(0b0000),

    /// A trampoline from an array-caller to the given Wasm-callee.
    ArrayToWasmTrampoline = FuncKey::new_kind(0b0001),

    /// A trampoline from a Wasm-caller to an array-callee of the given type.
    WasmToArrayTrampoline = FuncKey::new_kind(0b0010),

    /// A trampoline from a Wasm-caller to the given builtin.
    WasmToBuiltinTrampoline = FuncKey::new_kind(0b0011),

    /// A trampoline from the patchable ABI to the given builtin.
    PatchableToBuiltinTrampoline = FuncKey::new_kind(0b0100),

    /// A Pulley-specific host call.
    PulleyHostCall = FuncKey::new_kind(0b0101),

    /// A Wasm-caller to component builtin trampoline.
    #[cfg(feature = "component-model")]
    ComponentTrampoline = FuncKey::new_kind(0b0110),

    /// A Wasm-caller to array-callee `resource.drop` trampoline.
    #[cfg(feature = "component-model")]
    ResourceDropTrampoline = FuncKey::new_kind(0b0111),

    /// A Wasmtime unsafe intrinsic function.
    #[cfg(feature = "component-model")]
    UnsafeIntrinsic = FuncKey::new_kind(0b1000),
}

impl From<FuncKeyKind> for u32 {
    fn from(kind: FuncKeyKind) -> Self {
        kind as u32
    }
}

impl FuncKeyKind {
    /// Get this kind's raw representation.
    pub fn into_raw(self) -> u32 {
        self.into()
    }

    /// Construct a `FuncKind` from its raw representation.
    ///
    /// Panics when given invalid raw representations.
    pub fn from_raw(raw: u32) -> Self {
        match raw {
            x if x == Self::DefinedWasmFunction.into() => Self::DefinedWasmFunction,
            x if x == Self::ArrayToWasmTrampoline.into() => Self::ArrayToWasmTrampoline,
            x if x == Self::WasmToArrayTrampoline.into() => Self::WasmToArrayTrampoline,
            x if x == Self::WasmToBuiltinTrampoline.into() => Self::WasmToBuiltinTrampoline,
            x if x == Self::PatchableToBuiltinTrampoline.into() => {
                Self::PatchableToBuiltinTrampoline
            }
            x if x == Self::PulleyHostCall.into() => Self::PulleyHostCall,

            #[cfg(feature = "component-model")]
            x if x == Self::ComponentTrampoline.into() => Self::ComponentTrampoline,
            #[cfg(feature = "component-model")]
            x if x == Self::ResourceDropTrampoline.into() => Self::ResourceDropTrampoline,
            #[cfg(feature = "component-model")]
            x if x == Self::UnsafeIntrinsic.into() => Self::UnsafeIntrinsic,

            _ => panic!("invalid raw value passed to `FuncKind::from_raw`: {raw}"),
        }
    }
}

/// The namespace half of a `FuncKey`.
///
/// This is an opaque combination of the key's kind and module index, if any.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FuncKeyNamespace(u32);

impl fmt::Debug for FuncKeyNamespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Hex<T: fmt::LowerHex>(T);
        impl<T: fmt::LowerHex> fmt::Debug for Hex<T> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{:#x}", self.0)
            }
        }
        f.debug_struct("FuncKeyNamespace")
            .field("raw", &Hex(self.0))
            .field("kind", &self.kind())
            .field("module", &self.module())
            .finish()
    }
}

impl From<FuncKeyNamespace> for u32 {
    fn from(ns: FuncKeyNamespace) -> Self {
        ns.0
    }
}

impl FuncKeyNamespace {
    /// Get this `FuncNamespace`'s raw representation.
    pub fn into_raw(self) -> u32 {
        self.0
    }

    /// Construct a `FuncNamespace` from its raw representation.
    ///
    /// Panics when given invalid raw representations.
    pub fn from_raw(raw: u32) -> Self {
        match FuncKeyKind::from_raw(raw & FuncKey::KIND_MASK) {
            FuncKeyKind::DefinedWasmFunction | FuncKeyKind::ArrayToWasmTrampoline => Self(raw),
            FuncKeyKind::WasmToArrayTrampoline
            | FuncKeyKind::WasmToBuiltinTrampoline
            | FuncKeyKind::PatchableToBuiltinTrampoline
            | FuncKeyKind::PulleyHostCall => {
                assert_eq!(raw & FuncKey::MODULE_MASK, 0);
                Self(raw)
            }

            #[cfg(feature = "component-model")]
            FuncKeyKind::ComponentTrampoline => {
                let _ = Abi::from_raw(raw & FuncKey::MODULE_MASK);
                Self(raw)
            }

            #[cfg(feature = "component-model")]
            FuncKeyKind::ResourceDropTrampoline => {
                assert_eq!(raw & FuncKey::MODULE_MASK, 0);
                Self(raw)
            }

            #[cfg(feature = "component-model")]
            FuncKeyKind::UnsafeIntrinsic => {
                let _ = Abi::from_raw(raw & FuncKey::MODULE_MASK);
                Self(raw)
            }
        }
    }

    /// Get this `FuncNamespace`'s kind.
    pub fn kind(&self) -> FuncKeyKind {
        let raw = self.0 & FuncKey::KIND_MASK;
        FuncKeyKind::from_raw(raw)
    }

    fn module(&self) -> u32 {
        self.0 & FuncKey::MODULE_MASK
    }
}

/// The index half of a `FuncKey`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FuncKeyIndex(u32);

impl From<FuncKeyIndex> for u32 {
    fn from(index: FuncKeyIndex) -> Self {
        index.0
    }
}

impl FuncKeyIndex {
    /// Get this index's raw representation.
    pub fn into_raw(self) -> u32 {
        self.0
    }

    /// Construct a `FuncKeyIndex` from its raw representation.
    ///
    /// Invalid raw representations will not be caught eagerly, but will cause
    /// panics when paired with a `FuncKeyNamespace` to create a whole
    /// `FuncKey`.
    pub fn from_raw(raw: u32) -> Self {
        FuncKeyIndex(raw)
    }
}

/// ABI signature of functions that are generated here.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(test, derive(arbitrary::Arbitrary))]
pub enum Abi {
    /// The "wasm" ABI, or suitable to be a `wasm_call` field of a `VMFuncRef`.
    Wasm = 0,
    /// The "array" ABI, or suitable to be an `array_call` field.
    Array = 1,
    /// The "patchable" ABI. Signature same as Wasm ABI, but
    /// Cranelift-level (machine-level) ABI is different (no
    /// clobbers).
    Patchable = 2,
}

#[cfg(feature = "component-model")]
impl Abi {
    fn from_raw(raw: u32) -> Self {
        match raw {
            x if x == Self::Wasm.into_raw() => Self::Wasm,
            x if x == Self::Array.into_raw() => Self::Array,
            x if x == Self::Patchable.into_raw() => Self::Patchable,
            _ => panic!("invalid raw representation passed to `Abi::from_raw`: {raw}"),
        }
    }

    fn into_raw(self) -> u32 {
        (self as u8).into()
    }
}

/// A sortable, comparable function key for compilation output, call graph
/// edges, and relocations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FuncKey {
    /// A Wasm-defined function.
    DefinedWasmFunction(StaticModuleIndex, DefinedFuncIndex),

    /// A trampoline from an array-caller to the given Wasm-callee.
    ArrayToWasmTrampoline(StaticModuleIndex, DefinedFuncIndex),

    /// A trampoline from a Wasm-caller to an array-callee of the given type.
    WasmToArrayTrampoline(ModuleInternedTypeIndex),

    /// A trampoline from a Wasm-caller to the given builtin.
    WasmToBuiltinTrampoline(BuiltinFunctionIndex),

    /// A Pulley-specific host call.
    PulleyHostCall(HostCall),

    /// A trampoline from the patchable ABI to the given builtin.
    PatchableToBuiltinTrampoline(BuiltinFunctionIndex),

    /// A Wasm-caller to component builtin trampoline.
    #[cfg(feature = "component-model")]
    ComponentTrampoline(Abi, component::TrampolineIndex),

    /// A Wasm-caller to array-callee `resource.drop` trampoline.
    #[cfg(feature = "component-model")]
    ResourceDropTrampoline,

    /// A Wasmtime intrinsic function.
    #[cfg(feature = "component-model")]
    UnsafeIntrinsic(Abi, component::UnsafeIntrinsic),
}

impl Ord for FuncKey {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        // Make sure to sort by our raw parts, because `CompiledFunctionsTable`
        // relies on this for its binary search tables.
        let raw_self = self.into_raw_parts();
        let raw_other = other.into_raw_parts();
        raw_self.cmp(&raw_other)
    }
}

impl PartialOrd for FuncKey {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl FuncKey {
    const KIND_BITS: u32 = 4;
    const KIND_OFFSET: u32 = 32 - Self::KIND_BITS;
    const KIND_MASK: u32 = ((1 << Self::KIND_BITS) - 1) << Self::KIND_OFFSET;
    const MODULE_MASK: u32 = !Self::KIND_MASK;

    const fn new_kind(kind: u32) -> u32 {
        assert!(kind < (1 << Self::KIND_BITS));
        kind << Self::KIND_OFFSET
    }

    /// Split this key into its namespace and index halves.
    #[inline]
    pub fn into_parts(self) -> (FuncKeyNamespace, FuncKeyIndex) {
        let (namespace, index) = match self {
            FuncKey::DefinedWasmFunction(module, def_func) => {
                assert_eq!(module.as_u32() & Self::KIND_MASK, 0);
                let namespace = FuncKeyKind::DefinedWasmFunction.into_raw() | module.as_u32();
                let index = def_func.as_u32();
                (namespace, index)
            }
            FuncKey::ArrayToWasmTrampoline(module, def_func) => {
                assert_eq!(module.as_u32() & Self::KIND_MASK, 0);
                let namespace = FuncKeyKind::ArrayToWasmTrampoline.into_raw() | module.as_u32();
                let index = def_func.as_u32();
                (namespace, index)
            }
            FuncKey::WasmToArrayTrampoline(ty) => {
                let namespace = FuncKeyKind::WasmToArrayTrampoline.into_raw();
                let index = ty.as_u32();
                (namespace, index)
            }
            FuncKey::WasmToBuiltinTrampoline(builtin) => {
                let namespace = FuncKeyKind::WasmToBuiltinTrampoline.into_raw();
                let index = builtin.index();
                (namespace, index)
            }
            FuncKey::PatchableToBuiltinTrampoline(builtin) => {
                let namespace = FuncKeyKind::PatchableToBuiltinTrampoline.into_raw();
                let index = builtin.index();
                (namespace, index)
            }
            FuncKey::PulleyHostCall(host_call) => {
                let namespace = FuncKeyKind::PulleyHostCall.into_raw();
                let index = host_call.index();
                (namespace, index)
            }

            #[cfg(feature = "component-model")]
            FuncKey::ComponentTrampoline(abi, trampoline) => {
                let abi = abi.into_raw();
                assert_eq!(abi & Self::KIND_MASK, 0);
                let namespace = FuncKeyKind::ComponentTrampoline.into_raw() | abi;
                let index = trampoline.as_u32();
                (namespace, index)
            }
            #[cfg(feature = "component-model")]
            FuncKey::ResourceDropTrampoline => {
                let namespace = FuncKeyKind::ResourceDropTrampoline.into_raw();
                let index = 0;
                (namespace, index)
            }
            #[cfg(feature = "component-model")]
            FuncKey::UnsafeIntrinsic(abi, intrinsic) => {
                let abi = abi.into_raw();
                assert_eq!(abi & Self::KIND_MASK, 0);
                let namespace = FuncKeyKind::UnsafeIntrinsic.into_raw() | abi;
                let index = intrinsic.index();
                (namespace, index)
            }
        };
        (FuncKeyNamespace(namespace), FuncKeyIndex(index))
    }

    /// Get this key's kind.
    pub fn kind(self) -> FuncKeyKind {
        self.namespace().kind()
    }

    /// Get this key's namespace.
    pub fn namespace(self) -> FuncKeyNamespace {
        self.into_parts().0
    }

    /// Get this key's index.
    pub fn index(self) -> FuncKeyIndex {
        self.into_parts().1
    }

    /// Get ABI of the function that this key is defining.
    pub fn abi(self) -> Abi {
        match self {
            FuncKey::DefinedWasmFunction(_, _) => Abi::Wasm,
            FuncKey::ArrayToWasmTrampoline(_, _) => Abi::Array,
            FuncKey::WasmToArrayTrampoline(_) => Abi::Wasm,
            FuncKey::WasmToBuiltinTrampoline(_) => Abi::Wasm,
            FuncKey::PatchableToBuiltinTrampoline(_) => Abi::Patchable,
            FuncKey::PulleyHostCall(_) => Abi::Wasm,
            #[cfg(feature = "component-model")]
            FuncKey::ComponentTrampoline(abi, _) => abi,
            #[cfg(feature = "component-model")]
            FuncKey::ResourceDropTrampoline => Abi::Wasm,
            #[cfg(feature = "component-model")]
            FuncKey::UnsafeIntrinsic(abi, _) => abi,
        }
    }

    /// Get the raw, underlying `(namespace, index)` representation of this
    /// compilation key.
    ///
    /// The resulting values should only be used for (eventually) calling
    /// `FuncKey::from_raw_parts` or `FuncKey{Namespace,Index}::from_raw`.
    //
    // NB: We use two `u32`s to exactly match
    // `cranelift_codegen::ir::UserExternalName` and ensure that we can map
    // one-to-one between that and `FuncKey`.
    pub fn into_raw_parts(self) -> (u32, u32) {
        let (ns, index) = self.into_parts();
        (ns.into_raw(), index.into_raw())
    }

    /// Create a key from its namespace and index parts.
    ///
    /// Should only be called with namespaces and indices that are ultimately
    /// derived from the same key. For example, if you attempt to pair an index
    /// and namespace that come from different keys, that may panic. If it
    /// happens not to panic, you'll end up with a valid key that names an
    /// arbitrary function in the given namespace, but that function probably
    /// does not actually exist in the compilation artifact.
    pub fn from_parts(namespace: FuncKeyNamespace, index: FuncKeyIndex) -> Self {
        Self::from_raw_parts(namespace.into_raw(), index.into_raw())
    }

    /// Create a key from its raw, underlying representation.
    ///
    /// Should only be given the results of a previous call to
    /// `FuncKey::into_raw_parts`.
    ///
    /// Panics when given invalid raw parts.
    pub fn from_raw_parts(a: u32, b: u32) -> Self {
        match FuncKeyKind::from_raw(a & Self::KIND_MASK) {
            FuncKeyKind::DefinedWasmFunction => {
                let module = StaticModuleIndex::from_u32(a & Self::MODULE_MASK);
                let def_func = DefinedFuncIndex::from_u32(b);
                Self::DefinedWasmFunction(module, def_func)
            }
            FuncKeyKind::ArrayToWasmTrampoline => {
                let module = StaticModuleIndex::from_u32(a & Self::MODULE_MASK);
                let def_func = DefinedFuncIndex::from_u32(b);
                Self::ArrayToWasmTrampoline(module, def_func)
            }
            FuncKeyKind::WasmToArrayTrampoline => {
                assert_eq!(a & Self::MODULE_MASK, 0);
                let ty = ModuleInternedTypeIndex::from_u32(b);
                Self::WasmToArrayTrampoline(ty)
            }
            FuncKeyKind::WasmToBuiltinTrampoline => {
                assert_eq!(a & Self::MODULE_MASK, 0);
                let builtin = BuiltinFunctionIndex::from_u32(b);
                Self::WasmToBuiltinTrampoline(builtin)
            }
            FuncKeyKind::PatchableToBuiltinTrampoline => {
                assert_eq!(a & Self::MODULE_MASK, 0);
                let builtin = BuiltinFunctionIndex::from_u32(b);
                Self::PatchableToBuiltinTrampoline(builtin)
            }
            FuncKeyKind::PulleyHostCall => {
                assert_eq!(a & Self::MODULE_MASK, 0);
                let host_call = HostCall::from_index(b);
                Self::PulleyHostCall(host_call)
            }

            #[cfg(feature = "component-model")]
            FuncKeyKind::ComponentTrampoline => {
                let abi = Abi::from_raw(a & Self::MODULE_MASK);
                let trampoline = component::TrampolineIndex::from_u32(b);
                Self::ComponentTrampoline(abi, trampoline)
            }
            #[cfg(feature = "component-model")]
            FuncKeyKind::ResourceDropTrampoline => {
                assert_eq!(a & Self::MODULE_MASK, 0);
                assert_eq!(b, 0);
                Self::ResourceDropTrampoline
            }
            #[cfg(feature = "component-model")]
            FuncKeyKind::UnsafeIntrinsic => {
                let abi = Abi::from_raw(a & Self::MODULE_MASK);
                let intrinsic = component::UnsafeIntrinsic::from_u32(b);
                Self::UnsafeIntrinsic(abi, intrinsic)
            }
        }
    }

    /// Create a key from a raw packed `u64` representation.
    ///
    /// Should only be given a value produced by `into_raw_u64()`.
    ///
    /// Panics when given an invalid value.
    pub fn from_raw_u64(value: u64) -> Self {
        let hi = u32::try_from(value >> 32).unwrap();
        let lo = u32::try_from(value & 0xffff_ffff).unwrap();
        FuncKey::from_raw_parts(hi, lo)
    }

    /// Produce a packed `u64` representation of this key.
    ///
    /// May be used with `from_raw_64()` to reconstruct this key.
    pub fn into_raw_u64(&self) -> u64 {
        let (hi, lo) = self.into_raw_parts();
        (u64::from(hi) << 32) | u64::from(lo)
    }

    /// Unwrap a `FuncKey::DefinedWasmFunction` or else panic.
    pub fn unwrap_defined_wasm_function(self) -> (StaticModuleIndex, DefinedFuncIndex) {
        match self {
            Self::DefinedWasmFunction(module, def_func) => (module, def_func),
            _ => panic!("`FuncKey::unwrap_defined_wasm_function` called on {self:?}"),
        }
    }

    /// Unwrap a `FuncKey::ArrayToWasmTrampoline` or else panic.
    pub fn unwrap_array_to_wasm_trampoline(self) -> (StaticModuleIndex, DefinedFuncIndex) {
        match self {
            Self::ArrayToWasmTrampoline(module, def_func) => (module, def_func),
            _ => panic!("`FuncKey::unwrap_array_to_wasm_trampoline` called on {self:?}"),
        }
    }

    /// Unwrap a `FuncKey::WasmToArrayTrampoline` or else panic.
    pub fn unwrap_wasm_to_array_trampoline(self) -> ModuleInternedTypeIndex {
        match self {
            Self::WasmToArrayTrampoline(ty) => ty,
            _ => panic!("`FuncKey::unwrap_wasm_to_array_trampoline` called on {self:?}"),
        }
    }

    /// Unwrap a `FuncKey::WasmToBuiltinTrampoline` or else panic.
    pub fn unwrap_wasm_to_builtin_trampoline(self) -> BuiltinFunctionIndex {
        match self {
            Self::WasmToBuiltinTrampoline(builtin) => builtin,
            _ => panic!("`FuncKey::unwrap_wasm_to_builtin_trampoline` called on {self:?}"),
        }
    }

    /// Unwrap a `FuncKey::PulleyHostCall` or else panic.
    pub fn unwrap_pulley_host_call(self) -> HostCall {
        match self {
            Self::PulleyHostCall(host_call) => host_call,
            _ => panic!("`FuncKey::unwrap_pulley_host_call` called on {self:?}"),
        }
    }

    /// Unwrap a `FuncKey::ComponentTrampoline` or else panic.
    #[cfg(feature = "component-model")]
    pub fn unwrap_component_trampoline(self) -> (crate::Abi, component::TrampolineIndex) {
        match self {
            Self::ComponentTrampoline(abi, trampoline) => (abi, trampoline),
            _ => panic!("`FuncKey::unwrap_component_trampoline` called on {self:?}"),
        }
    }

    /// Unwrap a `FuncKey::ResourceDropTrampoline` or else panic.
    #[cfg(feature = "component-model")]
    pub fn unwrap_resource_drop_trampoline(self) {
        match self {
            Self::ResourceDropTrampoline => {}
            _ => panic!("`FuncKey::unwrap_resource_drop_trampoline` called on {self:?}"),
        }
    }

    /// Is this "Store-invariant"? This allows us to execute
    /// EngineCode directly rather than StoreCode.
    ///
    /// Any function that is either directly from Wasm code, or calls
    /// it directly (not indirected through a runtime-provided
    /// function pointer), is "store-variant": we need to use a
    /// StoreCode-specific version of the code to hit any patching
    /// that our specific instantiations may have (due to debugging
    /// breakpoints, etc). Trampolines into the runtime cannot be
    /// patched and so can use EngineCode instead. This allows for
    /// less complex plumbing in some places where we can avoid
    /// looking up the StoreCode (or having access to the Store).
    pub fn is_store_invariant(&self) -> bool {
        match self {
            Self::DefinedWasmFunction(..) | Self::ArrayToWasmTrampoline(..) => false,
            Self::WasmToArrayTrampoline(..)
            | Self::WasmToBuiltinTrampoline(..)
            | Self::PatchableToBuiltinTrampoline(..)
            | Self::PulleyHostCall(..) => true,
            #[cfg(feature = "component-model")]
            Self::ComponentTrampoline(..)
            | Self::ResourceDropTrampoline
            | Self::UnsafeIntrinsic(..) => true,
        }
    }
}
