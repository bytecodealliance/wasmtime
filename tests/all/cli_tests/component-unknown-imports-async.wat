;; A component with undefined async imports, to exercise
;; `-W unknown-imports-default` on the concurrent stub path.
;;
;; An `async func`-typed import can only be satisfied by `func_new_concurrent`,
;; so stubbing these at all exercises a separate path from the sync case.
;;
;; `f` returns a `future`, which has no default. Defining it still succeeds, so
;; the component instantiates; calling it is what fails.
(component
  (import "foo" (func async (result u32)))
  (import "bar" (instance (export "baz" (func async))))
  (import "qux" (type (sub resource)))

  (type $f (func (result (future u32))))
  (import "f" (func $fn (type $f)))

  (core func $lowered (canon lower (func $fn)))
  (core module $m
    (import "" "f" (func $f (result i32)))
    (func (export "run") (result i32)
      (drop (call $f))
      i32.const 0)
  )
  (core instance $i (instantiate $m
    (with "" (instance (export "f" (func $lowered))))))

  (func $run (result (result))
    (canon lift (core func $i "run")))

  (instance (export (interface "wasi:cli/run@0.2.0"))
    (export "run" (func $run)))
)
