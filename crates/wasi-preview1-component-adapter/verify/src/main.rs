use anyhow::{Result, bail};
use std::env;
use wasmparser::*;

fn main() -> Result<()> {
    let file = env::args()
        .nth(1)
        .expect("must pass wasm file as an argument");
    let wasm = wat::parse_file(&file)?;

    let mut validator = Validator::new();
    for payload in Parser::new(0).parse_all(&wasm) {
        let payload = payload?;
        validator.payload(&payload)?;
        match payload {
            Payload::Version { encoding, .. } => {
                if encoding != Encoding::Module {
                    bail!("adapter must be a core wasm module, not a component");
                }
            }
            Payload::End(_) => {}
            Payload::TypeSection(_) => {}
            Payload::ImportSection(s) => {
                for i in s {
                    let i = i?;
                    match i.ty {
                        TypeRef::Func(_) => {
                            if i.module.starts_with("wasi:") {
                                continue;
                            }
                            if i.module == "__main_module__" {
                                continue;
                            }
                            bail!("import from unknown module `{}`", i.module);
                        }
                        TypeRef::Table(_) => bail!("should not import table"),
                        TypeRef::Global(_) => bail!("should not import globals"),
                        TypeRef::Memory(_) => {}
                        TypeRef::Tag(_) => bail!("unsupported `tag` type"),
                    }
                }
            }
            Payload::TableSection(_) => {}
            Payload::MemorySection(_) => {
                bail!("preview1.wasm should import memory");
            }
            Payload::GlobalSection(_) => {}

            Payload::ExportSection(_) => {}

            Payload::FunctionSection(_) => {}

            Payload::CodeSectionStart { .. } => {}
            Payload::CodeSectionEntry(_) => {}
            Payload::CustomSection(_) => {}

            // sections that shouldn't appear in the specially-crafted core wasm
            // adapter self we're processing
            _ => {
                bail!("unsupported section {payload:?} found in preview1.wasm")
            }
        }
    }

    Ok(())
}
