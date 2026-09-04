;;! component_model_async = true
;;! multi_memory = true

;; Guest-to-guest sync calls whose adapters the thread-transparency analysis
;; classifies as opaque, so they keep their `enter-sync-call`/`exit-sync-call`
;; calls.
;;
;; Wherever the disqualifying canon can actually run, the callee writes a
;; `context` slot and the caller checks that its own slot survived, which would
;; break if one of these cases was misclassified as transparent.

;; The callee's core module imports `canon context.get`/`context.set`.
(component
  (component $A
    (core func $get (canon context.get i32 0))
    (core func $set (canon context.set i32 0))
    (core module $M
      (import "" "get" (func $get (result i32)))
      (import "" "set" (func $set (param i32)))
      (func (export "f") (param i32) (result i32)
        ;; A fresh task, so a fresh, zeroed slot.
        (if (call $get) (then unreachable))
        (call $set (i32.const 0x5555ffff))
        (if (i32.ne (call $get) (i32.const 0x5555ffff)) (then unreachable))
        (i32.add (local.get 0) (i32.const 42)))
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "get" (func $get))
      (export "set" (func $set))))))
    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f")))
  )

  (component $B
    (import "f" (func $f (param "x" u32) (result u32)))
    (core func $f' (canon lower (func $f)))
    (core func $get (canon context.get i32 0))
    (core func $set (canon context.set i32 0))
    (core module $N
      (import "" "f" (func $f (param i32) (result i32)))
      (import "" "get" (func $get (result i32)))
      (import "" "set" (func $set (param i32)))
      (func (export "g") (result i32) (local $r i32)
        (call $set (i32.const 0x1234abcd))
        (local.set $r (call $f (i32.const 1)))
        ;; The callee clobbered slot 0; the window must have restored ours.
        (if (i32.ne (call $get) (i32.const 0x1234abcd)) (then unreachable))
        (local.get $r))
    )
    (core instance $n (instantiate $N (with "" (instance
      (export "f" (func $f'))
      (export "get" (func $get))
      (export "set" (func $set))))))
    (func (export "g") (result u32) (canon lift (core func $n "g")))
  )

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f"))))
  (export "g" (func $b "g"))
)
(assert_return (invoke "g") (u32.const 43))

;; The disqualifying canon need not be reachable from the callee: here only the
;; `realloc` helper touches `context`, but it shares the callee's component
;; instance, which is the granularity the analysis works at.
(component
  (component $A
    (core func $set (canon context.set i32 0))
    (core module $Helpers
      (import "" "set" (func $set (param i32)))
      (memory (export "memory") 1)
      (global $next (mut i32) (i32.const 16))
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        (local $ret i32)
        (call $set (i32.const 0x5555ffff))
        (local.set $ret (global.get $next))
        (global.set $next (i32.add (global.get $next) (local.get 3)))
        (local.get $ret))
    )
    (core instance $helpers (instantiate $Helpers
      (with "" (instance (export "set" (func $set))))))
    (core module $M
      (import "" "memory" (memory 1))
      (func (export "f") (param $ptr i32) (param $len i32) (result i32)
        (local $sum i32)
        (loop $l
          (if (local.get $len) (then
            (local.set $sum
              (i32.add (local.get $sum) (i32.load8_u (local.get $ptr))))
            (local.set $ptr (i32.add (local.get $ptr) (i32.const 1)))
            (local.set $len (i32.sub (local.get $len) (i32.const 1)))
            (br $l))))
        (local.get $sum))
    )
    (core instance $m (instantiate $M
      (with "" (instance (export "memory" (memory $helpers "memory"))))))
    (func (export "f") (param "x" (list u8)) (result u32)
      (canon lift (core func $m "f")
        (memory $helpers "memory")
        (realloc (func $helpers "realloc"))))
  )

  (component $B
    (import "f" (func $f (param "x" (list u8)) (result u32)))
    (core func $get (canon context.get i32 0))
    (core func $set (canon context.set i32 0))
    (core module $Helpers
      (memory (export "memory") 1)
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        unreachable)
      (data (i32.const 8) "\01\02\03\04")
    )
    (core instance $helpers (instantiate $Helpers))
    (core func $f' (canon lower (func $f)
      (memory $helpers "memory")
      (realloc (func $helpers "realloc"))))
    (core module $N
      (import "" "f" (func $f (param i32 i32) (result i32)))
      (import "" "get" (func $get (result i32)))
      (import "" "set" (func $set (param i32)))
      (func (export "g") (result i32) (local $r i32)
        (call $set (i32.const 0x1234abcd))
        (local.set $r (call $f (i32.const 8) (i32.const 4)))
        (if (i32.ne (call $get) (i32.const 0x1234abcd)) (then unreachable))
        (local.get $r))
    )
    (core instance $n (instantiate $N (with "" (instance
      (export "f" (func $f'))
      (export "get" (func $get))
      (export "set" (func $set))))))
    (func (export "g") (result u32) (canon lift (core func $n "g")))
  )

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f"))))
  (export "g" (func $b "g"))
)
(assert_return (invoke "g") (u32.const 10))

;; Importing any other canonical built-in (here `resource.{new,drop}`) is
;; disqualifying. The resource's destructor writes `context`, so this one is
;; observable as well.
(component
  (component $A
    (core func $set (canon context.set i32 0))
    (core module $Dtor
      (import "" "set" (func $set (param i32)))
      (global $drops (mut i32) (i32.const 0))
      (func (export "dtor") (param i32)
        (call $set (i32.const 0x5555ffff))
        (global.set $drops (i32.add (global.get $drops) (i32.const 1))))
      (func (export "drops") (result i32) global.get $drops)
    )
    (core instance $dtor (instantiate $Dtor
      (with "" (instance (export "set" (func $set))))))
    (type $t (resource (rep i32) (dtor (core func $dtor "dtor"))))
    (core func $new (canon resource.new $t))
    (core func $drop (canon resource.drop $t))
    (core module $M
      (import "" "new" (func $new (param i32) (result i32)))
      (import "" "drop" (func $drop (param i32)))
      (import "" "drops" (func $drops (result i32)))
      (func (export "f") (param i32) (result i32)
        (call $drop (call $new (local.get 0)))
        ;; One destructor ran.
        (if (i32.ne (call $drops) (i32.const 1)) (then unreachable))
        (i32.add (local.get 0) (i32.const 42)))
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "new" (func $new))
      (export "drop" (func $drop))
      (export "drops" (func $dtor "drops"))))))
    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f")))
  )

  (component $B
    (import "f" (func $f (param "x" u32) (result u32)))
    (core func $f' (canon lower (func $f)))
    (core func $get (canon context.get i32 0))
    (core func $set (canon context.set i32 0))
    (core module $N
      (import "" "f" (func $f (param i32) (result i32)))
      (import "" "get" (func $get (result i32)))
      (import "" "set" (func $set (param i32)))
      (func (export "g") (result i32) (local $r i32)
        (call $set (i32.const 0x1234abcd))
        (local.set $r (call $f (i32.const 1)))
        (if (i32.ne (call $get) (i32.const 0x1234abcd)) (then unreachable))
        (local.get $r))
    )
    (core instance $n (instantiate $N (with "" (instance
      (export "f" (func $f'))
      (export "get" (func $get))
      (export "set" (func $set))))))
    (func (export "g") (result u32) (canon lift (core func $n "g")))
  )

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f"))))
  (export "g" (func $b "g"))
)
(assert_return (invoke "g") (u32.const 43))

;; An `async` function *type* is disqualifying even when the function is both
;; lifted and lowered synchronously: a sync-typed call into an `async`-typed
;; callee may block, and if it does the scheduler has to be able to find the
;; sync-typed call in progress on this instance.
;;
;; The callee here never blocks, so the call itself is allowed -- blocking is
;; only enforced if and when it actually happens -- and the caller-side
;; `context` assertions do run, checking that the window this adapter kept
;; restored $B's slot.
(component
  (component $A
    (core module $M
      (func (export "f") (param i32) (result i32)
        (i32.add (local.get 0) (i32.const 42)))
    )
    (core instance $m (instantiate $M))
    (func (export "f") async (param "x" u32) (result u32)
      (canon lift (core func $m "f")))
  )

  (component $B
    (import "f" (func $f async (param "x" u32) (result u32)))
    (core func $f' (canon lower (func $f)))
    (core func $get (canon context.get i32 0))
    (core func $set (canon context.set i32 0))
    (core module $N
      (import "" "f" (func $f (param i32) (result i32)))
      (import "" "get" (func $get (result i32)))
      (import "" "set" (func $set (param i32)))
      (func (export "g") (result i32) (local $r i32)
        (call $set (i32.const 0x1234abcd))
        (local.set $r (call $f (i32.const 1)))
        (if (i32.ne (call $get) (i32.const 0x1234abcd)) (then unreachable))
        (local.get $r))
    )
    (core instance $n (instantiate $N (with "" (instance
      (export "f" (func $f'))
      (export "get" (func $get))
      (export "set" (func $set))))))
    (func (export "g") (result u32) (canon lift (core func $n "g")))
  )

  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f"))))
  (export "g" (func $b "g"))
)
(assert_return (invoke "g") (u32.const 43))
