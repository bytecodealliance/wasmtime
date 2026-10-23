;; Guest-to-guest `borrow` transfers on both sides of FACT's choice between
;; `resource.transfer-borrow-to-owner` and `resource.transfer-borrow`.

;; The callee defines the resource, so `borrow` params arrive as reps.
(component
  (component $A
    (type $t' (resource (rep i32)))
    (export $t "t" (type $t'))

    (core func $ctor (canon resource.new $t))

    (core module $m
      ;; The offset tells a rep apart from a handle index.
      (func (export "[method]t.get") (param i32) (result i32)
        (i32.add (local.get 0) (i32.const 100)))

      ;; Never reached: lifting `b` traps, `self` already lent the handle out.
      (func (export "[method]t.take-own") (param i32) (param i32)
        (unreachable))
    )
    (core instance $i (instantiate $m))

    (func (export "[constructor]t") (param "x" u32) (result (own $t))
      (canon lift (core func $ctor)))
    (func (export "[method]t.get") (param "self" (borrow $t)) (result u32)
      (canon lift (core func $i "[method]t.get")))
    (func (export "[method]t.take-own") (param "self" (borrow $t)) (param "b" (own $t))
      (canon lift (core func $i "[method]t.take-own")))
  )
  (instance $a (instantiate $A))

  (component $B
    (import "a" (instance $i
      (export "t" (type $t (sub resource)))
      (export "[constructor]t" (func (param "x" u32) (result (own $t))))
      (export "[method]t.get" (func (param "self" (borrow $t)) (result u32)))
      (export "[method]t.take-own" (func (param "self" (borrow $t)) (param "b" (own $t))))
    ))

    (alias export $i "t" (type $t))
    (alias export $i "[constructor]t" (func $ctor))
    (alias export $i "[method]t.get" (func $get))
    (alias export $i "[method]t.take-own" (func $take-own))

    (core func $ctor' (canon lower (func $ctor)))
    (core func $get' (canon lower (func $get)))
    (core func $take-own' (canon lower (func $take-own)))
    (core func $drop (canon resource.drop $t))

    (core module $m
      (import "" "ctor" (func $ctor (param i32) (result i32)))
      (import "" "get" (func $get (param i32) (result i32)))
      (import "" "take-own" (func $take-own (param i32 i32)))
      (import "" "drop" (func $drop (param i32)))

      (func (export "get") (param i32) (result i32)
        (local $h i32) (local $v i32)
        (local.set $h (call $ctor (local.get 0)))
        (local.set $v (call $get (local.get $h)))
        (call $drop (local.get $h))
        (local.get $v))

      (func (export "lend-then-give-away")
        (local $h i32)
        (local.set $h (call $ctor (i32.const 100)))
        (call $take-own (local.get $h) (local.get $h)))
    )
    (core instance $i2 (instantiate $m (with "" (instance
      (export "ctor" (func $ctor'))
      (export "get" (func $get'))
      (export "take-own" (func $take-own'))
      (export "drop" (func $drop))))))

    (func (export "get") (param "x" u32) (result u32)
      (canon lift (core func $i2 "get")))
    (func (export "lend-then-give-away")
      (canon lift (core func $i2 "lend-then-give-away")))
  )
  (instance $b (instantiate $B (with "a" (instance $a))))
  (export "get" (func $b "get"))
  (export "lend-then-give-away" (func $b "lend-then-give-away"))
)

(assert_return (invoke "get" (u32.const 42)) (u32.const 142))
(assert_trap (invoke "lend-then-give-away") "cannot remove owned resource while borrowed")

;; Neither guest defines the host's resource, so `borrow` params arrive as
;; handles, which the callee must drop.
(component
  (import "host" (instance $host
    (export "resource1" (type $r (sub resource)))
    (export "[constructor]resource1" (func (param "r" u32) (result (own $r))))
  ))
  (alias export $host "resource1" (type $r))
  (alias export $host "[constructor]resource1" (func $ctor))

  (component $A
    (import "t" (type $t (sub resource)))
    (core func $drop (canon resource.drop $t))
    (core module $m
      (import "" "drop" (func $drop (param i32)))
      (func (export "get") (param i32) (result i32)
        (call $drop (local.get 0))
        (i32.add (local.get 0) (i32.const 1000)))
    )
    (core instance $i (instantiate $m
      (with "" (instance (export "drop" (func $drop))))))
    (func (export "get") (param "self" (borrow $t)) (result u32)
      (canon lift (core func $i "get")))
  )
  (instance $a (instantiate $A (with "t" (type $r))))

  (component $B
    (import "t" (type $t (sub resource)))
    (import "a" (instance $i
      (export "get" (func (param "self" (borrow $t)) (result u32)))
    ))
    (alias export $i "get" (func $get))

    (core func $get' (canon lower (func $get)))
    (core func $drop (canon resource.drop $t))
    (core module $m
      (import "" "get" (func $get (param i32) (result i32)))
      (import "" "drop" (func $drop (param i32)))
      (func (export "get") (param i32) (result i32)
        (local $v i32)
        (local.set $v (call $get (local.get 0)))
        (call $drop (local.get 0))
        (local.get $v))
    )
    (core instance $i2 (instantiate $m (with "" (instance
      (export "get" (func $get'))
      (export "drop" (func $drop))))))
    (func (export "get") (param "self" (borrow $t)) (result u32)
      (canon lift (core func $i2 "get")))
  )
  (instance $b (instantiate $B
    (with "t" (type $r))
    (with "a" (instance $a))))
  (alias export $b "get" (func $b-get))

  ;; The host owns the resource, so this outermost component is not the owner
  ;; either: it creates an `own` handle via the host constructor and lends it
  ;; down the chain, which is what gets `A` and `B` actually executed.
  (core func $ctor' (canon lower (func $ctor)))
  (core func $b-get' (canon lower (func $b-get)))
  (core func $drop (canon resource.drop $r))
  (core module $m
    (import "" "ctor" (func $ctor (param i32) (result i32)))
    (import "" "get" (func $get (param i32) (result i32)))
    (import "" "drop" (func $drop (param i32)))
    (func (export "run") (param i32) (result i32)
      (local $h i32) (local $v i32)
      (local.set $h (call $ctor (local.get 0)))
      (local.set $v (call $get (local.get $h)))
      (call $drop (local.get $h))
      (local.get $v))
  )
  (core instance $i (instantiate $m (with "" (instance
    (export "ctor" (func $ctor'))
    (export "get" (func $b-get'))
    (export "drop" (func $drop))))))
  (func (export "run") (param "x" u32) (result u32)
    (canon lift (core func $i "run")))
)

(assert_return (invoke "run" (u32.const 42)) (u32.const 1001))
