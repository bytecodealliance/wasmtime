(module
  (func $imported (import "env" "f") (param i32) (result i32))
  (func $local (result externref externref funcref funcref)
    global.get 0
    global.get 1
    global.get 2
    global.get 3)

  (global (export "externref-imported") externref (ref.func $imported))
  (global (export "externref-local") externref (ref.func $local))
  (global (export "funcref-imported") funcref (ref.func $imported))
  (global (export "funcref-local") funcref (ref.func $local)))
