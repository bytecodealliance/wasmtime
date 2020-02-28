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
use std::collections::BTreeMap;
use std::mem;
use wasmparser::*;

#[derive(Arbitrary, Debug)]
struct Swarm {
    config_debug_info: bool,
    module_new: bool,
    module_drop: bool,
    instance_new: bool,
    instance_drop: bool,
    call_exported_func: bool,
}

/// A call to one of Wasmtime's public APIs.
#[derive(Arbitrary, Clone, Debug)]
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
    predicted_rss: usize,
    /// Map from a module id to the predicted amount of rss it will take to
    /// instantiate.
    modules: BTreeMap<usize, usize>,
    /// Map from an instance id to the amount of rss it's expected to be using.
    instances: BTreeMap<usize, usize>,
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
    fn arbitrary(input: &mut Unstructured) -> arbitrary::Result<Self> {
        let swarm = Swarm::arbitrary(input)?;
        let mut calls = vec![];

        arbitrary_config(input, &swarm, &mut calls)?;
        calls.push(EngineNew);
        calls.push(StoreNew);

        let mut scope = Scope::default();
        let max_rss = 1 << 30; // 1GB

        for _ in 0..input.arbitrary_len::<ApiCall>()? {
            let mut choices: Vec<fn(_, &mut Scope) -> arbitrary::Result<ApiCall>> = vec![];

            if swarm.module_new {
                choices.push(|input, scope| {
                    let id = scope.next_id();
                    let wasm = super::WasmOptTtf::arbitrary(input)?;
                    let predicted_rss = predict_rss(&wasm.wasm).unwrap_or(0);
                    scope.modules.insert(id, predicted_rss);
                    Ok(ModuleNew { id, wasm })
                });
            }
            if swarm.module_drop && !scope.modules.is_empty() {
                choices.push(|input, scope| {
                    let modules: Vec<_> = scope.modules.keys().collect();
                    let id = **input.choose(&modules)?;
                    scope.modules.remove(&id);
                    Ok(ModuleDrop { id })
                });
            }
            if swarm.instance_new && !scope.modules.is_empty() && scope.predicted_rss < max_rss {
                choices.push(|input, scope| {
                    let modules: Vec<_> = scope.modules.iter().collect();
                    let (&module, &predicted_rss) = *input.choose(&modules)?;
                    let id = scope.next_id();
                    scope.instances.insert(id, predicted_rss);
                    scope.predicted_rss += predicted_rss;
                    Ok(InstanceNew { id, module })
                });
            }
            if swarm.instance_drop && !scope.instances.is_empty() {
                choices.push(|input, scope| {
                    let instances: Vec<_> = scope.instances.iter().collect();
                    let (&id, &rss) = *input.choose(&instances)?;
                    scope.instances.remove(&id);
                    scope.predicted_rss -= rss;
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

            if choices.is_empty() {
                break;
            }
            let c = input.choose(&choices)?;
            calls.push(c(input, &mut scope)?);
        }

        Ok(ApiCalls { calls })
    }

    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        arbitrary::size_hint::recursion_guard(depth, |depth| {
            arbitrary::size_hint::or(
                // This is the stuff we unconditionally need, which affects the
                // minimum size.
                arbitrary::size_hint::and(
                    <Swarm as Arbitrary>::size_hint(depth),
                    // `arbitrary_config` uses two bools when
                    // `swarm.config_debug_info` is true.
                    <(bool, bool) as Arbitrary>::size_hint(depth),
                ),
                // We can generate arbitrary `WasmOptTtf` instances, which have
                // no upper bound on the number of bytes they consume. This sets
                // the upper bound to `None`.
                <super::WasmOptTtf as Arbitrary>::size_hint(depth),
            )
        })
    }
}

fn arbitrary_config(
    input: &mut Unstructured,
    swarm: &Swarm,
    calls: &mut Vec<ApiCall>,
) -> arbitrary::Result<()> {
    calls.push(ConfigNew);

    if swarm.config_debug_info && bool::arbitrary(input)? {
        calls.push(ConfigDebugInfo(bool::arbitrary(input)?));
    }

    // TODO: flags, features, and compilation strategy.

    Ok(())
}

/// Attempt to heuristically predict how much rss instantiating the `wasm`
/// provided will take in wasmtime.
///
/// The intention of this function is to prevent out-of-memory situations from
/// trivially instantiating a bunch of modules. We're basically taking any
/// random sequence of fuzz inputs and generating API calls, but if we
/// instantiate a million things we'd reasonably expect that to exceed the fuzz
/// limit of 2GB because, well, instantiation does take a bit of memory.
///
/// This prediction will prevent new instances from being created once we've
/// created a bunch of instances. Once instances start being dropped, though,
/// it'll free up new slots to start making new instances.
fn predict_rss(wasm: &[u8]) -> Result<usize> {
    let mut prediction = 0;
    let mut reader = ModuleReader::new(wasm)?;
    while !reader.eof() {
        let section = reader.read()?;
        match section.code {
            // For each declared memory we'll have to map that all in, so add in
            // the minimum amount of memory to our predicted rss.
            SectionCode::Memory => {
                for entry in section.get_memory_section_reader()? {
                    let initial = entry?.limits.initial as usize;
                    prediction += initial * 64 * 1024;
                }
            }

            // We'll need to allocate tables and space for table elements, and
            // currently this is 3 pointers per table entry.
            SectionCode::Table => {
                for entry in section.get_table_section_reader()? {
                    let initial = entry?.limits.initial as usize;
                    prediction += initial * 3 * mem::size_of::<usize>();
                }
            }

            // ... and for now nothing else is counted. If we run into issues
            // with the fuzzers though we can always try to take into account
            // more things
            _ => {}
        }
    }
    Ok(prediction)
}
