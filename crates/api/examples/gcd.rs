//! Example of instantiating of the WebAssembly module and
//! invoking its exported function.

use wasmtime::*;

const WAT: &str = r#"
(module
  (func $gcd (param i32 i32) (result i32)
    (local i32)
    block  ;; label = @1
      block  ;; label = @2
        local.get 0
        br_if 0 (;@2;)
        local.get 1
        local.set 2
        br 1 (;@1;)
      end
      loop  ;; label = @2
        local.get 1
        local.get 0
        local.tee 2
        i32.rem_u
        local.set 0
        local.get 2
        local.set 1
        local.get 0
        br_if 0 (;@2;)
      end
    end
    local.get 2
  )
  (export "gcd" (func $gcd))
)
"#;

fn main() -> anyhow::Result<()> {
    // Load our WebAssembly (parsed WAT in our case), and then load it into a
    // `Module` which is attached to a `Store` cache.
    let wasm = wat::parse_str(WAT)?;
    let store = Store::default();
    let module = Module::new(&store, &wasm)?;

    // Find index of the `gcd` export.
    let gcd_index = module
        .exports()
        .iter()
        .enumerate()
        .find(|(_, export)| export.name().to_string() == "gcd")
        .unwrap()
        .0;

    // Instantiate the module.
    let instance = Instance::new(&store, &module, &[])?;

    // Invoke `gcd` export
    let gcd = instance.exports()[gcd_index].func().expect("gcd");
    let result = gcd.call(&[Val::from(6i32), Val::from(27i32)])?;

    println!("{:?}", result);
    Ok(())
}
