(component
  (core module $m
    (func (export "run") (result i32)
      i32.const 0)
  )
  (core instance $i (instantiate $m))
  (func $run (result (result))
    (canon lift (core func $i "run")))

  (instance (export (interface "wasi:cli/run@0.2.0-rc-2023-12-05"))
    (export "run" (func $run)))
)
