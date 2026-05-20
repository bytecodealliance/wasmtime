//! Generating sequences of Wasmtime API calls.
//!
//! We only generate *valid* sequences of API calls. To do this, we keep track
//! of what objects we've already created in earlier API calls via the `Scope`
//! struct.
//!
//! To generate even-more-pathological sequences of API calls, we use [swarm
//! testing]:
//!
//! > In swarm testing, the usual practice of potentially including all features
//! > in every test case is abandoned. Rather, a large “swarm” of randomly
//! > generated configurations, each of which omits some features, is used, with
//! > configurations receiving equal resources.
//!
//! [swarm testing]: https://www.cs.utah.edu/~regehr/papers/swarm12.pdf

use crate::generators::Config;
use arbitrary::{Arbitrary, Unstructured};
use std::collections::{BTreeMap, BTreeSet};
use wasmtime::ValType;

/// The subset of Wasm value types that can be used for host-created globals.
#[derive(Arbitrary, Debug, Clone, Copy)]
#[expect(missing_docs, reason = "self-describing variants")]
pub enum FuzzValType {
    I32,
    I64,
    F32,
    F64,
}

impl FuzzValType {
    /// Convert to the corresponding `wasmtime::ValType`.
    pub fn to_wasmtime(self) -> ValType {
        match self {
            Self::I32 => ValType::I32,
            Self::I64 => ValType::I64,
            Self::F32 => ValType::F32,
            Self::F64 => ValType::F64,
        }
    }
}

#[derive(Arbitrary, Debug)]
struct Swarm {
    store_new: bool,
    store_drop: bool,
    module_new: bool,
    module_drop: bool,
    instance_new: bool,
    instance_drop: bool,
    call_exported_func: bool,
    global_type_new: bool,
    global_type_drop: bool,
    global_new: bool,
    global_get: bool,
    global_set: bool,
    global_ty: bool,
    global_drop: bool,
    get_global_export: bool,
}

/// A call to one of Wasmtime's public APIs.
#[derive(Arbitrary, Debug)]
#[expect(missing_docs, reason = "self-describing fields")]
pub enum ApiCall {
    StoreNew {
        id: usize,
        config: Config,
    },
    StoreDrop {
        id: usize,
    },
    ModuleNew {
        id: usize,
        wasm: Vec<u8>,
    },
    ModuleDrop {
        id: usize,
    },
    InstanceNew {
        id: usize,
        module: usize,
        store: usize,
    },
    InstanceDrop {
        id: usize,
    },
    CallExportedFunc {
        instance: usize,
        nth: usize,
    },
    GlobalTypeNew {
        id: usize,
        ty: FuzzValType,
        mutable: bool,
    },
    GlobalTypeDrop {
        id: usize,
    },
    GlobalNew {
        id: usize,
        global_ty: usize,
        store: usize,
    },
    GlobalGet {
        global: usize,
    },
    GlobalSet {
        global: usize,
    },
    GlobalTy {
        global: usize,
    },
    GlobalDrop {
        id: usize,
    },
    GetGlobalExport {
        id: usize,
        instance: usize,
        nth: usize,
    },
}
use ApiCall::*;

struct Scope {
    id_counter: usize,

    /// Stores that are currently live.
    stores: BTreeSet<usize>,

    /// Modules that are currently live.
    modules: BTreeSet<usize>,

    /// Instances that are currently live. Maps from `instance_id` to the
    /// instance's associated `store_id`.
    instances: BTreeMap<usize, usize>,

    /// Global types that are currently live.
    global_types: BTreeSet<usize>,

    /// Globals that are currently live. Maps from `global_id` to the global's
    /// associated `store_id`.
    globals: BTreeMap<usize, usize>, // global_id -> store_id

    config: Config,
}

impl Scope {
    fn next_id(&mut self) -> usize {
        let id = self.id_counter;
        self.id_counter = id + 1;
        id
    }
}

/// A sequence of API calls.
#[derive(Debug)]
pub struct ApiCalls {
    /// The API calls.
    pub calls: Vec<ApiCall>,
}

impl<'a> Arbitrary<'a> for ApiCalls {
    fn arbitrary(input: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        crate::init_fuzzing();

        let swarm = Swarm::arbitrary(input)?;
        let mut calls = vec![];

        let config = Config::arbitrary(input)?;
        let mut scope = Scope {
            id_counter: 0,
            stores: BTreeSet::default(),
            modules: BTreeSet::default(),
            instances: BTreeMap::default(),
            global_types: BTreeSet::default(),
            globals: BTreeMap::default(),
            config: config.clone(),
        };

        let store_id = scope.next_id();
        scope.stores.insert(store_id);
        calls.push(StoreNew {
            id: store_id,
            config,
        });

        // Total limit on number of API calls we'll generate. This exists to
        // avoid libFuzzer timeouts.
        let max_calls = 100;

        let mut choices: Vec<fn(&mut Unstructured<'a>, &mut Scope) -> arbitrary::Result<ApiCall>> =
            vec![];

        for _ in 0..std::cmp::min(max_calls, input.arbitrary_len::<ApiCall>()?) {
            choices.clear();

            if swarm.store_new {
                choices.push(|_input, scope| {
                    let id = scope.next_id();
                    scope.stores.insert(id);
                    Ok(StoreNew {
                        id,
                        config: scope.config.clone(),
                    })
                });
            }
            if swarm.store_drop && scope.stores.len() > 1 {
                choices.push(|input, scope| {
                    let stores: Vec<_> = scope.stores.iter().collect();
                    let id = **input.choose(&stores)?;
                    scope.stores.remove(&id);
                    scope.instances.retain(|_, store_id| *store_id != id);
                    scope.globals.retain(|_, store_id| *store_id != id);
                    Ok(StoreDrop { id })
                });
            }
            if swarm.module_new {
                choices.push(|input, scope| {
                    let id = scope.next_id();
                    let wasm = scope.config.generate(input, Some(1000))?;
                    scope.modules.insert(id);
                    Ok(ModuleNew {
                        id,
                        wasm: wasm.to_bytes(),
                    })
                });
            }
            if swarm.module_drop && !scope.modules.is_empty() {
                choices.push(|input, scope| {
                    let modules: Vec<_> = scope.modules.iter().collect();
                    let id = **input.choose(&modules)?;
                    scope.modules.remove(&id);
                    Ok(ModuleDrop { id })
                });
            }
            if swarm.instance_new && !scope.modules.is_empty() && !scope.stores.is_empty() {
                choices.push(|input, scope| {
                    let modules: Vec<_> = scope.modules.iter().collect();
                    let module = **input.choose(&modules)?;
                    let stores: Vec<_> = scope.stores.iter().collect();
                    let store = **input.choose(&stores)?;
                    let id = scope.next_id();
                    scope.instances.insert(id, store);
                    Ok(InstanceNew { id, module, store })
                });
            }
            if swarm.instance_drop && !scope.instances.is_empty() {
                choices.push(|input, scope| {
                    let instances: Vec<_> = scope.instances.keys().collect();
                    let id = **input.choose(&instances)?;
                    scope.instances.remove(&id);
                    Ok(InstanceDrop { id })
                });
            }
            if swarm.call_exported_func && !scope.instances.is_empty() {
                choices.push(|input, scope| {
                    let instances: Vec<_> = scope.instances.keys().collect();
                    let instance = **input.choose(&instances)?;
                    let nth = usize::arbitrary(input)?;
                    Ok(CallExportedFunc { instance, nth })
                });
            }
            if swarm.global_type_new {
                choices.push(|input, scope| {
                    let id = scope.next_id();
                    let ty = FuzzValType::arbitrary(input)?;
                    let mutable = bool::arbitrary(input)?;
                    scope.global_types.insert(id);
                    Ok(GlobalTypeNew { id, ty, mutable })
                });
            }
            if swarm.global_type_drop && !scope.global_types.is_empty() {
                choices.push(|input, scope| {
                    let types: Vec<_> = scope.global_types.iter().collect();
                    let id = **input.choose(&types)?;
                    scope.global_types.remove(&id);
                    Ok(GlobalTypeDrop { id })
                });
            }
            if swarm.global_new && !scope.global_types.is_empty() && !scope.stores.is_empty() {
                choices.push(|input, scope| {
                    let types: Vec<_> = scope.global_types.iter().collect();
                    let global_ty = **input.choose(&types)?;
                    let stores: Vec<_> = scope.stores.iter().collect();
                    let store = **input.choose(&stores)?;
                    let id = scope.next_id();
                    scope.globals.insert(id, store);
                    Ok(GlobalNew {
                        id,
                        global_ty,
                        store,
                    })
                });
            }
            if swarm.global_get && !scope.globals.is_empty() {
                choices.push(|input, scope| {
                    let globals: Vec<_> = scope.globals.keys().collect();
                    let global = **input.choose(&globals)?;
                    Ok(GlobalGet { global })
                });
            }
            if swarm.global_set && !scope.globals.is_empty() {
                choices.push(|input, scope| {
                    let globals: Vec<_> = scope.globals.keys().collect();
                    let global = **input.choose(&globals)?;
                    Ok(GlobalSet { global })
                });
            }
            if swarm.global_ty && !scope.globals.is_empty() {
                choices.push(|input, scope| {
                    let globals: Vec<_> = scope.globals.keys().collect();
                    let global = **input.choose(&globals)?;
                    Ok(GlobalTy { global })
                });
            }
            if swarm.global_drop && !scope.globals.is_empty() {
                choices.push(|input, scope| {
                    let globals: Vec<_> = scope.globals.keys().collect();
                    let id = **input.choose(&globals)?;
                    scope.globals.remove(&id);
                    Ok(GlobalDrop { id })
                });
            }
            if swarm.get_global_export && !scope.instances.is_empty() {
                choices.push(|input, scope| {
                    let instances: Vec<_> = scope.instances.keys().collect();
                    let instance = **input.choose(&instances)?;
                    let nth = usize::arbitrary(input)?;
                    let id = scope.next_id();
                    let store = *scope.instances.get(&instance).unwrap();
                    scope.globals.insert(id, store);
                    Ok(GetGlobalExport { id, instance, nth })
                });
            }

            if choices.is_empty() {
                break;
            }
            let c = input.choose(&choices)?;
            calls.push(c(input, &mut scope)?);
        }

        Ok(ApiCalls { calls })
    }
}
