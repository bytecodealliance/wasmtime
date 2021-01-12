use std::{env, fs::File, io::Write, path::Path};

fn main() -> std::io::Result<()> {
    let out_dir_env = env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir_env).canonicalize().unwrap();
    for (file, content) in wasi_crypto::witx_interfaces() {
        let path = out_dir.join(file);
        assert_eq!(path.parent().unwrap(), out_dir);
        let mut fp = File::create(path)?;
        fp.write_all(content.as_bytes())?;
    }
    Ok(())
}
