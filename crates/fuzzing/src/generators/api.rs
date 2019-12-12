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

use arbitrary::{Arbitrary, Unstructured};
use std::collections::HashSet;

struct Swarm {
    config_debug_info: bool,
    module_new: bool,
    module_drop: bool,
    instance_new: bool,
    instance_drop: bool,
    call_exported_func: bool,
}

impl Arbitrary for Swarm {
    fn arbitrary<U>(input: &mut U) -> Result<Self, U::Error>
    where
        U: Unstructured + ?Sized,
    {
        Ok(Swarm {
            config_debug_info: bool::arbitrary(input)?,
            module_new: bool::arbitrary(input)?,
            module_drop: bool::arbitrary(input)?,
            instance_new: bool::arbitrary(input)?,
            instance_drop: bool::arbitrary(input)?,
            call_exported_func: bool::arbitrary(input)?,
        })
    }
}

/// A call to one of Wasmtime's public APIs.
#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub enum ApiCall {
    ConfigNew,
    ConfigDebugInfo(bool),
    EngineNew,
    StoreNew,
    ModuleNew { id: usize, wasm: super::WasmOptTtf },
    ModuleDrop { id: usize },
    InstanceNew { id: usize, module: usize },
    InstanceDrop { id: usize },
    CallExportedFunc { instance: usize, nth: usize },
}
use ApiCall::*;

#[derive(Default)]
struct Scope {
    id_counter: usize,
    modules: HashSet<usize>,
    instances: HashSet<usize>,
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

impl Arbitrary for ApiCalls {
    fn arbitrary<U>(input: &mut U) -> Result<Self, U::Error>
    where
        U: Unstructured + ?Sized,
    {
        let swarm = Swarm::arbitrary(input)?;
        let mut calls = vec![];

        arbitrary_config(input, &swarm, &mut calls)?;
        calls.push(EngineNew);
        calls.push(StoreNew);

        let mut scope = Scope::default();

        for _ in 0..input.container_size()? {
            let mut choices: Vec<fn(_, &mut Scope) -> Result<ApiCall, U::Error>> = vec![];

            if swarm.module_new {
                choices.push(|input, scope| {
                    let id = scope.next_id();
                    scope.modules.insert(id);
                    let wasm = super::WasmOptTtf::arbitrary(input)?;
                    Ok(ModuleNew { id, wasm })
                });
            }
            if swarm.module_drop && !scope.modules.is_empty() {
                choices.push(|input, scope| {
                    let modules: Vec<_> = scope.modules.iter().cloned().collect();
                    let id = arbitrary_choice(input, &modules)?.cloned().unwrap();
                    scope.modules.remove(&id);
                    Ok(ModuleDrop { id })
                });
            }
            if swarm.instance_new && !scope.modules.is_empty() {
                choices.push(|input, scope| {
                    let modules: Vec<_> = scope.modules.iter().cloned().collect();
                    let module = arbitrary_choice(input, &modules)?.cloned().unwrap();
                    let id = scope.next_id();
                    scope.instances.insert(id);
                    Ok(InstanceNew { id, module })
                });
            }
            if swarm.instance_drop && !scope.instances.is_empty() {
                choices.push(|input, scope| {
                    let instances: Vec<_> = scope.instances.iter().cloned().collect();
                    let id = arbitrary_choice(input, &instances)?.cloned().unwrap();
                    scope.instances.remove(&id);
                    Ok(InstanceDrop { id })
                });
            }
            if swarm.call_exported_func && !scope.instances.is_empty() {
                choices.push(|input, scope| {
                    let instances: Vec<_> = scope.instances.iter().cloned().collect();
                    let instance = arbitrary_choice(input, &instances)?.cloned().unwrap();
                    let nth = usize::arbitrary(input)?;
                    Ok(CallExportedFunc { instance, nth })
                });
            }

            if let Some(c) = arbitrary_choice(input, &choices)? {
                calls.push(c(input, &mut scope)?);
            } else {
                break;
            }
        }

        Ok(ApiCalls { calls })
    }
}

fn arbitrary_choice<'a, T, U>(input: &mut U, choices: &'a [T]) -> Result<Option<&'a T>, U::Error>
where
    U: Unstructured + ?Sized,
{
    if choices.is_empty() {
        Ok(None)
    } else {
        let i = usize::arbitrary(input)? % choices.len();
        Ok(Some(&choices[i]))
    }
}

fn arbitrary_config<U>(
    input: &mut U,
    swarm: &Swarm,
    calls: &mut Vec<ApiCall>,
) -> Result<(), U::Error>
where
    U: Unstructured + ?Sized,
{
    calls.push(ConfigNew);

    if swarm.config_debug_info && bool::arbitrary(input)? {
        calls.push(ConfigDebugInfo(bool::arbitrary(input)?));
    }

    // TODO: flags, features, and compilation strategy.

    Ok(())
}
