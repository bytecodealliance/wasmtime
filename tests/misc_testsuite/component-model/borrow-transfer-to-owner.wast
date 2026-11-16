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

;; An out-of-bounds handle takes the libcall, keeping the host's diagnostic.
;; Separate component since a trap locks the instance down.
(component
  (component $A
    (type $t' (resource (rep i32)))
    (export $t "t" (type $t'))
    (core func $ctor (canon resource.new $t))
    (core module $m
      (func (export "[method]t.get") (param i32) (result i32)
        (local.get 0))
    )
    (core instance $i (instantiate $m))
    (func (export "[constructor]t") (param "x" u32) (result (own $t))
      (canon lift (core func $ctor)))
    (func (export "[method]t.get") (param "self" (borrow $t)) (result u32)
      (canon lift (core func $i "[method]t.get")))
  )
  (instance $a (instantiate $A))

  (component $B
    (import "a" (instance $i
      (export "t" (type $t (sub resource)))
      (export "[method]t.get" (func (param "self" (borrow $t)) (result u32)))
    ))
    (alias export $i "t" (type $t))
    (alias export $i "[method]t.get" (func $get))
    (core func $get' (canon lower (func $get)))
    (core module $m
      (import "" "get" (func $get (param i32) (result i32)))
      (func (export "lend-unknown") (result i32)
        (call $get (i32.const 1234)))
    )
    (core instance $i2 (instantiate $m
      (with "" (instance (export "get" (func $get'))))))
    (func (export "lend-unknown") (result u32)
      (canon lift (core func $i2 "lend-unknown")))
  )
  (instance $b (instantiate $B (with "a" (instance $a))))
  (export "lend-unknown" (func $b "lend-unknown"))
)

(assert_trap (invoke "lend-unknown") "unknown handle index 1234")

;; The `own` transfer forces the deferred thread, so the `borrow` after it must
;; be tracked as a real lend rather than recorded lazily.
(component
  (component $A
    (type $t' (resource (rep i32)))
    (export $t "t" (type $t'))

    (core func $ctor (canon resource.new $t))
    (core func $drop (canon resource.drop $t))

    (core module $m
      (import "" "drop" (func $drop (param i32)))
      ;; `o` is an owned handle to drop; `b` is a rep since we own `t`.
      (func (export "take") (param $o i32) (param $b i32) (result i32)
        (call $drop (local.get $o))
        (local.get $b))
    )
    (core instance $i (instantiate $m
      (with "" (instance (export "drop" (func $drop))))))

    (func (export "[constructor]t") (param "x" u32) (result (own $t))
      (canon lift (core func $ctor)))
    (func (export "take") (param "o" (own $t)) (param "b" (borrow $t)) (result u32)
      (canon lift (core func $i "take")))
  )
  (instance $a (instantiate $A))

  (component $B
    (import "a" (instance $i
      (export "t" (type $t (sub resource)))
      (export "[constructor]t" (func (param "x" u32) (result (own $t))))
      (export "take" (func (param "o" (own $t)) (param "b" (borrow $t)) (result u32)))
    ))
    (alias export $i "t" (type $t))
    (alias export $i "[constructor]t" (func $ctor))
    (alias export $i "take" (func $take))

    (core func $ctor' (canon lower (func $ctor)))
    (core func $take' (canon lower (func $take)))
    (core func $drop (canon resource.drop $t))

    (core module $m
      (import "" "ctor" (func $ctor (param i32) (result i32)))
      (import "" "take" (func $take (param i32 i32) (result i32)))
      (import "" "drop" (func $drop (param i32)))

      (func (export "run") (result i32)
        (local $given i32) (local $lent i32) (local $v i32)
        (local.set $given (call $ctor (i32.const 1)))
        (local.set $lent (call $ctor (i32.const 42)))
        (local.set $v (call $take (local.get $given) (local.get $lent)))
        ;; Only succeeds if the lend of `$lent` was undone on the way out.
        (call $drop (local.get $lent))
        (local.get $v))
    )
    (core instance $i2 (instantiate $m (with "" (instance
      (export "ctor" (func $ctor'))
      (export "take" (func $take'))
      (export "drop" (func $drop))))))

    (func (export "run") (result u32)
      (canon lift (core func $i2 "run")))
  )
  (instance $b (instantiate $B (with "a" (instance $a))))
  (export "lend-after-force" (func $b "run"))
)

(assert_return (invoke "lend-after-force") (u32.const 42))

;; A wrong-typed handle takes the libcall, keeping the host's diagnostic.
(component
  (component $A
    (type $t' (resource (rep i32)))
    (type $u' (resource (rep i32)))
    (export $t "t" (type $t'))
    (export $u "u" (type $u'))
    (core func $ctor-u (canon resource.new $u))
    (core module $m
      (func (export "get") (param i32) (result i32)
        (local.get 0))
    )
    (core instance $i (instantiate $m))
    (func (export "[constructor]u") (param "x" u32) (result (own $u))
      (canon lift (core func $ctor-u)))
    (func (export "get") (param "self" (borrow $t)) (result u32)
      (canon lift (core func $i "get")))
  )
  (instance $a (instantiate $A))

  (component $B
    (import "a" (instance $i
      (export "t" (type $t (sub resource)))
      (export "u" (type $u (sub resource)))
      (export "[constructor]u" (func (param "x" u32) (result (own $u))))
      (export "get" (func (param "self" (borrow $t)) (result u32)))
    ))
    (alias export $i "[constructor]u" (func $ctor-u))
    (alias export $i "get" (func $get))
    (core func $ctor-u' (canon lower (func $ctor-u)))
    (core func $get' (canon lower (func $get)))
    (core module $m
      (import "" "ctor-u" (func $ctor-u (param i32) (result i32)))
      (import "" "get" (func $get (param i32) (result i32)))
      ;; Lends a `u` handle where a `t` is expected.
      (func (export "lend-wrong-type") (result i32)
        (call $get (call $ctor-u (i32.const 7))))
    )
    (core instance $i2 (instantiate $m (with "" (instance
      (export "ctor-u" (func $ctor-u'))
      (export "get" (func $get'))))))
    (func (export "lend-wrong-type") (result u32)
      (canon lift (core func $i2 "lend-wrong-type")))
  )
  (instance $b (instantiate $B (with "a" (instance $a))))
  (export "lend-wrong-type" (func $b "lend-wrong-type"))
)

(assert_trap (invoke "lend-wrong-type")
  "handle index 1 used with the wrong type, expected guest-defined resource but found a different guest-defined resource")
