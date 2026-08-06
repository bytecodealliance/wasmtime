;;! component_model_async = true
;;! multi_memory = true

;; Guest-to-guest sync calls that are thread transparent, so they drop their
;; `enter-sync-call`/`exit-sync-call` window.
;;
;; A transparent adapter is only correct if the caller cannot tell, so every
;; case below has the caller drive a `context` slot across the call and check
;; that it is still intact afterwards.

;; A plain `u32 -> u32` adapter whose callee imports nothing.
(component
  (component $A
    (core module $M
      (func (export "f") (param i32) (result i32)
        (i32.add (local.get 0) (i32.const 42)))
    )
    (core instance $m (instantiate $M))
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

;; Aggregates stay transparent; only handles disqualify a signature.
(component
  (component $A
    (core module $M
      ;; `(option (tuple u32 u32))` flattens to a discriminant plus two fields.
      (func (export "f") (param i32 i32 i32) (result i32)
        (if (i32.eqz (local.get 0)) (then unreachable))
        (i32.add (local.get 1) (local.get 2)))
    )
    (core instance $m (instantiate $M))
    (func (export "f") (param "x" (option (tuple u32 u32))) (result u32)
      (canon lift (core func $m "f")))
  )

  (component $B
    (import "f" (func $f (param "x" (option (tuple u32 u32))) (result u32)))
    (core func $f' (canon lower (func $f)))
    (core func $get (canon context.get i32 0))
    (core func $set (canon context.set i32 0))
    (core module $N
      (import "" "f" (func $f (param i32 i32 i32) (result i32)))
      (import "" "get" (func $get (result i32)))
      (import "" "set" (func $set (param i32)))
      (func (export "g") (result i32) (local $r i32)
        (call $set (i32.const 0x1234abcd))
        (local.set $r
          (call $f (i32.const 1) (i32.const 20) (i32.const 22)))
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
(assert_return (invoke "g") (u32.const 42))

;; A `(list u8)` argument, so the adapter copies through both memories and
;; calls the callee's `realloc`, all still within a transparent adapter.
(component
  (component $A
    (core module $M
      (memory (export "memory") 1)
      (global $next (mut i32) (i32.const 16))
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        (local $ret i32)
        (local.set $ret (global.get $next))
        (global.set $next (i32.add (global.get $next) (local.get 3)))
        (local.get $ret))
      ;; Sum the bytes the adapter copied in for us.
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
    (core instance $m (instantiate $M))
    (func (export "f") (param "x" (list u8)) (result u32)
      (canon lift (core func $m "f")
        (memory $m "memory")
        (realloc (func $m "realloc"))))
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

;; An unrelated *sibling* component instance declaring a disqualifying canon
;; does not taint anyone else: the $B -> $A adapter is still transparent.
(component
  (component $Dirty
    (core func $get (canon context.get i32 0))
    (core module $M (import "" "get" (func (result i32))))
    (core instance $m (instantiate $M
      (with "" (instance (export "get" (func $get))))))
  )

  (component $A
    (core module $M
      (func (export "f") (param i32) (result i32)
        (i32.add (local.get 0) (i32.const 42)))
    )
    (core instance $m (instantiate $M))
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

  (instance $dirty (instantiate $Dirty))
  (instance $a (instantiate $A))
  (instance $b (instantiate $B (with "f" (func $a "f"))))
  (export "g" (func $b "g"))
)
(assert_return (invoke "g") (u32.const 43))

;; A three-deep chain $Outer -> $Mid -> $Inner where every link is clean, so
;; both adapters are transparent and nest inside one another.
(component
  (component $Inner
    (core module $M
      (func (export "f") (param i32) (result i32)
        (i32.add (local.get 0) (i32.const 2)))
    )
    (core instance $m (instantiate $M))
    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f")))
  )

  (component $Mid
    (import "f" (func $f (param "x" u32) (result u32)))
    (core func $f' (canon lower (func $f)))
    (core module $M
      (import "" "f" (func $f (param i32) (result i32)))
      (func (export "f") (param i32) (result i32)
        (i32.add (call $f (local.get 0)) (i32.const 20)))
    )
    (core instance $m (instantiate $M
      (with "" (instance (export "f" (func $f'))))))
    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f")))
  )

  (component $Outer
    (import "f" (func $f (param "x" u32) (result u32)))
    (core func $f' (canon lower (func $f)))
    (core func $get (canon context.get i32 0))
    (core func $set (canon context.set i32 0))
    (core module $M
      (import "" "f" (func $f (param i32) (result i32)))
      (import "" "get" (func $get (result i32)))
      (import "" "set" (func $set (param i32)))
      (func (export "g") (result i32) (local $r i32)
        (call $set (i32.const 0x1234abcd))
        (local.set $r (call $f (i32.const 20)))
        (if (i32.ne (call $get) (i32.const 0x1234abcd)) (then unreachable))
        (local.get $r))
    )
    (core instance $m (instantiate $M (with "" (instance
      (export "f" (func $f'))
      (export "get" (func $get))
      (export "set" (func $set))))))
    (func (export "g") (result u32) (canon lift (core func $m "g")))
  )

  (instance $inner (instantiate $Inner))
  (instance $mid (instantiate $Mid (with "f" (func $inner "f"))))
  (instance $outer (instantiate $Outer (with "f" (func $mid "f"))))
  (export "g" (func $outer "g"))
)
(assert_return (invoke "g") (u32.const 42))
