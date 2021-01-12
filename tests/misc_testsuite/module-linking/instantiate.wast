(module
  (module)
  (instance $a (instantiate 0))
)

(module $a
  (global (export "global") (mut i32) (i32.const 0))

  (func (export "reset")
    i32.const 0
    global.set 0)

  (func $set (export "inc")
    i32.const 1
    global.get 0
    i32.add
    global.set 0)

  (func (export "get") (result i32)
    global.get 0)

  (func (export "load") (result i32)
    i32.const 0
    i32.load)

  (memory (export "memory") 1)
  (table (export "table") 1 funcref)
  (elem (i32.const 0) $set)
)

;; Imported functions work
(module
  (import "a" "inc" (func $set))
  (module
    (import "" (func))
    (start 0))
  (instance $a (instantiate 0 "" (func $set)))
)

(assert_return (invoke $a "get") (i32.const 1))

;; Imported globals work
(module
  (import "a" "global" (global $g (mut i32)))
  (module
    (import "" (global (mut i32)))
    (func
      i32.const 2
      global.set 0)
    (start 0))

  (instance $a (instantiate 0 "" (global $g)))
)
(assert_return (invoke $a "get") (i32.const 2))

;; Imported tables work
(module
  (import "a" "table" (table $t 1 funcref))
  (module
    (import "" (table 1 funcref))
    (func
      i32.const 0
      call_indirect)
    (start 0))

  (instance $a (instantiate 0 "" (table $t)))
)
(assert_return (invoke $a "get") (i32.const 3))

;; Imported memories work
(module
  (import "a" "memory" (memory $m 1))
  (module
    (import "" (memory 1))
    (func
      i32.const 0
      i32.const 100
      i32.store)
    (start 0))

  (instance $a (instantiate 0 "" (memory $m)))
)
(assert_return (invoke $a "load") (i32.const 100))

;; Imported instances work
(module
  (import "a" "inc" (func $set))

  (module $m1
    (import "" (instance (export "" (func))))
    (alias (func 0 ""))
    (start 0))

  (module $m2
    (func (export "") (import "")))
  (instance $i (instantiate $m2 "" (func $set)))
  (instance (instantiate $m1 "" (instance $i)))
)
(assert_return (invoke $a "get") (i32.const 4))

;; Imported modules work
(module
  (import "a" "inc" (func $set))

  (module $m1
    (import "" (module $m (export "" (func $f (result i32)))))
    (instance $i (instantiate $m))
    (func $get (export "") (result i32)
      call (func $i "")))

  (module $m2
    (func (export "") (result i32)
      i32.const 5))
  (instance $i (instantiate $m1 "" (module $m2)))
  (func (export "get") (result i32)
    call (func $i ""))
)
(assert_return (invoke "get") (i32.const 5))

;; imported modules again
(module
  (module $m
    (import "" (module $m (export "get" (func (result i32)))))
    (instance $i (instantiate $m))
    (alias $f (func $i "get"))
    (export "" (func $f))
  )
  (module $m2
    (func (export "get") (result i32)
      i32.const 6))
  (instance $a (instantiate $m "" (module $m2)))

  (func (export "get") (result i32)
    call (func $a ""))
)
(assert_return (invoke "get") (i32.const 6))

;; all at once
(module
  (import "a" "inc" (func $f))
  (import "a" "global" (global $g (mut i32)))
  (import "a" "table" (table $t 1 funcref))
  (import "a" "memory" (memory $m 1))

  (module
    (import "m" (memory 1))
    (import "g" (global (mut i32)))
    (import "t" (table 1 funcref))
    (import "f" (func))
    (func $start
      call 0

      i32.const 0
      i32.const 4
      i32.store

      i32.const 0
      call_indirect

      global.get 0
      global.set 0)
    (start $start))

  (instance $a
    (instantiate 0
      "m" (memory $m)
      "g" (global $g)
      "t" (table $t)
      "f" (func $f)
    )
  )
)

;; instantiate lots
(module
  (import "a" "inc" (func $f))
  (import "a" "global" (global $g (mut i32)))
  (import "a" "table" (table $t 1 funcref))
  (import "a" "memory" (memory $m 1))

  (module $mm (import "" (memory 1)))
  (module $mf (import "" (func)))
  (module $mt (import "" (table 1 funcref)))
  (module $mg (import "" (global (mut i32))))

  (instance (instantiate $mm "" (memory $m)))
  (instance (instantiate $mf "" (func $f)))
  (instance (instantiate $mt "" (table $t)))
  (instance (instantiate $mg "" (global $g)))
)

;; instantiate nested
(assert_return (invoke $a "reset"))
(assert_return (invoke $a "get") (i32.const 0))
(module
  (import "a" "inc" (func))
  (module
    (import "" (func))
    (module
      (import "" (func))
      (module
        (import "" (func))
        (module
          (import "" (func))
          (start 0)
        )
        (instance (instantiate 0 "" (func 0)))
      )
      (instance (instantiate 0 "" (func 0)))
    )
    (instance (instantiate 0 "" (func 0)))
  )
  (instance (instantiate 0 "" (func 0)))
)
(assert_return (invoke $a "get") (i32.const 1))

;; module/instance top-level imports work
(module $b
  (module (export "m"))
  (instance (export "i") (instantiate 0))
)
(module (import "b" "m" (module)))
(module (import "b" "m" (module (import "" (func)))))
(module (import "b" "i" (instance)))
(assert_unlinkable
  (module
    (import "b" "i" (instance (export "" (func))))
  )
  "incompatible import type")

;; ensure we ignore other exported items
(module $b
  (module $m
    (func (export "f") (result i32)
      i32.const 300)
    (global (export "g") i32 (i32.const 0xfeed))
  )

  (instance (export "i") (instantiate 0))
)
(module
  (import "b" "i" (instance $i
    (export "g" (global $g i32))
  ))

  (func (export "get") (result i32)
    global.get (global $i "g"))
)
(assert_return (invoke "get") (i32.const 0xfeed))

;; ensure the right export is used even when subtyping comes into play
(module $b
  (module $m
    (func (export "f") (result i32)
      i32.const 300)
    (func (export "g") (param i32) (result i32)
      i32.const 100
      local.get 0
      i32.add)
  )

  (instance (export "i") (instantiate 0))
)
(module
  (import "b" "i" (instance $i
    ;; notice that this order is swapped
    (export "g" (func (param i32) (result i32)))
    (export "f" (func (result i32)))
  ))

  (func (export "f") (result i32)
    call (func $i "f"))
  (func (export "g") (param i32) (result i32)
    local.get 0
    call (func $i "g"))
)
(assert_return (invoke "f") (i32.const 300))
(assert_return (invoke "g" (i32.const 3000)) (i32.const 3100))

(module $a
  (func (export "f")))

(module
  (import "a" "f" (func))

  (module $m1
    (import "a" "f" (func)))
  (instance (instantiate $m1 "a" (instance 0)))
)

(module
  (import "a" "f" (func))

  ;; this module provides nothing
  (module $m1)

  ;; this module imports a module which it says imports something
  (module $m2
    (module $a
      (func (export "")))
    (instance $i (instantiate $a))
    (import "m" (module $b (import "" (func))))
    (instance $b (instantiate $b "" (func $i ""))))

  ;; we should be able to instantiate m2 with m1 because m1 doesn't actually
  ;; import anything (always safe to remove imports!)
  (instance (instantiate $m2 "m" (module $m1)))
)
