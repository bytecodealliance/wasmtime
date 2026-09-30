;; A component whose two instance imports are meant to be satisfied at compile
;; time by `host-api-a.wat` and `host-api-b.wat`. It traps unless both builtins
;; are actually wired up and return their expected values.
(component
  (import "host-api-a" (instance $a
    (export "get" (func (result u32)))))
  (import "host-api-b" (instance $b
    (export "get" (func (result u32)))))

  (core func $a-get (canon lower (func $a "get")))
  (core func $b-get (canon lower (func $b "get")))

  (core module $m
    (import "" "a-get" (func $a-get (result i32)))
    (import "" "b-get" (func $b-get (result i32)))
    (func (export "run") (result i32)
      (if (i32.ne (call $a-get) (i32.const 42))
        (then unreachable))
      (if (i32.ne (call $b-get) (i32.const 100))
        (then unreachable))
      i32.const 0)
  )

  (core instance $i
    (instantiate $m
      (with "" (instance (export "a-get" (func $a-get))
                         (export "b-get" (func $b-get))))))

  (func $run (result (result))
    (canon lift (core func $i "run")))

  (instance (export (interface "wasi:cli/run@0.2.0"))
    (export "run" (func $run)))
)
