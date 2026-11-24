;;! flags = "-O signals-based-traps=n -W max-wasm-stack=65536"

;; Tests stack-overflow detection, with signals-based traps disabled, in a
;; function with enough results that Cranelift passes an implicit return-area
;; pointer, which must not disturb where the stack-overflow handler finds its
;; `vmctx` argument.

(module
  (func $f (export "f") (param $depth i32) (param $x i64)
    (result i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64)
    (if (result i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64) (local.get $depth)
      (then (call $f (i32.sub (local.get $depth) (i32.const 1))
                     (i64.add (local.get $x) (i64.const 1))))
      (else (local.get $x) (local.get $x) (local.get $x) (local.get $x) (local.get $x) (local.get $x) (local.get $x) (local.get $x) (local.get $x) (local.get $x) (local.get $x) (local.get $x) (local.get $x) (local.get $x) (local.get $x) (local.get $x))))
)

(assert_return (invoke "f" (i32.const 3) (i64.const 10))
  (i64.const 13) (i64.const 13) (i64.const 13) (i64.const 13)
  (i64.const 13) (i64.const 13) (i64.const 13) (i64.const 13)
  (i64.const 13) (i64.const 13) (i64.const 13) (i64.const 13)
  (i64.const 13) (i64.const 13) (i64.const 13) (i64.const 13))

(assert_exhaustion (invoke "f" (i32.const 0x7fffffff) (i64.const 0)) "call stack exhausted")

;; The instance is still usable afterwards.
(assert_return (invoke "f" (i32.const 1) (i64.const 5))
  (i64.const 6) (i64.const 6) (i64.const 6) (i64.const 6)
  (i64.const 6) (i64.const 6) (i64.const 6) (i64.const 6)
  (i64.const 6) (i64.const 6) (i64.const 6) (i64.const 6)
  (i64.const 6) (i64.const 6) (i64.const 6) (i64.const 6))
