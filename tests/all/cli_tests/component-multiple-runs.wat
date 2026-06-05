;; This component has two different interfaces each exporting a function
;; called "run", which behave distinctly. wasi:cli/run's run returns ok,
;; some:other's run returns error.
(component
  (core module $m
    (func (export "run") (result i32)
      i32.const 0)
    (func (export "run2") (result i32)
      i32.const 1)
  )
  (core instance $i (instantiate $m))
  (func $run (result (result))
    (canon lift (core func $i "run")))
  (func $run2 (result (result))
    (canon lift (core func $i "run2")))

  (instance (export (interface "wasi:cli/run@0.2.0"))
    (export "run" (func $run)))

  (instance (export (interface "some:other/one"))
    (export "run" (func $run2)))
)
