use crate::generators::Stacks;
use wasmtime::*;

/// Run the given `Stacks` test case and assert that the host's view of the Wasm
/// stack matches the test case's understanding of the Wasm stack.
///
/// Returns the maximum stack depth we checked.
pub fn check_stacks(stacks: Stacks) -> usize {
    let wasm = stacks.wasm();
    crate::oracles::log_wasm(&wasm);

    let engine = Engine::default();
    let module = Module::new(&engine, &wasm).expect("should compile okay");

    let mut linker = Linker::new(&engine);
    linker
        .func_wrap(
            "host",
            "check_stack",
            |mut caller: Caller<'_, ()>| -> Result<(), Trap> {
                let fuel = caller
                    .get_export("fuel")
                    .expect("should export `fuel`")
                    .into_global()
                    .expect("`fuel` export should be a global");

                let fuel_left = fuel.get(&mut caller).unwrap_i32();
                if fuel_left == 0 {
                    return Err(Trap::new("out of fuel"));
                }

                fuel.set(&mut caller, Val::I32(fuel_left - 1)).unwrap();
                Ok(())
            },
        )
        .unwrap();

    let mut store = Store::new(&engine, ());

    let instance = linker
        .instantiate(&mut store, &module)
        .expect("should instantiate okay");

    let run = instance
        .get_typed_func::<(u32,), (), _>(&mut store, "run")
        .expect("should export `run` function");

    let mut max_stack_depth = 0;
    for input in stacks.inputs().iter().copied() {
        if let Err(trap) = run.call(&mut store, (input.into(),)) {
            let get_stack = instance
                .get_typed_func::<(), (u32, u32), _>(&mut store, "get_stack")
                .expect("should export `get_stack` function as expected");

            let (ptr, len) = get_stack
                .call(&mut store, ())
                .expect("`get_stack` should not trap");

            let memory = instance
                .get_memory(&mut store, "memory")
                .expect("should have `memory` export");

            let host_trace = trap.trace().unwrap();
            max_stack_depth = max_stack_depth.max(host_trace.len());
            assert_stack_matches(&mut store, memory, ptr, len, host_trace);
        }
    }
    max_stack_depth
}

/// Assert that the Wasm program's view of the stack matches the host's view.
fn assert_stack_matches(
    store: &mut impl AsContextMut,
    memory: Memory,
    ptr: u32,
    len: u32,
    host_trace: &[FrameInfo],
) {
    let mut data = vec![0; len as usize];
    memory
        .read(&mut *store, ptr as usize, &mut data)
        .expect("should be in bounds");

    let mut wasm_trace = vec![];
    for entry in data.chunks(4).rev() {
        let mut bytes = [0; 4];
        bytes.copy_from_slice(entry);
        let entry = u32::from_le_bytes(bytes);
        wasm_trace.push(entry);
    }

    log::debug!("Wasm thinks its stack is: {:?}", wasm_trace);
    log::debug!(
        "Host thinks the stack is: {:?}",
        host_trace
            .iter()
            .map(|f| f.func_index())
            .collect::<Vec<_>>()
    );

    assert_eq!(wasm_trace.len(), host_trace.len());
    for (wasm_entry, host_entry) in wasm_trace.into_iter().zip(host_trace) {
        assert_eq!(wasm_entry, host_entry.func_index());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbitrary::{Arbitrary, Unstructured};
    use rand::prelude::*;

    const TARGET_STACK_DEPTH: usize = 10;

    #[test]
    fn smoke_test() {
        let mut rng = SmallRng::seed_from_u64(0);
        let mut buf = vec![0; 2048];

        for _ in 0..1024 {
            rng.fill_bytes(&mut buf);
            let u = Unstructured::new(&buf);
            if let Ok(stacks) = Stacks::arbitrary_take_rest(u) {
                let max_stack_depth = check_stacks(stacks);
                if max_stack_depth >= TARGET_STACK_DEPTH {
                    return;
                }
            }
        }

        panic!(
            "never generated a `Stacks` test case that reached {TARGET_STACK_DEPTH} \
             deep stack frames",
        );
    }
}
