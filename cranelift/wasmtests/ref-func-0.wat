(module
  (func $imported (import "env" "f") (param i32) (result i32))
  (func $local (result anyref anyref funcref funcref)
    global.get 0
    global.get 1
    global.get 2
    global.get 3)

  (global (export "anyref-imported") anyref (ref.func $imported))
  (global (export "anyref-local") anyref (ref.func $local))
  (global (export "funcref-imported") funcref (ref.func $imported))
  (global (export "funcref-local") funcref (ref.func $local)))
