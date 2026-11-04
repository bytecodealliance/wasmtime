;;! bulk_memory = true
;;! reference_types = true
;;! memory64 = true
;;! consume_fuel = true

;; Fuel charged by bulk memory and table operations, on top of the usual cost
;; per operator. Like `costs.wast`, each test sets the fuel to 1000,
;; instantiates a module, and checks how much fuel is left.
(module $Fuel
  (import "wasmtime" "set-fuel" (func $set (param i64)))
  (import "wasmtime" "get-fuel" (func $get (result i64)))
  (export "set-fuel" (func $set))
  (export "get-fuel" (func $get)))

;; A small constant size (size * cost <= 128, the `SMALL_BULK_OP_COST`
;; threshold) charges the size-proportional "variable" fuel up front, before
;; the op runs.
(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (memory 1)
  (func $f
    i32.const 0
    i32.const 0
    i32.const 100
    memory.copy
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 893))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (memory 1)
  (func $f
    i32.const 0
    i32.const 0
    i32.const 100
    memory.fill
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 893))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (memory 1)
  (func $f
    i32.const 0
    i32.const 0
    i32.const 20
    memory.init $d
  )
  (start $f)
  (data $d "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"))
(assert_return (invoke $Fuel "get-fuel") (i64.const 973))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (table 100 funcref)
  (func $f
    i32.const 0
    i32.const 0
    i32.const 100
    table.copy
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 893))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (table 100 funcref)
  (func $f
    i32.const 0
    ref.null func
    i32.const 100
    table.fill
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 893))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (table 20 funcref)
  (func $f
    i32.const 0
    i32.const 0
    i32.const 20
    table.init $e
  )
  (start $f)
  (elem $e func $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 973))

;; A large constant size (> 128) instead defers that variable fuel until
;; after the op has run.
(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (memory 1)
  (func $f
    i32.const 0
    i32.const 0
    i32.const 200
    memory.copy
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 793))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (memory 1)
  (func $f
    i32.const 0
    i32.const 0
    i32.const 200
    memory.fill
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 793))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (memory 1)
  (func $f
    i32.const 0
    i32.const 0
    i32.const 200
    memory.init $d
  )
  (start $f)
  (data $d "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"))
(assert_return (invoke $Fuel "get-fuel") (i64.const 793))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (table 200 funcref)
  (func $f
    i32.const 0
    i32.const 0
    i32.const 200
    table.copy
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 793))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (table 200 funcref)
  (func $f
    i32.const 0
    ref.null func
    i32.const 200
    table.fill
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 793))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (table 200 funcref)
  (func $f
    i32.const 0
    i32.const 0
    i32.const 200
    table.init $e
  )
  (start $f)
  (elem $e func $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 793))

;; A dynamic (runtime) size likewise defers the variable fuel until after
;; the op has run, whatever its magnitude.
(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (memory 1)
  (global $n (mut i32) (i32.const 50))
  (func $f
    i32.const 0
    i32.const 0
    global.get $n
    memory.copy
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 943))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (memory 1)
  (global $n (mut i32) (i32.const 50))
  (func $f
    i32.const 0
    i32.const 0
    global.get $n
    memory.fill
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 943))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (memory 1)
  (global $n (mut i32) (i32.const 50))
  (func $f
    i32.const 0
    i32.const 0
    global.get $n
    memory.init $d
  )
  (start $f)
  (data $d "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"))
(assert_return (invoke $Fuel "get-fuel") (i64.const 943))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (table 50 funcref)
  (global $n (mut i32) (i32.const 50))
  (func $f
    i32.const 0
    i32.const 0
    global.get $n
    table.copy
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 943))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (table 50 funcref)
  (global $n (mut i32) (i32.const 50))
  (func $f
    i32.const 0
    ref.null func
    global.get $n
    table.fill
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 943))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (table 50 funcref)
  (global $n (mut i32) (i32.const 50))
  (func $f
    i32.const 0
    i32.const 0
    global.get $n
    table.init $e
  )
  (start $f)
  (elem $e func $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 943))

;; Growing a memory or table charges a small constant size's variable fuel up
;; front, but defers a large or dynamic size's variable fuel until the grow has
;; succeeded.
;;
;; The following tests cover the 12 cases of table/memory x small/large/dynamic
;; size x success/failure.
(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (table 0 funcref)
  (func $f
    ref.null func
    i32.const 100
    table.grow
    drop
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 894))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (table 0 0 funcref)
  (func $f
    ref.null func
    i32.const 100
    table.grow
    drop
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 894))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (table 0 funcref)
  (func $f
    ref.null func
    i32.const 200
    table.grow
    drop
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 794))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (table 0 0 funcref)
  (func $f
    ref.null func
    i32.const 200
    table.grow
    drop
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 994))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (table 0 funcref)
  (global $d (mut i32) (i32.const 50))
  (func $f
    ref.null func
    global.get $d
    table.grow
    drop
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 944))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (table 0 0 funcref)
  (global $d (mut i32) (i32.const 50))
  (func $f
    ref.null func
    global.get $d
    table.grow
    drop
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 994))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (memory 1)
  (func $f
    i32.const 100
    memory.grow
    drop
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 895))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (memory 0 0)
  (func $f
    i32.const 100
    memory.grow
    drop
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 895))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (memory 1)
  (func $f
    i32.const 200
    memory.grow
    drop
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 795))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (memory 0 0)
  (func $f
    i32.const 200
    memory.grow
    drop
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 995))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (memory 1)
  (global $d (mut i32) (i32.const 50))
  (func $f
    global.get $d
    memory.grow
    drop
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 945))

(invoke $Fuel "set-fuel" (i64.const 1000))
(module
  (memory 0 0)
  (global $d (mut i32) (i32.const 50))
  (func $f
    global.get $d
    memory.grow
    drop
  )
  (start $f))
(assert_return (invoke $Fuel "get-fuel") (i64.const 995))

;; A bulk op whose size-proportional cost exceeds the remaining fuel runs out
;; of fuel.
(invoke $Fuel "set-fuel" (i64.const 100))
(assert_trap
  (module
    (memory 1)
    (start 0)
    (func
      i32.const 0
      i32.const 0
      i32.const 200
      memory.copy))
  "all fuel consumed")

(invoke $Fuel "set-fuel" (i64.const 100))
(assert_trap
  (module
    (memory 1)
    (start 0)
    (func
      i32.const 0
      i32.const 0
      i32.const 200
      memory.fill))
  "all fuel consumed")

(invoke $Fuel "set-fuel" (i64.const 100))
(assert_trap
  (module
    (memory 1)
    (start 0)
    (func
      i32.const 0
      i32.const 0
      i32.const 200
      memory.init $d)
    (data $d
      "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"))
  "all fuel consumed")

(invoke $Fuel "set-fuel" (i64.const 100))
(assert_trap
  (module
    (table 200 funcref)
    (start 0)
    (func
      i32.const 0
      i32.const 0
      i32.const 200
      table.copy))
  "all fuel consumed")

(invoke $Fuel "set-fuel" (i64.const 100))
(assert_trap
  (module
    (table 200 funcref)
    (start 0)
    (func
      i32.const 0
      ref.null func
      i32.const 200
      table.fill))
  "all fuel consumed")

(invoke $Fuel "set-fuel" (i64.const 100))
(assert_trap
  (module
    (table 0 funcref)
    (start 0)
    (func
      ref.null func
      i32.const 200
      table.grow
      drop))
  "all fuel consumed")

(invoke $Fuel "set-fuel" (i64.const 100))
(assert_trap
  (module
    (table 200 funcref)
    (start 0)
    (func
      i32.const 0
      i32.const 0
      i32.const 200
      table.init $e)
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

;; A failed `table.grow` by a huge 64-bit delta must not add fuel, so these
;; loops still run out of fuel and leave none behind.
(module
  (table $t i64 0 0x10000 (ref null func))
  (func (export "run") (param $delta i64)
    (loop $l
      (drop (table.grow $t (ref.null func) (local.get $delta)))
      (br $l))))
(invoke $Fuel "set-fuel" (i64.const 100000))
(assert_trap (invoke "run" (i64.const -500)) "all fuel consumed")
(assert_return (invoke $Fuel "get-fuel") (i64.const 0))

(module
  (table $t i64 0 0x10000 (ref null func))
  (func (export "run")
    (loop $l
      (drop (table.grow $t (ref.null func) (i64.const -500)))
      (br $l))))
(invoke $Fuel "set-fuel" (i64.const 100000))
(assert_trap (invoke "run") "all fuel consumed")
(assert_return (invoke $Fuel "get-fuel") (i64.const 0))
