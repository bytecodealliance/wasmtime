;;! multi_memory = true
;;! flags = "-S max-component-handles=20"

;; Tests for the store-level cap on simultaneously-live component-model
;; handles, set to 20 here. Each component instantiation gets a fresh store.
;;
;; `$C` is instantiated twice, so each instance has its own handle table.
;; `spray(n)` allocates `n` live handles; `churn(n)` allocates and drops one
;; `n` times, staying at one live; `drop-all(n)` drops handles `1..=n`, the
;; ones a first `spray(n)` returns.
(component definition $Tester
  (component $C
    (type $r (resource (rep i32)))
    (core func $resource.new (canon resource.new $r))
    (core func $resource.drop (canon resource.drop $r))

    (core module $m
      (import "" "resource.new" (func $resource.new (param i32) (result i32)))
      (import "" "resource.drop" (func $resource.drop (param i32)))
      (func (export "spray") (param $n i32) (result i32)
        (local $i i32)
        (loop $l
          (if (i32.lt_u (local.get $i) (local.get $n))
            (then
              (drop (call $resource.new (local.get $i)))
              (local.set $i (i32.add (local.get $i) (i32.const 1)))
              (br $l))))
        (local.get $i))
      (func (export "churn") (param $n i32) (result i32)
        (local $i i32)
        (loop $l
          (if (i32.lt_u (local.get $i) (local.get $n))
            (then
              (call $resource.drop (call $resource.new (local.get $i)))
              (local.set $i (i32.add (local.get $i) (i32.const 1)))
              (br $l))))
        (local.get $i))
      (func (export "drop-all") (param $n i32)
        (local $i i32)
        (loop $l
          (if (i32.lt_u (local.get $i) (local.get $n))
            (then
              (local.set $i (i32.add (local.get $i) (i32.const 1)))
              (call $resource.drop (local.get $i))
              (br $l))))))
    (core instance $i (instantiate $m
      (with "" (instance
        (export "resource.new" (func $resource.new))
        (export "resource.drop" (func $resource.drop))
      ))
    ))

    (func (export "spray") (param "n" u32) (result u32)
      (canon lift (core func $i "spray")))
    (func (export "churn") (param "n" u32) (result u32)
      (canon lift (core func $i "churn")))
    (func (export "drop-all") (param "n" u32)
      (canon lift (core func $i "drop-all"))))

  (instance $i0 (instantiate $C))
  (export "spray0" (func $i0 "spray"))
  (export "churn0" (func $i0 "churn"))
  (export "drop-all0" (func $i0 "drop-all"))
  (instance $i1 (instantiate $C))
  (export "spray1" (func $i1 "spray")))

;; The cap is per store, not per table: 12 handles in each of two instances
;; cross the limit together but not individually.
(component instance $t $Tester)
(assert_return (invoke "spray0" (u32.const 12)) (u32.const 12))
(assert_trap (invoke "spray1" (u32.const 12)) "store handle limit exceeded")

;; The cap counts live table slots, not cumulative allocations: churning far
;; past the limit succeeds because each dropped slot is reused, and the whole
;; budget is available afterwards.
(component instance $t $Tester)
(assert_return (invoke "churn0" (u32.const 2000)) (u32.const 2000))
(assert_return (invoke "spray0" (u32.const 15)) (u32.const 15))

;; Freed slots still count: after the first instance's table grows to 15 slots
;; and frees them all, the second can't grow its table past the remaining 5.
(component instance $t $Tester)
(assert_return (invoke "spray0" (u32.const 15)) (u32.const 15))
(assert_return (invoke "drop-all0" (u32.const 15)))
(assert_trap (invoke "spray1" (u32.const 10)) "store handle limit exceeded")

;; Borrows lent to the resource's owner count against the limit while the call
;; is active, and return their budget when it returns.
;;
;; `$D` defines a resource and `$U` passes a `list<borrow<R>>` of `n` copies of
;; one handle to `$D`'s `take`, `k` times. `$D` receives reps, so no handles
;; are created for the borrows, but each lend still counts against the limit
;; until `take` returns.
(component definition $Lender
  (component $D
    (type $R (resource (rep i32)))
    (core func $resource.new (canon resource.new $R))

    (core module $m
      (import "" "resource.new" (func $resource.new (param i32) (result i32)))
      (memory (export "mem") 1)
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        (i32.const 16))
      (func (export "make") (result i32)
        (call $resource.new (i32.const 42)))
      (func (export "take") (param i32 i32) (result i32)
        (local.get 1)))
    (core instance $i (instantiate $m
      (with "" (instance
        (export "resource.new" (func $resource.new))
      ))
    ))

    (export $R' "r" (type $R))
    (func (export "make") (result (own $R'))
      (canon lift (core func $i "make")))
    (func (export "take") (param "xs" (list (borrow $R'))) (result u32)
      (canon lift (core func $i "take")
        (memory (core memory $i "mem"))
        (realloc (core func $i "realloc")))))
  (instance $d (instantiate $D))

  (component $U
    (import "r" (type $R (sub resource)))
    (import "make" (func $make (result (own $R))))
    (import "take" (func $take (param "xs" (list (borrow $R))) (result u32)))

    (core module $libc (memory (export "mem") 1))
    (core instance $libc (instantiate $libc))
    (core func $make (canon lower (func $make)))
    (core func $take
      (canon lower (func $take) (memory (core memory $libc "mem"))))

    (core module $m
      (import "" "mem" (memory 1))
      (import "" "make" (func $make (result i32)))
      (import "" "take" (func $take (param i32 i32) (result i32)))
      (func (export "run") (param $n i32) (param $k i32) (result i32)
        (local $h i32)
        (local $i i32)
        (local.set $h (call $make))
        (loop $l
          (if (i32.lt_u (local.get $i) (local.get $n))
            (then
              (i32.store (i32.shl (local.get $i) (i32.const 2)) (local.get $h))
              (local.set $i (i32.add (local.get $i) (i32.const 1)))
              (br $l))))
        (local.set $i (i32.const 0))
        (loop $l
          (if (i32.lt_u (local.get $i) (local.get $k))
            (then
              (drop (call $take (i32.const 0) (local.get $n)))
              (local.set $i (i32.add (local.get $i) (i32.const 1)))
              (br $l))))
        (local.get $k)))
    (core instance $i (instantiate $m
      (with "" (instance
        (export "mem" (memory $libc "mem"))
        (export "make" (func $make))
        (export "take" (func $take))
      ))
    ))

    (func (export "run") (param "n" u32) (param "k" u32) (result u32)
      (canon lift (core func $i "run"))))
  (instance $u (instantiate $U
    (with "r" (type $d "r"))
    (with "make" (func $d "make"))
    (with "take" (func $d "take"))))
  (export "run" (func $u "run")))

;; Ten lends at a time, many times over, fit.
(component instance $l $Lender)
(assert_return (invoke "run" (u32.const 10) (u32.const 100)) (u32.const 100))

;; Fifty lends in one call don't.
(component instance $l $Lender)
(assert_trap (invoke "run" (u32.const 50) (u32.const 1))
  "store handle limit exceeded")
