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

use crate::generators::{Config, ModuleConfig};
use arbitrary::{Arbitrary, Unstructured};
use std::collections::BTreeSet;

#[derive(Arbitrary, Debug)]
struct Swarm {
    module_new: bool,
    module_drop: bool,
    instance_new: bool,
    instance_drop: bool,
    call_exported_func: bool,
}

/// A call to one of Wasmtime's public APIs.
#[derive(Arbitrary, Debug)]
#[allow(missing_docs)]
pub enum ApiCall {
    StoreNew(Config),
    ModuleNew { id: usize, wasm: Vec<u8> },
    ModuleDrop { id: usize },
    InstanceNew { id: usize, module: usize },
    InstanceDrop { id: usize },
    CallExportedFunc { instance: usize, nth: usize },
}
use ApiCall::*;

struct Scope {
    id_counter: usize,
    modules: BTreeSet<usize>,
    instances: BTreeSet<usize>,
    module_config: ModuleConfig,
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
        let module_config = config.module_config.clone();
        calls.push(StoreNew(config));

        let mut scope = Scope {
            id_counter: 0,
            modules: BTreeSet::default(),
            instances: BTreeSet::default(),
            module_config,
        };

        // Total limit on number of API calls we'll generate. This exists to
        // avoid libFuzzer timeouts.
        let max_calls = 100;

        for _ in 0..input.arbitrary_len::<ApiCall>()? {
            if calls.len() > max_calls {
                break;
            }

            let mut choices: Vec<fn(_, &mut Scope) -> arbitrary::Result<ApiCall>> = vec![];

            if swarm.module_new {
                choices.push(|input, scope| {
                    let id = scope.next_id();
                    let mut wasm = scope.module_config.generate(input)?;
                    wasm.ensure_termination(1000);
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
            if swarm.instance_new && !scope.modules.is_empty() {
                choices.push(|input, scope| {
                    let modules: Vec<_> = scope.modules.iter().collect();
                    let module = **input.choose(&modules)?;
                    let id = scope.next_id();
                    scope.instances.insert(id);
                    Ok(InstanceNew { id, module })
                });
            }
            if swarm.instance_drop && !scope.instances.is_empty() {
                choices.push(|input, scope| {
                    let instances: Vec<_> = scope.instances.iter().collect();
                    let id = **input.choose(&instances)?;
                    scope.instances.remove(&id);
                    Ok(InstanceDrop { id })
                });
            }
            if swarm.call_exported_func && !scope.instances.is_empty() {
                choices.push(|input, scope| {
                    let instances: Vec<_> = scope.instances.iter().collect();
                    let instance = **input.choose(&instances)?;
                    let nth = usize::arbitrary(input)?;
                    Ok(CallExportedFunc { instance, nth })
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
