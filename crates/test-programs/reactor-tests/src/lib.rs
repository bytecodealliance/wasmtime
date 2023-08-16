wit_bindgen::generate!("test-reactor" in "../../wasi/wit");

export_test_reactor!(T);

struct T;

static mut STATE: Vec<String> = Vec::new();

impl TestReactor for T {
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
        unsafe {
            for s in STATE.iter() {
                wasi::io::streams::write(o, s.as_bytes()).map_err(|_| ())?;
            }
            Ok(())
        }
    }
    fn pass_an_imported_record(stat: wasi::filesystem::types::DescriptorStat) -> String {
        format!("{stat:?}")
    }
}
