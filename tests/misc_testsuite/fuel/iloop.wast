;;! consume_fuel = true

;; Infinite loops and exponential call trees run out of fuel.
(module $Fuel
  (import "wasmtime" "set-fuel" (func $set (param i64)))
  (export "set-fuel" (func $set)))

(invoke $Fuel "set-fuel" (i64.const 10000))
(assert_trap
  (module
    (start 0)
    (func loop br 0 end))
  "all fuel consumed")

(invoke $Fuel "set-fuel" (i64.const 10000))
(assert_trap
  (module
    (start 0)
    (func loop i32.const 1 br_if 0 end))
  "all fuel consumed")

(invoke $Fuel "set-fuel" (i64.const 10000))
(assert_trap
  (module
    (start 0)
    (func loop i32.const 0 br_table 0 end))
  "all fuel consumed")

(invoke $Fuel "set-fuel" (i64.const 10000))
(assert_trap
  (module
    (start 0)
    (func $f0 call $f1 call $f1)
    (func $f1 call $f2 call $f2)
    (func $f2 call $f3 call $f3)
    (func $f3 call $f4 call $f4)
    (func $f4 call $f5 call $f5)
    (func $f5 call $f6 call $f6)
    (func $f6 call $f7 call $f7)
    (func $f7 call $f8 call $f8)
    (func $f8 call $f9 call $f9)
    (func $f9 call $f10 call $f10)
    (func $f10 call $f11 call $f11)
    (func $f11 call $f12 call $f12)
    (func $f12 call $f13 call $f13)
    (func $f13 call $f14 call $f14)
    (func $f14 call $f15 call $f15)
    (func $f15 call $f16 call $f16)
    (func $f16))
  "all fuel consumed")
