;; A command which exports `wasi:cli/run` under the label `r`, as in
;; `export r: wasi:cli/run@0.2.0;`. The CLI only looks for the export named
;; after the interface itself, so it doesn't find anything to run here.
(component
  (core module $m
    (func (export "run") (result i32)
      i32.const 0)
  )
  (core instance $i (instantiate $m))
  (func $run (result (result))
    (canon lift (core func $i "run")))

  (instance $run (export "run" (func $run)))
  (export "r" (implements "wasi:cli/run@0.2.0") (instance $run))
)
