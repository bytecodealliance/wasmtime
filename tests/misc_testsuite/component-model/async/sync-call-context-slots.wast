;;! component_model_async = true
;;! component_model_more_async_builtins = true
;;! component_model_threading = true

;; Like sync-call-context.wast, but drives *both* component-context slots (0 and
;; 1) at once.

;; Both slots, inline fast path.
(component
  (component $A
    (core func $get0 (canon context.get i32 0))
    (core func $set0 (canon context.set i32 0))
    (core func $get1 (canon context.get i32 1))
    (core func $set1 (canon context.set i32 1))
    (core module $M
      (import "" "get0" (func $get0 (result i32)))
      (import "" "set0" (func $set0 (param i32)))
      (import "" "get1" (func $get1 (result i32)))
      (import "" "set1" (func $set1 (param i32)))
      (func (export "f'") (param i32) (result i32)
        ;; Fresh thread: both slots zero.
        (if (i32.ne (call $get0) (i32.const 0)) (then unreachable))
        (if (i32.ne (call $get1) (i32.const 0)) (then unreachable))
        (call $set0 (i32.const 0xAAAA1111))
        (call $set1 (i32.const 0xBBBB2222))
        ;; The two slots must not alias each other.
        (if (i32.ne (call $get0) (i32.const 0xAAAA1111)) (then unreachable))
        (if (i32.ne (call $get1) (i32.const 0xBBBB2222)) (then unreachable))
        (i32.add (local.get 0) (i32.const 42))
      )
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "get0" (func $get0)) (export "set0" (func $set0))
      (export "get1" (func $get1)) (export "set1" (func $set1))
    ))))
    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f'")))
  )

  (component $B
    (import "f" (func $f (param "x" u32) (result u32)))
    (core func $f' (canon lower (func $f)))
    (core func $get0 (canon context.get i32 0))
    (core func $set0 (canon context.set i32 0))
    (core func $get1 (canon context.get i32 1))
    (core func $set1 (canon context.set i32 1))
    (core module $N
      (import "" "f'" (func $f' (param i32) (result i32)))
      (import "" "get0" (func $get0 (result i32)))
      (import "" "set0" (func $set0 (param i32)))
      (import "" "get1" (func $get1 (result i32)))
      (import "" "set1" (func $set1 (param i32)))
      (func (export "g'") (result i32) (local $r i32)
        (call $set0 (i32.const 0x11110000))
        (call $set1 (i32.const 0x22220000))
        (local.set $r (call $f' (i32.const 1234)))
        ;; Both of our slots restored, independently and unswapped.
        (if (i32.ne (call $get0) (i32.const 0x11110000)) (then unreachable))
        (if (i32.ne (call $get1) (i32.const 0x22220000)) (then unreachable))
        (local.get $r)
      )
    )
    (core instance $n (instantiate $N (with "" (instance
      (export "f'" (func $f'))
      (export "get0" (func $get0)) (export "set0" (func $set0))
      (export "get1" (func $get1)) (export "set1" (func $set1))
    ))))
    (func (export "g") (result u32)
      (canon lift (core func $n "g'")))
  )

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f"))))
  (export "g" (func $b "g"))
)
(assert_return (invoke "g") (u32.const 1276))

;; Both slots, forced slow, out-of-line path.
(component
  (component $A
    (core func $get0 (canon context.get i32 0))
    (core func $set0 (canon context.set i32 0))
    (core func $get1 (canon context.get i32 1))
    (core func $set1 (canon context.set i32 1))
    (core func $bpinc (canon backpressure.inc))
    (core func $bpdec (canon backpressure.dec))
    (core module $M
      (import "" "get0" (func $get0 (result i32)))
      (import "" "set0" (func $set0 (param i32)))
      (import "" "get1" (func $get1 (result i32)))
      (import "" "set1" (func $set1 (param i32)))
      (import "" "backpressure.inc" (func $bpinc))
      (import "" "backpressure.dec" (func $bpdec))
      (func (export "f'") (param i32) (result i32)
        (if (i32.ne (call $get0) (i32.const 0)) (then unreachable))
        (if (i32.ne (call $get1) (i32.const 0)) (then unreachable))
        (call $set0 (i32.const 0xAAAA1111))
        (call $set1 (i32.const 0xBBBB2222))
        ;; Call `backptressure.{inc,dec}` to force lazy task creation.
        (call $bpinc) (call $bpdec)
        ;; Both slots survive the force.
        (if (i32.ne (call $get0) (i32.const 0xAAAA1111)) (then unreachable))
        (if (i32.ne (call $get1) (i32.const 0xBBBB2222)) (then unreachable))
        (i32.add (local.get 0) (i32.const 42))
      )
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "get0" (func $get0)) (export "set0" (func $set0))
      (export "get1" (func $get1)) (export "set1" (func $set1))
      (export "backpressure.inc" (func $bpinc))
      (export "backpressure.dec" (func $bpdec))
    ))))
    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f'")))
  )

  (component $B
    (import "f" (func $f (param "x" u32) (result u32)))
    (core func $f' (canon lower (func $f)))
    (core func $get0 (canon context.get i32 0))
    (core func $set0 (canon context.set i32 0))
    (core func $get1 (canon context.get i32 1))
    (core func $set1 (canon context.set i32 1))
    (core module $N
      (import "" "f'" (func $f' (param i32) (result i32)))
      (import "" "get0" (func $get0 (result i32)))
      (import "" "set0" (func $set0 (param i32)))
      (import "" "get1" (func $get1 (result i32)))
      (import "" "set1" (func $set1 (param i32)))
      (func (export "g'") (result i32) (local $r i32)
        (call $set0 (i32.const 0x11110000))
        (call $set1 (i32.const 0x22220000))
        (local.set $r (call $f' (i32.const 1234)))
        (if (i32.ne (call $get0) (i32.const 0x11110000)) (then unreachable))
        (if (i32.ne (call $get1) (i32.const 0x22220000)) (then unreachable))
        (local.get $r)
      )
    )
    (core instance $n (instantiate $N (with "" (instance
      (export "f'" (func $f'))
      (export "get0" (func $get0)) (export "set0" (func $set0))
      (export "get1" (func $get1)) (export "set1" (func $set1))
    ))))
    (func (export "g") (result u32)
      (canon lift (core func $n "g'")))
  )

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f"))))
  (export "g" (func $b "g"))
)
(assert_return (invoke "g") (u32.const 1276))
