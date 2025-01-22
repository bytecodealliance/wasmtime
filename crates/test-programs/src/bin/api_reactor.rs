use std::sync::{Mutex, MutexGuard};

wit_bindgen::generate!({
    world: "test-reactor",
    path: "../wasi/src/p2/wit",
    generate_all,
});

struct T;

export!(T);

fn state() -> MutexGuard<'static, Vec<String>> {
    static STATE: Mutex<Vec<String>> = Mutex::new(Vec::new());
    STATE.lock().unwrap()
}

impl Guest for T {
    fn add_strings(ss: Vec<String>) -> u32 {
        let mut state = state();
        for s in ss {
            match s.split_once("$") {
                Some((prefix, var)) if prefix.is_empty() => match std::env::var(var) {
                    Ok(val) => state.push(val),
                    Err(_) => state.push("undefined".to_owned()),
                },
                _ => state.push(s),
            }
        }
        state.len() as u32
    }
    fn get_strings() -> Vec<String> {
        state().clone()
    }

    fn write_strings_to(o: OutputStream) -> Result<(), ()> {
        let pollable = o.subscribe();
        for s in state().iter() {
            let mut out = s.as_bytes();
            while !out.is_empty() {
                pollable.block();
                let n = match o.check_write() {
                    Ok(n) => n,
                    Err(_) => return Err(()),
                };

                let len = (n as usize).min(out.len());
                match o.write(&out[..len]) {
                    Ok(_) => out = &out[len..],
                    Err(_) => return Err(()),
                }
            }
        }

        match o.flush() {
            Ok(_) => {}
            Err(_) => return Err(()),
        }
        pollable.block();
        match o.check_write() {
            Ok(_) => {}
            Err(_) => return Err(()),
        }

        Ok(())
    }
    fn pass_an_imported_record(stat: wasi::filesystem::types::DescriptorStat) -> String {
        format!("{stat:?}")
    }
}

// Technically this should not be here for a reactor, but given the current
// framework for tests it's required since this file is built as a `bin`
fn main() {}
