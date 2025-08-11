//! Keys for identifying functions during compilation, in call graphs, and when
//! resolving relocations.

#[cfg(feature = "component-model")]
use crate::component;
use crate::{
    BuiltinFunctionIndex, DefinedFuncIndex, HostCall, ModuleInternedTypeIndex, StaticModuleIndex,
};

/// A sortable, comparable function key for compilation output, call graph
/// edges, and relocations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

    /// A Wasm-caller to component builtin trampoline.
    #[cfg(feature = "component-model")]
    ComponentTrampoline(component::TrampolineIndex),

    /// A Wasm-caller to array-callee `resource.drop` trampoline.
    #[cfg(feature = "component-model")]
    ResourceDropTrampoline,
}

impl FuncKey {
    const KIND_BITS: u32 = 3;
    const KIND_OFFSET: u32 = 32 - Self::KIND_BITS;
    const KIND_MASK: u32 = ((1 << Self::KIND_BITS) - 1) << Self::KIND_OFFSET;
    const MODULE_MASK: u32 = !Self::KIND_MASK;

    const fn new_kind(kind: u32) -> u32 {
        assert!(kind < (1 << Self::KIND_BITS));
        kind << Self::KIND_OFFSET
    }

    const DEFINED_WASM_FUNCTION_KIND: u32 = Self::new_kind(0);
    const ARRAY_TO_WASM_TRAMPOLINE_KIND: u32 = Self::new_kind(1);
    const WASM_TO_ARRAY_TRAMPOLINE_KIND: u32 = Self::new_kind(2);
    const WASM_TO_BUILTIN_TRAMPOLINE_KIND: u32 = Self::new_kind(3);
    const PULLEY_HOST_CALL_KIND: u32 = Self::new_kind(4);

    #[cfg(feature = "component-model")]
    const COMPONENT_TRAMPOLINE_KIND: u32 = Self::new_kind(5);
    #[cfg(feature = "component-model")]
    const RESOURCE_DROP_TRAMPOLINE_KIND: u32 = Self::new_kind(6);

    /// Get the raw, underlying representation of this compilation key.
    ///
    /// The resulting values should only be used for (eventually) calling
    /// `CompileKey::from_raw_parts`.
    //
    // NB: We use two `u32`s to exactly match
    // `cranelift_codegen::ir::UserExternalName` and ensure that we can map
    // one-to-one between that and `FuncKey`.
    pub fn into_raw_parts(self) -> (u32, u32) {
        match self {
            FuncKey::DefinedWasmFunction(module, def_func) => {
                assert_eq!(module.as_u32() & Self::KIND_MASK, 0);
                let namespace = Self::DEFINED_WASM_FUNCTION_KIND | module.as_u32();
                let index = def_func.as_u32();
                (namespace, index)
            }
            FuncKey::ArrayToWasmTrampoline(module, def_func) => {
                assert_eq!(module.as_u32() & Self::KIND_MASK, 0);
                let namespace = Self::ARRAY_TO_WASM_TRAMPOLINE_KIND | module.as_u32();
                let index = def_func.as_u32();
                (namespace, index)
            }
            FuncKey::WasmToArrayTrampoline(ty) => {
                let namespace = Self::WASM_TO_ARRAY_TRAMPOLINE_KIND;
                let index = ty.as_u32();
                (namespace, index)
            }
            FuncKey::WasmToBuiltinTrampoline(builtin) => {
                let namespace = Self::WASM_TO_BUILTIN_TRAMPOLINE_KIND;
                let index = builtin.index();
                (namespace, index)
            }
            FuncKey::PulleyHostCall(host_call) => {
                let namespace = Self::PULLEY_HOST_CALL_KIND;
                let index = host_call.index();
                (namespace, index)
            }

            #[cfg(feature = "component-model")]
            FuncKey::ComponentTrampoline(trampoline) => {
                let namespace = Self::COMPONENT_TRAMPOLINE_KIND;
                let index = trampoline.as_u32();
                (namespace, index)
            }
            #[cfg(feature = "component-model")]
            FuncKey::ResourceDropTrampoline => {
                let namespace = Self::RESOURCE_DROP_TRAMPOLINE_KIND;
                let index = 0;
                (namespace, index)
            }
        }
    }

    /// Create a compilation key from its raw, underlying representation.
    ///
    /// Should only be given the results of a previous call to
    /// `CompileKey::into_raw_parts`.
    pub fn from_raw_parts(a: u32, b: u32) -> Self {
        match a & Self::KIND_MASK {
            Self::DEFINED_WASM_FUNCTION_KIND => {
                let module = StaticModuleIndex::from_u32(a & Self::MODULE_MASK);
                let def_func = DefinedFuncIndex::from_u32(b);
                Self::DefinedWasmFunction(module, def_func)
            }
            Self::ARRAY_TO_WASM_TRAMPOLINE_KIND => {
                let module = StaticModuleIndex::from_u32(a & Self::MODULE_MASK);
                let def_func = DefinedFuncIndex::from_u32(b);
                Self::ArrayToWasmTrampoline(module, def_func)
            }
            Self::WASM_TO_ARRAY_TRAMPOLINE_KIND => {
                assert_eq!(a & Self::MODULE_MASK, 0);
                let ty = ModuleInternedTypeIndex::from_u32(b);
                Self::WasmToArrayTrampoline(ty)
            }
            Self::WASM_TO_BUILTIN_TRAMPOLINE_KIND => {
                assert_eq!(a & Self::MODULE_MASK, 0);
                let builtin = BuiltinFunctionIndex::from_u32(b);
                Self::WasmToBuiltinTrampoline(builtin)
            }
            Self::PULLEY_HOST_CALL_KIND => {
                assert_eq!(a & Self::MODULE_MASK, 0);
                let host_call = HostCall::from_index(b);
                Self::PulleyHostCall(host_call)
            }

            #[cfg(feature = "component-model")]
            Self::COMPONENT_TRAMPOLINE_KIND => {
                assert_eq!(a & Self::MODULE_MASK, 0);
                let trampoline = component::TrampolineIndex::from_u32(b);
                Self::ComponentTrampoline(trampoline)
            }
            #[cfg(feature = "component-model")]
            Self::RESOURCE_DROP_TRAMPOLINE_KIND => {
                assert_eq!(a & Self::MODULE_MASK, 0);
                assert_eq!(b, 0);
                Self::ResourceDropTrampoline
            }

            k => panic!(
                "bad raw parts given to `FuncKey::from_raw_parts` call: ({a}, {b}), kind would be {k}"
            ),
        }
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
    pub fn unwrap_component_trampoline(self) -> component::TrampolineIndex {
        match self {
            Self::ComponentTrampoline(trampoline) => trampoline,
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
}
