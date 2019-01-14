extern crate lightbeam;

use lightbeam::translate;
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::Path;

fn read_to_end<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    let mut buffer = Vec::new();
    if path.as_ref() == Path::new("-") {
        let stdin = io::stdin();
        let mut stdin = stdin.lock();
        stdin.read_to_end(&mut buffer)?;
    } else {
        let mut file = File::open(path)?;
        file.read_to_end(&mut buffer)?;
    }
    Ok(buffer)
}

fn maybe_main() -> Result<(), String> {
    let data = read_to_end("test.wasm").map_err(|e| e.to_string())?;
    let translated = translate(&data).map_err(|e| e.to_string())?;
    let result: u32 = translated.execute_func(0, (5u32, 3u32)).unwrap();
    println!("f(5, 3) = {}", result);

    Ok(())
}

fn main() {
    match maybe_main() {
        Ok(()) => (),
        Err(e) => eprintln!("error: {}", e),
    }
}
