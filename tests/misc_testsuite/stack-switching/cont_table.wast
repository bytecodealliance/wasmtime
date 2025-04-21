;;! stack_switching = true
(module
  (type $ft (func (param i32) (result i32)))
  (type $ct (cont $ft))

  (table $conts0 0 (ref null $ct))
  (table $conts1 0 (ref null $ct))

  (func $f (param $x i32) (result i32)
    (i32.add (local.get $x) (i32.const 1))
  )
  (elem declare func $f)

  (func (export "table0_size") (result i32)
    (table.size $conts0)
  )

  (func (export "table0_grow_f") (result i32)
    (table.grow $conts0 (cont.new $ct (ref.func $f)) (i32.const 10))
  )
  (func (export "table1_grow_f") (result i32)
    (table.grow $conts1 (cont.new $ct (ref.func $f)) (i32.const 10))
  )

  (func (export "table_copy_0_to_1")
    (table.copy $conts1 $conts0 (i32.const 0) (i32.const 0) (i32.const 10))
  )

  (func (export "table0_fill_f")
    (table.fill $conts0 (i32.const 0) (cont.new $ct (ref.func $f)) (i32.const 10))
  )

  (func (export "table0_null_at") (param $i i32) (result i32)
    (ref.is_null (table.get $conts0 (local.get $i)))
  )
  (func (export "table1_null_at") (param $i i32) (result i32)
    (ref.is_null (table.get $conts1 (local.get $i)))
  )

  (func (export "table0_set_f") (param $i i32)
    (table.set $conts0 (local.get $i) (cont.new $ct (ref.func $f)))
  )

  (func (export "table0_set_null") (param $i i32)
    (table.set $conts0 (local.get $i) (ref.null $ct))
  )

  (func (export "table0_run") (param $i i32) (result i32)
     (resume $ct (i32.const 99) (table.get $conts0 (local.get $i)))
  )
  (func (export "table1_run") (param $i i32) (result i32)
     (resume $ct (i32.const 99) (table.get $conts1 (local.get $i)))
  )
)

(assert_return (invoke "table0_size") (i32.const 0))
(assert_return (invoke "table0_grow_f") (i32.const 0))
(assert_return (invoke "table0_size") (i32.const 10))
;; At this point table 0 contains reference to the same, resumeable continuation
;; at all indices


;; We now consume the continuation, do table0[0] := null and write a fresh
;; continuation to table0[9], which we then consume
(assert_return (invoke "table0_run" (i32.const 0)) (i32.const 100))
(assert_return (invoke "table0_null_at" (i32.const 9)) (i32.const 0))
(assert_return (invoke "table0_set_f" (i32.const 9)))
(assert_return (invoke "table0_run" (i32.const 9)) (i32.const 100))


;; Refill table with references to the same continuation, consume it, and do table0[9] := null
(invoke "table0_fill_f")
(assert_return (invoke "table0_run" (i32.const 0)) (i32.const 100))
(assert_return (invoke "table0_set_f" (i32.const 0)))
(assert_return (invoke "table0_set_null" (i32.const 9)))


;; fill table1 with references to the same continuation running f and consume it
(invoke "table1_grow_f")
(assert_return (invoke "table1_run" (i32.const 0)) (i32.const 100))

;; We copy table0 to table1, meaning that table1[0] should contain an available
;; continuation, indices 1 to 8 contain the same consumed one, and table1[9]
;; contains null.
(invoke "table_copy_0_to_1")
(assert_return (invoke "table1_run" (i32.const 0)) (i32.const 100))
(assert_return (invoke "table1_null_at" (i32.const 9)) (i32.const 1))
