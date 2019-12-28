use wasmtime_rust::wasmtime;

#[wasmtime]
trait WasmMarkdown {
    fn render(&mut self, input: &str) -> String;
}

fn main() -> anyhow::Result<()> {
    let mut markdown = WasmMarkdown::load_bytes(std::fs::read("markdown.wasm")?)?;
    println!("{}", markdown.render("# Hello, Rust!"));

    Ok(())
}
