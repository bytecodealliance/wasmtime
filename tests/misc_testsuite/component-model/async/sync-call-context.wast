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
