use lightbeam::translate;

const WAT: &str = r#"
(module
  (func (param i32) (param i32) (result i32) (i32.add (get_local 0) (get_local 1)))
)
"#;

fn main() -> anyhow::Result<()> {
    let data = wat::parse_str(WAT)?;
    let translated = translate(&data)?;
    let result: u32 = translated.execute_func(0, (5u32, 3u32))?;
    println!("f(5, 3) = {}", result);

    Ok(())
}
