;;! bulk_memory = true
;;! gc = true
;;! consume_fuel = true

(module $Fuel
  (import "wasmtime" "set-fuel" (func $set (param i64)))
  (export "set-fuel" (func $set)))

;; Infinite loops through the typed-reference branch instructions run out of
;; fuel.
(invoke $Fuel "set-fuel" (i64.const 10000))
(assert_trap
  (module
    (start 0)
    (func loop ref.null func br_on_null 0 drop end))
  "all fuel consumed")

(invoke $Fuel "set-fuel" (i64.const 10000))
(assert_trap
  (module
    (start 0)
    (func
      ref.func 0
      loop (param (ref func))
        br_on_non_null 0
        unreachable
      end)
    (elem declare func 0))
  "all fuel consumed")

(invoke $Fuel "set-fuel" (i64.const 10000))
(assert_trap
  (module
    (start 0)
    (func
      i32.const 0
      ref.i31
      loop (param (ref i31))
        br_on_cast 0 anyref (ref i31)
        unreachable
      end))
  "all fuel consumed")

(invoke $Fuel "set-fuel" (i64.const 10000))
(assert_trap
  (module
    (start 0)
    (func
      ref.null any
      loop (param anyref)
        br_on_cast_fail 0 anyref (ref i31)
        unreachable
      end))
  "all fuel consumed")

;; An array bulk op whose size-proportional cost exceeds the remaining fuel
;; runs out of fuel.
(invoke $Fuel "set-fuel" (i64.const 100))
(assert_trap
  (module
    (type $a (array i8))
    (start 0)
    (func
      i32.const 200
      array.new_default $a
      drop))
  "all fuel consumed")

(invoke $Fuel "set-fuel" (i64.const 100))
(assert_trap
  (module
    (type $a (array (mut i8)))
    (start 0)
    (func
      i32.const 0
      i32.const 200
      array.new $a
      drop))
  "all fuel consumed")

(invoke $Fuel "set-fuel" (i64.const 10000))
(module $Arrays
  (type $a (array (mut i8)))
  (global $a (export "a") (ref $a) (array.new_default $a (i32.const 200)))
  (global $b (export "b") (ref $a) (array.new_default $a (i32.const 200))))
(register "arrays" $Arrays)

(invoke $Fuel "set-fuel" (i64.const 100))
(assert_trap
  (module
    (type $a (array (mut i8)))
    (import "arrays" "a" (global $a (ref $a)))
    (import "arrays" "b" (global $b (ref $a)))
    (start 0)
    (func
      global.get $a
      i32.const 0
      global.get $b
      i32.const 0
      i32.const 200
      array.copy $a $a))
  "all fuel consumed")

(invoke $Fuel "set-fuel" (i64.const 100))
(assert_trap
  (module
    (type $a (array (mut i8)))
    (import "arrays" "a" (global $a (ref $a)))
    (start 0)
    (func
      global.get $a
      i32.const 0
      i32.const 0
      i32.const 200
      array.fill $a))
  "all fuel consumed")

(invoke $Fuel "set-fuel" (i64.const 100))
(assert_trap
  (module
    (type $a (array (mut i8)))
    (start 0)
    (func
      i32.const 0
      i32.const 200
      array.new_data $a $d
      drop)
    (data $d
      "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"))
  "all fuel consumed")

(invoke $Fuel "set-fuel" (i64.const 100))
(assert_trap
  (module
    (type $a (array (mut i8)))
    (import "arrays" "a" (global $a (ref $a)))
    (start 0)
    (func
      global.get $a
      i32.const 0
      i32.const 0
      i32.const 200
      array.init_data $a $d)
    (data $d
      "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"))
  "all fuel consumed")

(invoke $Fuel "set-fuel" (i64.const 10000))
(module $FuncArray
  (type $a (array (mut funcref)))
  (global (export "a") (ref $a) (array.new_default $a (i32.const 200))))
(register "func-array" $FuncArray)

(invoke $Fuel "set-fuel" (i64.const 100))
(assert_trap
  (module
    (type $a (array (mut funcref)))
    (start 0)
    (func
      i32.const 0
      i32.const 200
      array.new_elem $a $e
      drop)
    (func $f)
    (elem $e func
      $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f
      $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f
      $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f
      $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f
      $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f
      $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f
      $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f
      $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f))
  "all fuel consumed")

(invoke $Fuel "set-fuel" (i64.const 100))
(assert_trap
  (module
    (type $a (array (mut funcref)))
    (import "func-array" "a" (global $a (ref $a)))
    (start 0)
    (func
      global.get $a
      i32.const 0
      i32.const 0
      i32.const 200
      array.init_elem $a $e)
    (func $f)
    (elem $e func
      $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f
      $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f
      $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f
      $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f
      $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f
      $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f
      $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f
      $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f))
  "all fuel consumed")

;; Fuel is saved and reloaded around `call_ref`, so a deep recursion through
;; it still runs out of fuel (#14407).
(module
  (type $t (func (param i32 i32)))
  (func $run (export "run") (param $cnt i32) (param $depth i32)
    (local $i i32)
    (local.set $i (local.get $cnt))
    local.get $depth
    if
      (local.set $depth (i32.sub (local.get $depth) (i32.const 1)))
      loop $l
        (call_ref $t
          (local.get $cnt)
          (local.get $depth)
          (ref.func $run))
        (local.tee $i (i32.sub (local.get $i) (i32.const 1)))
        if br $l end
      end
    end))
(invoke $Fuel "set-fuel" (i64.const 100000))
(assert_trap (invoke "run" (i32.const 10) (i32.const 10)) "all fuel consumed")

;; The fuel for a large `table.grow` is charged after the grow completes, so
;; the call runs out of fuel but the table still grew.
(invoke $Fuel "set-fuel" (i64.const 10000))
(module
  (type $ft (func))
  (func $f (type $ft))
  (table $t 1 1000 (ref $ft) (ref.func $f))
  (func (export "grow") (result i32)
    (table.grow $t (ref.func $f) (i32.const 999)))
  (func (export "call") (param i32)
    (call_indirect $t (type $ft) (local.get 0))))
(invoke $Fuel "set-fuel" (i64.const 2))
(assert_trap (invoke "grow") "all fuel consumed")
(invoke $Fuel "set-fuel" (i64.const 10000))
(assert_return (invoke "call" (i32.const 999)))
