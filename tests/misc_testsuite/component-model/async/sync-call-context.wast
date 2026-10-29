;;! component_model_async = true
;;! component_model_more_async_builtins = true

;; Tests for guest-to-guest, sync-to-sync calls and their inline fast paths.

;; Inline fast path, no task forcing.
(component
  (component $A
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))
    (core module $M
      (import "" "context.get" (func $cget (result i32)))
      (import "" "context.set" (func $cset (param i32)))
      (func (export "f'") (param i32) (result i32)
        ;; A is a freshly-entered (deferred) thread: its context starts at 0.
        (if (i32.ne (call $cget) (i32.const 0)) (then unreachable))
        (call $cset (i32.const 0x5678))
        (if (i32.ne (call $cget) (i32.const 0x5678)) (then unreachable))
        (i32.add (local.get 0) (i32.const 42))
      )
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "context.get" (func $cget))
      (export "context.set" (func $cset))
    ))))
    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f'")))
  )

  (component $B
    (import "f" (func $f (param "x" u32) (result u32)))
    (core func $f' (canon lower (func $f)))
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))
    (core module $N
      (import "" "f'" (func $f' (param i32) (result i32)))
      (import "" "context.get" (func $cget (result i32)))
      (import "" "context.set" (func $cset (param i32)))
      (func (export "g'") (result i32) (local $r i32)
        (call $cset (i32.const 0x1234))
        (if (i32.ne (call $cget) (i32.const 0x1234)) (then unreachable))
        (local.set $r (call $f' (i32.const 1234)))
        ;; The callee's context mutation must NOT leak: ours is restored.
        (if (i32.ne (call $cget) (i32.const 0x1234)) (then unreachable))
        (local.get $r)
      )
    )
    (core instance $n (instantiate $N (with "" (instance
      (export "f'" (func $f'))
      (export "context.get" (func $cget))
      (export "context.set" (func $cset))
    ))))
    (func (export "g") (result u32)
      (canon lift (core func $n "g'")))
  )

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f"))))
  (export "g" (func $b "g"))
)
(assert_return (invoke "g") (u32.const 1276))

;; Single guest-to-guest sync call, forced slow path.
(component
  (component $A
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))
    (core func $bpinc (canon backpressure.inc))
    (core func $bpdec (canon backpressure.dec))
    (core module $M
      (import "" "context.get" (func $cget (result i32)))
      (import "" "context.set" (func $cset (param i32)))
      (import "" "backpressure.inc" (func $bpinc))
      (import "" "backpressure.dec" (func $bpdec))
      (func (export "f'") (param i32) (result i32)
        (if (i32.ne (call $cget) (i32.const 0)) (then unreachable))
        (call $cset (i32.const 0x5678))
        ;; Force promotion of the deferred thread mid-call.
        (call $bpinc)
        (call $bpdec)
        ;; Our context must survive the force.
        (if (i32.ne (call $cget) (i32.const 0x5678)) (then unreachable))
        (i32.add (local.get 0) (i32.const 42))
      )
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "context.get" (func $cget))
      (export "context.set" (func $cset))
      (export "backpressure.inc" (func $bpinc))
      (export "backpressure.dec" (func $bpdec))
    ))))
    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f'")))
  )

  (component $B
    (import "f" (func $f (param "x" u32) (result u32)))
    (core func $f' (canon lower (func $f)))
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))
    (core module $N
      (import "" "f'" (func $f' (param i32) (result i32)))
      (import "" "context.get" (func $cget (result i32)))
      (import "" "context.set" (func $cset (param i32)))
      (func (export "g'") (result i32) (local $r i32)
        (call $cset (i32.const 0x1234))
        (local.set $r (call $f' (i32.const 1234)))
        ;; Restored even though the callee forced the slow exit path.
        (if (i32.ne (call $cget) (i32.const 0x1234)) (then unreachable))
        (local.get $r)
      )
    )
    (core instance $n (instantiate $N (with "" (instance
      (export "f'" (func $f'))
      (export "context.get" (func $cget))
      (export "context.set" (func $cset))
    ))))
    (func (export "g") (result u32)
      (canon lift (core func $n "g'")))
  )

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f"))))
  (export "g" (func $b "g"))
)
(assert_return (invoke "g") (u32.const 1276))

;; Nested A->B->C sync-call chain, fast path.
(component
  (component $Leaf
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))
    (core module $M
      (import "" "context.get" (func $cget (result i32)))
      (import "" "context.set" (func $cset (param i32)))
      (func (export "leaf'") (param i32) (result i32)
        (if (i32.ne (call $cget) (i32.const 0)) (then unreachable))
        (call $cset (i32.const 0x99aabbcc))
        (if (i32.ne (call $cget) (i32.const 0x99aabbcc)) (then unreachable))
        (i32.add (local.get 0) (i32.const 1))
      )
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "context.get" (func $cget))
      (export "context.set" (func $cset))
    ))))
    (func (export "leaf") (param "x" u32) (result u32)
      (canon lift (core func $m "leaf'")))
  )

  (component $Mid
    (import "leaf" (func $leaf (param "x" u32) (result u32)))
    (core func $leaf' (canon lower (func $leaf)))
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))
    (core module $M
      (import "" "leaf'" (func $leaf' (param i32) (result i32)))
      (import "" "context.get" (func $cget (result i32)))
      (import "" "context.set" (func $cset (param i32)))
      (func (export "mid'") (param i32) (result i32) (local $r i32)
        (if (i32.ne (call $cget) (i32.const 0)) (then unreachable))
        (call $cset (i32.const 0x55667788))
        (local.set $r (call $leaf' (local.get 0)))
        (if (i32.ne (call $cget) (i32.const 0x55667788)) (then unreachable))
        (i32.add (local.get $r) (i32.const 10))
      )
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "leaf'" (func $leaf'))
      (export "context.get" (func $cget))
      (export "context.set" (func $cset))
    ))))
    (func (export "mid") (param "x" u32) (result u32)
      (canon lift (core func $m "mid'")))
  )

  (component $Root
    (import "mid" (func $mid (param "x" u32) (result u32)))
    (core func $mid' (canon lower (func $mid)))
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))
    (core module $M
      (import "" "mid'" (func $mid' (param i32) (result i32)))
      (import "" "context.get" (func $cget (result i32)))
      (import "" "context.set" (func $cset (param i32)))
      (func (export "root'") (result i32) (local $r i32)
        (call $cset (i32.const 0x11223344))
        (local.set $r (call $mid' (i32.const 100)))
        (if (i32.ne (call $cget) (i32.const 0x11223344)) (then unreachable))
        (i32.add (local.get $r) (i32.const 1000))
      )
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "mid'" (func $mid'))
      (export "context.get" (func $cget))
      (export "context.set" (func $cset))
    ))))
    (func (export "root") (result u32)
      (canon lift (core func $m "root'")))
  )

  (instance $leaf (instantiate $Leaf))
  (instance $mid (instantiate $Mid (with "leaf" (func $leaf "leaf"))))
  (instance $root (instantiate $Root (with "mid" (func $mid "mid"))))
  (export "root" (func $root "root"))
)
(assert_return (invoke "root") (u32.const 1111))

;; Nested A->B->C call chain, forced at the deepest level.
(component
  (component $Leaf
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))
    (core func $bpinc (canon backpressure.inc))
    (core func $bpdec (canon backpressure.dec))
    (core module $M
      (import "" "context.get" (func $cget (result i32)))
      (import "" "context.set" (func $cset (param i32)))
      (import "" "backpressure.inc" (func $bpinc))
      (import "" "backpressure.dec" (func $bpdec))
      (func (export "leaf'") (param i32) (result i32)
        (if (i32.ne (call $cget) (i32.const 0)) (then unreachable))
        (call $cset (i32.const 0x99aabbcc))
        ;; Force promotion.
        (call $bpinc)
        (call $bpdec)
        (if (i32.ne (call $cget) (i32.const 0x99aabbcc)) (then unreachable))
        (i32.add (local.get 0) (i32.const 1))
      )
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "context.get" (func $cget))
      (export "context.set" (func $cset))
      (export "backpressure.inc" (func $bpinc))
      (export "backpressure.dec" (func $bpdec))
    ))))
    (func (export "leaf") (param "x" u32) (result u32)
      (canon lift (core func $m "leaf'")))
  )

  (component $Mid
    (import "leaf" (func $leaf (param "x" u32) (result u32)))
    (core func $leaf' (canon lower (func $leaf)))
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))
    (core module $M
      (import "" "leaf'" (func $leaf' (param i32) (result i32)))
      (import "" "context.get" (func $cget (result i32)))
      (import "" "context.set" (func $cset (param i32)))
      (func (export "mid'") (param i32) (result i32) (local $r i32)
        (if (i32.ne (call $cget) (i32.const 0)) (then unreachable))
        (call $cset (i32.const 0x55667788))
        (local.set $r (call $leaf' (local.get 0)))
        ;; Restored after the (forced) nested call.
        (if (i32.ne (call $cget) (i32.const 0x55667788)) (then unreachable))
        (i32.add (local.get $r) (i32.const 10))
      )
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "leaf'" (func $leaf'))
      (export "context.get" (func $cget))
      (export "context.set" (func $cset))
    ))))
    (func (export "mid") (param "x" u32) (result u32)
      (canon lift (core func $m "mid'")))
  )

  (component $Root
    (import "mid" (func $mid (param "x" u32) (result u32)))
    (core func $mid' (canon lower (func $mid)))
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))
    (core module $M
      (import "" "mid'" (func $mid' (param i32) (result i32)))
      (import "" "context.get" (func $cget (result i32)))
      (import "" "context.set" (func $cset (param i32)))
      (func (export "root'") (result i32) (local $r i32)
        (call $cset (i32.const 0x11223344))
        (local.set $r (call $mid' (i32.const 100)))
        (if (i32.ne (call $cget) (i32.const 0x11223344)) (then unreachable))
        (i32.add (local.get $r) (i32.const 1000))
      )
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "mid'" (func $mid'))
      (export "context.get" (func $cget))
      (export "context.set" (func $cset))
    ))))
    (func (export "root") (result u32)
      (canon lift (core func $m "root'")))
  )

  (instance $leaf (instantiate $Leaf))
  (instance $mid (instantiate $Mid (with "leaf" (func $leaf "leaf"))))
  (instance $root (instantiate $Root (with "mid" (func $mid "mid"))))
  (export "root" (func $root "root"))
)
(assert_return (invoke "root") (u32.const 1111))

;; Forcing promotion at an intermediate level, then a deeper sync call.
(component
  (component $Leaf
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))
    (core module $M
      (import "" "context.get" (func $cget (result i32)))
      (import "" "context.set" (func $cset (param i32)))
      (func (export "leaf'") (param i32) (result i32)
        (if (i32.ne (call $cget) (i32.const 0)) (then unreachable))
        (call $cset (i32.const 0x99aabbcc))
        (if (i32.ne (call $cget) (i32.const 0x99aabbcc)) (then unreachable))
        (i32.add (local.get 0) (i32.const 1))
      )
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "context.get" (func $cget))
      (export "context.set" (func $cset))
    ))))
    (func (export "leaf") (param "x" u32) (result u32)
      (canon lift (core func $m "leaf'")))
  )

  (component $Mid
    (import "leaf" (func $leaf (param "x" u32) (result u32)))
    (core func $leaf' (canon lower (func $leaf)))
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))
    (core func $bpinc (canon backpressure.inc))
    (core func $bpdec (canon backpressure.dec))
    (core module $M
      (import "" "leaf'" (func $leaf' (param i32) (result i32)))
      (import "" "context.get" (func $cget (result i32)))
      (import "" "context.set" (func $cset (param i32)))
      (import "" "backpressure.inc" (func $bpinc))
      (import "" "backpressure.dec" (func $bpdec))
      (func (export "mid'") (param i32) (result i32) (local $r i32)
        (if (i32.ne (call $cget) (i32.const 0)) (then unreachable))
        (call $cset (i32.const 0x55667788))
        ;; Force *before* descending into the leaf.
        (call $bpinc)
        (call $bpdec)
        (if (i32.ne (call $cget) (i32.const 0x55667788)) (then unreachable))
        (local.set $r (call $leaf' (local.get 0)))
        (if (i32.ne (call $cget) (i32.const 0x55667788)) (then unreachable))
        (i32.add (local.get $r) (i32.const 10))
      )
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "leaf'" (func $leaf'))
      (export "context.get" (func $cget))
      (export "context.set" (func $cset))
      (export "backpressure.inc" (func $bpinc))
      (export "backpressure.dec" (func $bpdec))
    ))))
    (func (export "mid") (param "x" u32) (result u32)
      (canon lift (core func $m "mid'")))
  )

  (component $Root
    (import "mid" (func $mid (param "x" u32) (result u32)))
    (core func $mid' (canon lower (func $mid)))
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))
    (core module $M
      (import "" "mid'" (func $mid' (param i32) (result i32)))
      (import "" "context.get" (func $cget (result i32)))
      (import "" "context.set" (func $cset (param i32)))
      (func (export "root'") (result i32) (local $r i32)
        (call $cset (i32.const 0x11223344))
        (local.set $r (call $mid' (i32.const 100)))
        (if (i32.ne (call $cget) (i32.const 0x11223344)) (then unreachable))
        (i32.add (local.get $r) (i32.const 1000))
      )
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "mid'" (func $mid'))
      (export "context.get" (func $cget))
      (export "context.set" (func $cset))
    ))))
    (func (export "root") (result u32)
      (canon lift (core func $m "root'")))
  )

  (instance $leaf (instantiate $Leaf))
  (instance $mid (instantiate $Mid (with "leaf" (func $leaf "leaf"))))
  (instance $root (instantiate $Root (with "mid" (func $mid "mid"))))
  (export "root" (func $root "root"))
)
(assert_return (invoke "root") (u32.const 1111))

;; Repeated sync calls from the same caller.
(component
  (component $A
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))
    (core module $M
      (import "" "context.get" (func $cget (result i32)))
      (import "" "context.set" (func $cset (param i32)))
      (func (export "f'") (param i32) (result i32)
        ;; Each fresh entry must zero the context regardless of prior calls.
        (if (i32.ne (call $cget) (i32.const 0)) (then unreachable))
        (call $cset (i32.add (local.get 0) (i32.const 0x10000)))
        (i32.add (local.get 0) (i32.const 42))
      )
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "context.get" (func $cget))
      (export "context.set" (func $cset))
    ))))
    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f'")))
  )

  (component $B
    (import "f" (func $f (param "x" u32) (result u32)))
    (core func $f' (canon lower (func $f)))
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))
    (core module $N
      (import "" "f'" (func $f' (param i32) (result i32)))
      (import "" "context.get" (func $cget (result i32)))
      (import "" "context.set" (func $cset (param i32)))
      (func (export "g'") (result i32) (local $r i32)
        (call $cset (i32.const 0x1234))
        (local.set $r (call $f' (i32.const 1)))
        (if (i32.ne (call $cget) (i32.const 0x1234)) (then unreachable))
        (local.set $r (i32.add (local.get $r) (call $f' (i32.const 2))))
        ;; Still restored after the second call.
        (if (i32.ne (call $cget) (i32.const 0x1234)) (then unreachable))
        (local.get $r)
      )
    )
    (core instance $n (instantiate $N (with "" (instance
      (export "f'" (func $f'))
      (export "context.get" (func $cget))
      (export "context.set" (func $cset))
    ))))
    (func (export "g") (result u32)
      (canon lift (core func $n "g'")))
  )

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f"))))
  (export "g" (func $b "g"))
)
;; (1 + 42) + (2 + 42) = 87
(assert_return (invoke "g") (u32.const 87))

;; Nested chain forced at *two* levels (re-forcing).
(component
  (component $Leaf
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))
    (core func $bpinc (canon backpressure.inc))
    (core func $bpdec (canon backpressure.dec))
    (core module $M
      (import "" "context.get" (func $cget (result i32)))
      (import "" "context.set" (func $cset (param i32)))
      (import "" "backpressure.inc" (func $bpinc))
      (import "" "backpressure.dec" (func $bpdec))
      (func (export "leaf'") (param i32) (result i32)
        (if (i32.ne (call $cget) (i32.const 0)) (then unreachable))
        (call $cset (i32.const 0x99aabbcc))
        ;; Force promotion.
        (call $bpinc) (call $bpdec)
        (if (i32.ne (call $cget) (i32.const 0x99aabbcc)) (then unreachable))
        (i32.add (local.get 0) (i32.const 1))
      )
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "context.get" (func $cget))
      (export "context.set" (func $cset))
      (export "backpressure.inc" (func $bpinc))
      (export "backpressure.dec" (func $bpdec))
    ))))
    (func (export "leaf") (param "x" u32) (result u32)
      (canon lift (core func $m "leaf'")))
  )

  (component $Mid
    (import "leaf" (func $leaf (param "x" u32) (result u32)))
    (core func $leaf' (canon lower (func $leaf)))
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))
    (core func $bpinc (canon backpressure.inc))
    (core func $bpdec (canon backpressure.dec))
    (core module $M
      (import "" "leaf'" (func $leaf' (param i32) (result i32)))
      (import "" "context.get" (func $cget (result i32)))
      (import "" "context.set" (func $cset (param i32)))
      (import "" "backpressure.inc" (func $bpinc))
      (import "" "backpressure.dec" (func $bpdec))
      (func (export "mid'") (param i32) (result i32) (local $r i32)
        (if (i32.ne (call $cget) (i32.const 0)) (then unreachable))
        (call $cset (i32.const 0x55667788))
        ;; Force promotion.
        (call $bpinc) (call $bpdec)
        (local.set $r (call $leaf' (local.get 0)))
        (if (i32.ne (call $cget) (i32.const 0x55667788)) (then unreachable))
        (i32.add (local.get $r) (i32.const 10))
      )
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "leaf'" (func $leaf'))
      (export "context.get" (func $cget))
      (export "context.set" (func $cset))
      (export "backpressure.inc" (func $bpinc))
      (export "backpressure.dec" (func $bpdec))
    ))))
    (func (export "mid") (param "x" u32) (result u32)
      (canon lift (core func $m "mid'")))
  )

  (component $Root
    (import "mid" (func $mid (param "x" u32) (result u32)))
    (core func $mid' (canon lower (func $mid)))
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))
    (core module $M
      (import "" "mid'" (func $mid' (param i32) (result i32)))
      (import "" "context.get" (func $cget (result i32)))
      (import "" "context.set" (func $cset (param i32)))
      (func (export "root'") (result i32) (local $r i32)
        (call $cset (i32.const 0x11223344))
        (local.set $r (call $mid' (i32.const 100)))
        (if (i32.ne (call $cget) (i32.const 0x11223344)) (then unreachable))
        (i32.add (local.get $r) (i32.const 1000))
      )
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "mid'" (func $mid'))
      (export "context.get" (func $cget))
      (export "context.set" (func $cset))
    ))))
    (func (export "root") (result u32)
      (canon lift (core func $m "root'")))
  )

  (instance $leaf (instantiate $Leaf))
  (instance $mid (instantiate $Mid (with "leaf" (func $leaf "leaf"))))
  (instance $root (instantiate $Root (with "mid" (func $mid "mid"))))
  (export "root" (func $root "root"))
)
(assert_return (invoke "root") (u32.const 1111))

;; The tests below force the slow path with a guest-to-host call instead of
;; backpressure. `host-return-two` is a synchronous host import, so calling it
;; forces the deferred thread.

;; Single guest-to-guest sync call, forced by a host call.
;; - $Outer sets its context and calls $Inner.
;; - $Inner sees a zeroed context, sets its own, then calls the host.
;; - Both contexts survive the forced slow path.
(component
  (import "host-return-two" (func $host (result u32)))

  (component $Inner
    (import "host" (func $host (result u32)))
    (core func $host (canon lower (func $host)))
    (core func $context.get (canon context.get i32 0))
    (core func $context.set (canon context.set i32 0))
    (core module $M
      (import "" "host" (func $host (result i32)))
      (import "" "context.get" (func $context.get (result i32)))
      (import "" "context.set" (func $context.set (param i32)))
      (func (export "f") (param i32) (result i32)
        (if (i32.ne (call $context.get) (i32.const 0)) (then unreachable))
        (call $context.set (i32.const 0x5678))
        (drop (call $host))
        (if (i32.ne (call $context.get) (i32.const 0x5678))
          (then unreachable))
        (i32.add (local.get 0) (i32.const 42))
      )
    )
    (core instance $m (instantiate $M
      (with "" (instance
        (export "host" (func $host))
        (export "context.get" (func $context.get))
        (export "context.set" (func $context.set))
      ))
    ))
    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f")))
  )

  (component $Outer
    (import "f" (func $f (param "x" u32) (result u32)))
    (core func $f (canon lower (func $f)))
    (core func $context.get (canon context.get i32 0))
    (core func $context.set (canon context.set i32 0))
    (core module $M
      (import "" "f" (func $f (param i32) (result i32)))
      (import "" "context.get" (func $context.get (result i32)))
      (import "" "context.set" (func $context.set (param i32)))
      (func (export "g") (result i32) (local $r i32)
        (call $context.set (i32.const 0x1234))
        (local.set $r (call $f (i32.const 1234)))
        (if (i32.ne (call $context.get) (i32.const 0x1234))
          (then unreachable))
        (local.get $r)
      )
    )
    (core instance $m (instantiate $M
      (with "" (instance
        (export "f" (func $f))
        (export "context.get" (func $context.get))
        (export "context.set" (func $context.set))
      ))
    ))
    (func (export "g") (result u32)
      (canon lift (core func $m "g")))
  )

  (instance $inner (instantiate $Inner (with "host" (func $host))))
  (instance $outer (instantiate $Outer (with "f" (func $inner "f"))))
  (export "g" (func $outer "g"))
)
(assert_return (invoke "g") (u32.const 1276))

;; Repeated calls, each forced by a host call.
;; - $Inner stores its argument in its context before calling the host.
;; - A fresh entry must see a zeroed context every time.
(component
  (import "host-return-two" (func $host (result u32)))

  (component $Inner
    (import "host" (func $host (result u32)))
    (core func $host (canon lower (func $host)))
    (core func $context.get (canon context.get i32 0))
    (core func $context.set (canon context.set i32 0))
    (core module $M
      (import "" "host" (func $host (result i32)))
      (import "" "context.get" (func $context.get (result i32)))
      (import "" "context.set" (func $context.set (param i32)))
      (func (export "f") (param i32) (result i32)
        (if (i32.ne (call $context.get) (i32.const 0)) (then unreachable))
        (call $context.set (local.get 0))
        (drop (call $host))
        (if (i32.ne (call $context.get) (local.get 0)) (then unreachable))
        (i32.add (local.get 0) (i32.const 42))
      )
    )
    (core instance $m (instantiate $M
      (with "" (instance
        (export "host" (func $host))
        (export "context.get" (func $context.get))
        (export "context.set" (func $context.set))
      ))
    ))
    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f")))
  )

  (component $Outer
    (import "f" (func $f (param "x" u32) (result u32)))
    (core func $f (canon lower (func $f)))
    (core func $context.get (canon context.get i32 0))
    (core func $context.set (canon context.set i32 0))
    (core module $M
      (import "" "f" (func $f (param i32) (result i32)))
      (import "" "context.get" (func $context.get (result i32)))
      (import "" "context.set" (func $context.set (param i32)))
      (func (export "g") (param i32) (result i32) (local $r i32)
        (call $context.set (i32.const 0x4321))
        (local.set $r (call $f (local.get 0)))
        (if (i32.ne (call $context.get) (i32.const 0x4321))
          (then unreachable))
        (local.get $r)
      )
    )
    (core instance $m (instantiate $M
      (with "" (instance
        (export "f" (func $f))
        (export "context.get" (func $context.get))
        (export "context.set" (func $context.set))
      ))
    ))
    (func (export "g") (param "x" u32) (result u32)
      (canon lift (core func $m "g")))
  )

  (instance $inner (instantiate $Inner (with "host" (func $host))))
  (instance $outer (instantiate $Outer (with "f" (func $inner "f"))))
  (export "g" (func $outer "g"))
)
(assert_return (invoke "g" (u32.const 7)) (u32.const 49))
(assert_return (invoke "g" (u32.const 100)) (u32.const 142))
(assert_return (invoke "g" (u32.const 0x10000)) (u32.const 0x1002a))
(assert_return (invoke "g" (u32.const 1)) (u32.const 43))

;; Nested $Root -> $Mid -> $Leaf chain, forced by a host call in $Leaf.
;; - Each level sets its own context before calling the next.
;; - Every level's context survives the force.
(component
  (import "host-return-two" (func $host (result u32)))

  (component $Leaf
    (import "host" (func $host (result u32)))
    (core func $host (canon lower (func $host)))
    (core func $context.get (canon context.get i32 0))
    (core func $context.set (canon context.set i32 0))
    (core module $M
      (import "" "host" (func $host (result i32)))
      (import "" "context.get" (func $context.get (result i32)))
      (import "" "context.set" (func $context.set (param i32)))
      (func (export "leaf") (param i32) (result i32)
        (if (i32.ne (call $context.get) (i32.const 0)) (then unreachable))
        (call $context.set (i32.const 0x0c0ffee0))
        (drop (call $host))
        (if (i32.ne (call $context.get) (i32.const 0x0c0ffee0))
          (then unreachable))
        (i32.add (local.get 0) (i32.const 1))
      )
    )
    (core instance $m (instantiate $M
      (with "" (instance
        (export "host" (func $host))
        (export "context.get" (func $context.get))
        (export "context.set" (func $context.set))
      ))
    ))
    (func (export "leaf") (param "x" u32) (result u32)
      (canon lift (core func $m "leaf")))
  )

  (component $Mid
    (import "leaf" (func $leaf (param "x" u32) (result u32)))
    (core func $leaf (canon lower (func $leaf)))
    (core func $context.get (canon context.get i32 0))
    (core func $context.set (canon context.set i32 0))
    (core module $M
      (import "" "leaf" (func $leaf (param i32) (result i32)))
      (import "" "context.get" (func $context.get (result i32)))
      (import "" "context.set" (func $context.set (param i32)))
      (func (export "mid") (param i32) (result i32) (local $r i32)
        (if (i32.ne (call $context.get) (i32.const 0)) (then unreachable))
        (call $context.set (i32.const 0x0d00d100))
        (local.set $r (call $leaf (local.get 0)))
        (if (i32.ne (call $context.get) (i32.const 0x0d00d100))
          (then unreachable))
        (i32.add (local.get $r) (i32.const 10))
      )
    )
    (core instance $m (instantiate $M
      (with "" (instance
        (export "leaf" (func $leaf))
        (export "context.get" (func $context.get))
        (export "context.set" (func $context.set))
      ))
    ))
    (func (export "mid") (param "x" u32) (result u32)
      (canon lift (core func $m "mid")))
  )

  (component $Root
    (import "mid" (func $mid (param "x" u32) (result u32)))
    (core func $mid (canon lower (func $mid)))
    (core func $context.get (canon context.get i32 0))
    (core func $context.set (canon context.set i32 0))
    (core module $M
      (import "" "mid" (func $mid (param i32) (result i32)))
      (import "" "context.get" (func $context.get (result i32)))
      (import "" "context.set" (func $context.set (param i32)))
      (func (export "root") (result i32) (local $r i32)
        (call $context.set (i32.const 0x0badf00d))
        (local.set $r (call $mid (i32.const 100)))
        (if (i32.ne (call $context.get) (i32.const 0x0badf00d))
          (then unreachable))
        (i32.add (local.get $r) (i32.const 1000))
      )
    )
    (core instance $m (instantiate $M
      (with "" (instance
        (export "mid" (func $mid))
        (export "context.get" (func $context.get))
        (export "context.set" (func $context.set))
      ))
    ))
    (func (export "root") (result u32)
      (canon lift (core func $m "root")))
  )

  (instance $leaf (instantiate $Leaf (with "host" (func $host))))
  (instance $mid (instantiate $Mid (with "leaf" (func $leaf "leaf"))))
  (instance $root (instantiate $Root (with "mid" (func $mid "mid"))))
  (export "root" (func $root "root"))
)
(assert_return (invoke "root") (u32.const 1111))

;; A thread-transparent frame nested inside an opaque one.
;; - $Leaf touches no thread state, so the $Mid -> $Leaf adapter pushes no
;;   frame.
;; - $Mid calls $Leaf, and only then calls the host to force its thread.
;; - Repeated calls would expose any state the omitted frame failed to clean
;;   up.
(component
  (import "host-return-two" (func $host (result u32)))

  (component $Leaf
    (core module $M
      (func (export "leaf") (param i32) (result i32)
        (i32.add (local.get 0) (i32.const 1))
      )
    )
    (core instance $m (instantiate $M))
    (func (export "leaf") (param "x" u32) (result u32)
      (canon lift (core func $m "leaf")))
  )

  (component $Mid
    (import "leaf" (func $leaf (param "x" u32) (result u32)))
    (import "host" (func $host (result u32)))
    (core func $leaf (canon lower (func $leaf)))
    (core func $host (canon lower (func $host)))
    (core func $context.get (canon context.get i32 0))
    (core func $context.set (canon context.set i32 0))
    (core module $M
      (import "" "leaf" (func $leaf (param i32) (result i32)))
      (import "" "host" (func $host (result i32)))
      (import "" "context.get" (func $context.get (result i32)))
      (import "" "context.set" (func $context.set (param i32)))
      (func (export "mid") (param i32) (result i32) (local $r i32)
        (if (i32.ne (call $context.get) (i32.const 0)) (then unreachable))
        (call $context.set (i32.const 0x0d00d100))
        (local.set $r (call $leaf (local.get 0)))
        (drop (call $host))
        (if (i32.ne (call $context.get) (i32.const 0x0d00d100))
          (then unreachable))
        (i32.add (local.get $r) (i32.const 10))
      )
    )
    (core instance $m (instantiate $M
      (with "" (instance
        (export "leaf" (func $leaf))
        (export "host" (func $host))
        (export "context.get" (func $context.get))
        (export "context.set" (func $context.set))
      ))
    ))
    (func (export "mid") (param "x" u32) (result u32)
      (canon lift (core func $m "mid")))
  )

  (component $Root
    (import "mid" (func $mid (param "x" u32) (result u32)))
    (core func $mid (canon lower (func $mid)))
    (core func $context.get (canon context.get i32 0))
    (core func $context.set (canon context.set i32 0))
    (core module $M
      (import "" "mid" (func $mid (param i32) (result i32)))
      (import "" "context.get" (func $context.get (result i32)))
      (import "" "context.set" (func $context.set (param i32)))
      (func (export "root") (result i32) (local $r i32)
        (call $context.set (i32.const 0x0badf00d))
        (local.set $r (call $mid (i32.const 100)))
        (if (i32.ne (call $context.get) (i32.const 0x0badf00d))
          (then unreachable))
        (i32.add (local.get $r) (i32.const 1000))
      )
    )
    (core instance $m (instantiate $M
      (with "" (instance
        (export "mid" (func $mid))
        (export "context.get" (func $context.get))
        (export "context.set" (func $context.set))
      ))
    ))
    (func (export "root") (result u32)
      (canon lift (core func $m "root")))
  )

  (instance $leaf (instantiate $Leaf))
  (instance $mid (instantiate $Mid
    (with "leaf" (func $leaf "leaf"))
    (with "host" (func $host))
  ))
  (instance $root (instantiate $Root (with "mid" (func $mid "mid"))))
  (export "root" (func $root "root"))
)
(assert_return (invoke "root") (u32.const 1111))
(assert_return (invoke "root") (u32.const 1111))
(assert_return (invoke "root") (u32.const 1111))

;; An opaque frame nested inside a thread-transparent one.
;; - $Mid touches no thread state, so the $Root -> $Mid adapter is
;;   thread-transparent.
;; - $Leaf sees a zeroed context, sets its own, then calls the host.
;; - Neither $Mid nor $Leaf disturbs $Root's context.
(component
  (import "host-return-two" (func $host (result u32)))

  (component $Leaf
    (import "host" (func $host (result u32)))
    (core func $host (canon lower (func $host)))
    (core func $context.get (canon context.get i32 0))
    (core func $context.set (canon context.set i32 0))
    (core module $M
      (import "" "host" (func $host (result i32)))
      (import "" "context.get" (func $context.get (result i32)))
      (import "" "context.set" (func $context.set (param i32)))
      (func (export "leaf") (param i32) (result i32)
        (if (i32.ne (call $context.get) (i32.const 0)) (then unreachable))
        (call $context.set (i32.const 0x0feed000))
        (drop (call $host))
        (if (i32.ne (call $context.get) (i32.const 0x0feed000))
          (then unreachable))
        (i32.add (local.get 0) (i32.const 1))
      )
    )
    (core instance $m (instantiate $M
      (with "" (instance
        (export "host" (func $host))
        (export "context.get" (func $context.get))
        (export "context.set" (func $context.set))
      ))
    ))
    (func (export "leaf") (param "x" u32) (result u32)
      (canon lift (core func $m "leaf")))
  )

  (component $Mid
    (import "leaf" (func $leaf (param "x" u32) (result u32)))
    (core func $leaf (canon lower (func $leaf)))
    (core module $M
      (import "" "leaf" (func $leaf (param i32) (result i32)))
      (func (export "mid") (param i32) (result i32)
        (i32.add (call $leaf (local.get 0)) (i32.const 10))
      )
    )
    (core instance $m (instantiate $M
      (with "" (instance
        (export "leaf" (func $leaf))
      ))
    ))
    (func (export "mid") (param "x" u32) (result u32)
      (canon lift (core func $m "mid")))
  )

  (component $Root
    (import "mid" (func $mid (param "x" u32) (result u32)))
    (core func $mid (canon lower (func $mid)))
    (core func $context.get (canon context.get i32 0))
    (core func $context.set (canon context.set i32 0))
    (core module $M
      (import "" "mid" (func $mid (param i32) (result i32)))
      (import "" "context.get" (func $context.get (result i32)))
      (import "" "context.set" (func $context.set (param i32)))
      (func (export "root") (result i32) (local $r i32)
        (call $context.set (i32.const 0x0badf00d))
        (local.set $r (call $mid (i32.const 100)))
        (if (i32.ne (call $context.get) (i32.const 0x0badf00d))
          (then unreachable))
        (i32.add (local.get $r) (i32.const 1000))
      )
    )
    (core instance $m (instantiate $M
      (with "" (instance
        (export "mid" (func $mid))
        (export "context.get" (func $context.get))
        (export "context.set" (func $context.set))
      ))
    ))
    (func (export "root") (result u32)
      (canon lift (core func $m "root")))
  )

  (instance $leaf (instantiate $Leaf (with "host" (func $host))))
  (instance $mid (instantiate $Mid (with "leaf" (func $leaf "leaf"))))
  (instance $root (instantiate $Root (with "mid" (func $mid "mid"))))
  (export "root" (func $root "root"))
)
(assert_return (invoke "root") (u32.const 1111))
(assert_return (invoke "root") (u32.const 1111))
(assert_return (invoke "root") (u32.const 1111))

;; A table shared between core instances must not let a callee reach the
;; caller's context through a thread-transparent adapter.
;; - $Inner's lifted `f` calls through a table that $Evil filled, during
;;   instantiation, with a function that uses `context.{get,set}`.
;; - $Outer sets its context, then calls `f`.
;; - The callee must see a zeroed context, and $Outer's must be intact.

;; The table is defined by a third module that imports nothing.
(component
  (component $Inner
    (core func $context.get (canon context.get i32 0))
    (core func $context.set (canon context.set i32 0))

    (core module $Shared (table (export "t") 1 funcref))
    (core instance $shared (instantiate $Shared))

    ;; Reads the caller's slot, clobbers it, and returns what it read.
    (core module $Evil
      (import "" "t" (table 1 funcref))
      (import "" "context.get" (func $context.get (result i32)))
      (import "" "context.set" (func $context.set (param i32)))
      (func $leak (result i32) (local $t i32)
        (local.set $t (call $context.get))
        (call $context.set (i32.const 0x9999))
        (local.get $t)
      )
      (elem (table 0) (i32.const 0) func $leak)
    )
    (core instance $evil (instantiate $Evil
      (with "" (instance
        (export "t" (table $shared "t"))
        (export "context.get" (func $context.get))
        (export "context.set" (func $context.set))
      ))
    ))

    (core module $Victim
      (import "" "t" (table 1 funcref))
      (type $sig (func (result i32)))
      (func (export "f") (result i32)
        (call_indirect (type $sig) (i32.const 0))
      )
    )
    (core instance $victim (instantiate $Victim
      (with "" (instance
        (export "t" (table $shared "t"))
      ))
    ))

    (func (export "f") (result u32) (canon lift (core func $victim "f")))
  )

  (component $Outer
    (import "f" (func $f (result u32)))
    (core func $f (canon lower (func $f)))
    (core func $context.get (canon context.get i32 0))
    (core func $context.set (canon context.set i32 0))
    (core module $M
      (import "" "f" (func $f (result i32)))
      (import "" "context.get" (func $context.get (result i32)))
      (import "" "context.set" (func $context.set (param i32)))
      ;; Returns what the callee saw in our slot.
      (func (export "g") (result i32)
        (call $context.set (i32.const 0x1234))
        (call $f)
      )
      ;; Returns our slot after the call.
      (func (export "h") (result i32)
        (call $context.set (i32.const 0x1234))
        (drop (call $f))
        (call $context.get)
      )
    )
    (core instance $m (instantiate $M
      (with "" (instance
        (export "f" (func $f))
        (export "context.get" (func $context.get))
        (export "context.set" (func $context.set))
      ))
    ))
    (func (export "g") (result u32) (canon lift (core func $m "g")))
    (func (export "h") (result u32) (canon lift (core func $m "h")))
  )

  (instance $inner (instantiate $Inner))
  (instance $outer (instantiate $Outer (with "f" (func $inner "f"))))
  (export "g" (func $outer "g"))
  (export "h" (func $outer "h"))
)
(assert_return (invoke "g") (u32.const 0))
(assert_return (invoke "h") (u32.const 0x1234))

;; The table is defined and exported by $Victim, which imports nothing.
(component
  (component $Inner
    (core func $context.get (canon context.get i32 0))
    (core func $context.set (canon context.set i32 0))

    (core module $Victim
      (table (export "t") 1 funcref)
      (type $sig (func (result i32)))
      (func (export "f") (result i32)
        (call_indirect (type $sig) (i32.const 0))
      )
    )
    (core instance $victim (instantiate $Victim))

    ;; Reads the caller's slot, clobbers it, and returns what it read.
    (core module $Evil
      (import "" "t" (table 1 funcref))
      (import "" "context.get" (func $context.get (result i32)))
      (import "" "context.set" (func $context.set (param i32)))
      (func $leak (result i32) (local $t i32)
        (local.set $t (call $context.get))
        (call $context.set (i32.const 0x9999))
        (local.get $t)
      )
      (elem (table 0) (i32.const 0) func $leak)
    )
    (core instance $evil (instantiate $Evil
      (with "" (instance
        (export "t" (table $victim "t"))
        (export "context.get" (func $context.get))
        (export "context.set" (func $context.set))
      ))
    ))

    (func (export "f") (result u32) (canon lift (core func $victim "f")))
  )

  (component $Outer
    (import "f" (func $f (result u32)))
    (core func $f (canon lower (func $f)))
    (core func $context.get (canon context.get i32 0))
    (core func $context.set (canon context.set i32 0))
    (core module $M
      (import "" "f" (func $f (result i32)))
      (import "" "context.get" (func $context.get (result i32)))
      (import "" "context.set" (func $context.set (param i32)))
      ;; Returns what the callee saw in our slot.
      (func (export "g") (result i32)
        (call $context.set (i32.const 0x1234))
        (call $f)
      )
      ;; Returns our slot after the call.
      (func (export "h") (result i32)
        (call $context.set (i32.const 0x1234))
        (drop (call $f))
        (call $context.get)
      )
    )
    (core instance $m (instantiate $M
      (with "" (instance
        (export "f" (func $f))
        (export "context.get" (func $context.get))
        (export "context.set" (func $context.set))
      ))
    ))
    (func (export "g") (result u32) (canon lift (core func $m "g")))
    (func (export "h") (result u32) (canon lift (core func $m "h")))
  )

  (instance $inner (instantiate $Inner))
  (instance $outer (instantiate $Outer (with "f" (func $inner "f"))))
  (export "g" (func $outer "g"))
  (export "h" (func $outer "h"))
)
(assert_return (invoke "g") (u32.const 0))
(assert_return (invoke "h") (u32.const 0x1234))

;; Control: $Victim also imports `context.get` directly.
(component
  (component $Inner
    (core func $context.get (canon context.get i32 0))
    (core func $context.set (canon context.set i32 0))

    (core module $Shared (table (export "t") 1 funcref))
    (core instance $shared (instantiate $Shared))

    ;; Reads the caller's slot, clobbers it, and returns what it read.
    (core module $Evil
      (import "" "t" (table 1 funcref))
      (import "" "context.get" (func $context.get (result i32)))
      (import "" "context.set" (func $context.set (param i32)))
      (func $leak (result i32) (local $t i32)
        (local.set $t (call $context.get))
        (call $context.set (i32.const 0x9999))
        (local.get $t)
      )
      (elem (table 0) (i32.const 0) func $leak)
    )
    (core instance $evil (instantiate $Evil
      (with "" (instance
        (export "t" (table $shared "t"))
        (export "context.get" (func $context.get))
        (export "context.set" (func $context.set))
      ))
    ))

    (core module $Victim
      (import "" "t" (table 1 funcref))
      (import "" "context.get" (func (result i32)))
      (type $sig (func (result i32)))
      (func (export "f") (result i32)
        (call_indirect (type $sig) (i32.const 0))
      )
    )
    (core instance $victim (instantiate $Victim
      (with "" (instance
        (export "t" (table $shared "t"))
        (export "context.get" (func $context.get))
      ))
    ))

    (func (export "f") (result u32) (canon lift (core func $victim "f")))
  )

  (component $Outer
    (import "f" (func $f (result u32)))
    (core func $f (canon lower (func $f)))
    (core func $context.get (canon context.get i32 0))
    (core func $context.set (canon context.set i32 0))
    (core module $M
      (import "" "f" (func $f (result i32)))
      (import "" "context.get" (func $context.get (result i32)))
      (import "" "context.set" (func $context.set (param i32)))
      ;; Returns what the callee saw in our slot.
      (func (export "g") (result i32)
        (call $context.set (i32.const 0x1234))
        (call $f)
      )
      ;; Returns our slot after the call.
      (func (export "h") (result i32)
        (call $context.set (i32.const 0x1234))
        (drop (call $f))
        (call $context.get)
      )
    )
    (core instance $m (instantiate $M
      (with "" (instance
        (export "f" (func $f))
        (export "context.get" (func $context.get))
        (export "context.set" (func $context.set))
      ))
    ))
    (func (export "g") (result u32) (canon lift (core func $m "g")))
    (func (export "h") (result u32) (canon lift (core func $m "h")))
  )

  (instance $inner (instantiate $Inner))
  (instance $outer (instantiate $Outer (with "f" (func $inner "f"))))
  (export "g" (func $outer "g"))
  (export "h" (func $outer "h"))
)
(assert_return (invoke "g") (u32.const 0))
(assert_return (invoke "h") (u32.const 0x1234))
