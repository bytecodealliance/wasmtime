;;! flags = "-O signals-based-traps=n -W max-wasm-stack=524288"

;; Like `stack-overflow-big-frame-tiny-stack.wast`, but the stack is big enough
;; for a few frames before overflowing, so that the overflow is detected in a
;; function called from other wasm.
;;
;; Keeps 4100 `i64` values, which can't be rematerialized, live on the wasm
;; stack across a (maybe) recursive call, so that `$big`'s frame is at least
;; 32 KiB.

(module
  (type $many (func (result
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64)))

  (func $one (result i64) (i64.const 1))

  ;; Returns 100 ones.
  (func $many (type $many)
    (call $one) (call $one) (call $one) (call $one) (call $one) (call $one) (call $one) (call $one)
    (call $one) (call $one) (call $one) (call $one) (call $one) (call $one) (call $one) (call $one)
    (call $one) (call $one) (call $one) (call $one) (call $one) (call $one) (call $one) (call $one)
    (call $one) (call $one) (call $one) (call $one) (call $one) (call $one) (call $one) (call $one)
    (call $one) (call $one) (call $one) (call $one) (call $one) (call $one) (call $one) (call $one)
    (call $one) (call $one) (call $one) (call $one) (call $one) (call $one) (call $one) (call $one)
    (call $one) (call $one) (call $one) (call $one) (call $one) (call $one) (call $one) (call $one)
    (call $one) (call $one) (call $one) (call $one) (call $one) (call $one) (call $one) (call $one)
    (call $one) (call $one) (call $one) (call $one) (call $one) (call $one) (call $one) (call $one)
    (call $one) (call $one) (call $one) (call $one) (call $one) (call $one) (call $one) (call $one)
    (call $one) (call $one) (call $one) (call $one) (call $one) (call $one) (call $one) (call $one)
    (call $one) (call $one) (call $one) (call $one) (call $one) (call $one) (call $one) (call $one)
    (call $one) (call $one) (call $one) (call $one))

  ;; Consumes 100 values, and returns the sum of the first and last.
  (func $consume (param
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64
    i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64)
    (result i64)
    (i64.add (local.get 0) (local.get 99)))

  (func $big (export "big") (param $depth i32) (result i64)
    (local $sum i64)
    (call $many) (call $many) (call $many) (call $many) (call $many) (call $many)
    (call $many) (call $many) (call $many) (call $many) (call $many) (call $many)
    (call $many) (call $many) (call $many) (call $many) (call $many) (call $many)
    (call $many) (call $many) (call $many) (call $many) (call $many) (call $many)
    (call $many) (call $many) (call $many) (call $many) (call $many) (call $many)
    (call $many) (call $many) (call $many) (call $many) (call $many) (call $many)
    (call $many) (call $many) (call $many) (call $many) (call $many)
    (if (local.get $depth)
      (then (local.set $sum (call $big (i32.sub (local.get $depth) (i32.const 1))))))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.set $sum (i64.add (call $consume) (local.get $sum)))
    (local.get $sum))

  (func (export "ok") (result i64) (call $one))
)

;; A few levels of recursion fit fine, and the stack-limit check in the prologue
;; doesn't disturb the function's arguments.
(assert_return (invoke "big" (i32.const 3)) (i64.const 328))

;; Unbounded recursion overflows the stack.
(assert_exhaustion (invoke "big" (i32.const 1000000)) "call stack exhausted")

;; A core wasm trap doesn't poison the instance or store, so the instance is
;; still usable afterwards.
(assert_return (invoke "big" (i32.const 0)) (i64.const 82))
(assert_return (invoke "ok") (i64.const 1))
