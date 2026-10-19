;; A command which imports `wasi:cli/environment` under the label `env2`, as in
;; `import env2: wasi:cli/environment@0.3.0;`. The CLI only provides WASI under
;; each interface's own name, so this can't be instantiated.
(component
  (import "env2" (implements "wasi:cli/environment@0.3.0") (instance
    (export "get-arguments" (func (result (list string))))
  ))

  (core module $m
    (func (export "run") (result i32)
      i32.const 0)
  )
  (core instance $i (instantiate $m))
  (func $run (result (result))
    (canon lift (core func $i "run")))

  (instance (export (interface "wasi:cli/run@0.2.0"))
    (export "run" (func $run)))
)
