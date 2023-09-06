wit_bindgen::generate!({
    world: "test-reactor",
    path: "../../wasi/wit",
    exports: {
        world: T,
    }
});

struct T;
use wasi::io::streams;
use wasi::poll::poll;

static mut STATE: Vec<String> = Vec::new();

struct DropPollable {
    pollable: poll::Pollable,
}

impl Drop for DropPollable {
    fn drop(&mut self) {
        poll::drop_pollable(self.pollable);
    }
}

impl Guest for T {
    fn add_strings(ss: Vec<String>) -> u32 {
        for s in ss {
            match s.split_once("$") {
                Some((prefix, var)) if prefix.is_empty() => match std::env::var(var) {
                    Ok(val) => unsafe { STATE.push(val) },
                    Err(_) => unsafe { STATE.push("undefined".to_owned()) },
                },
                _ => unsafe { STATE.push(s) },
            }
        }
        unsafe { STATE.len() as u32 }
    }
    fn get_strings() -> Vec<String> {
        unsafe { STATE.clone() }
    }

    fn write_strings_to(o: OutputStream) -> Result<(), ()> {
        let sub = DropPollable {
            pollable: streams::subscribe_to_output_stream(o),
        };
        unsafe {
            for s in STATE.iter() {
                let mut out = s.as_bytes();
                while !out.is_empty() {
                    poll::poll_oneoff(&[sub.pollable]);
                    let n = match streams::check_write(o) {
                        Ok(n) => n,
                        Err(_) => return Err(()),
                    };

                    let len = (n as usize).min(out.len());
                    match streams::write(o, &out[..len]) {
                        Ok(_) => out = &out[len..],
                        Err(_) => return Err(()),
                    }
                }
            }

            match streams::flush(o) {
                Ok(_) => {}
                Err(_) => return Err(()),
            }

            poll::poll_oneoff(&[sub.pollable]);
            match streams::check_write(o) {
                Ok(_) => {}
                Err(_) => return Err(()),
            }

            Ok(())
        }
    }
    fn pass_an_imported_record(stat: wasi::filesystem::types::DescriptorStat) -> String {
        format!("{stat:?}")
    }
}
