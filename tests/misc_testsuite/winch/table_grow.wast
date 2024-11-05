;;! reference_types = true

(module
  (table $t1 0 funcref)

  (func (export "grow-by-10") (param $r funcref) (result i32)
    (table.grow $t1 (local.get $r) (i32.const 10))
  )
  (func (export "grow-over") (param $r funcref) (result i32)
    (table.grow $t1 (local.get $r) (i32.const 0xffff_fff0))
  )

  (func (export "size") (result i32)
    (table.size $t1))
)

(assert_return (invoke "size") (i32.const 0))
(assert_return (invoke "grow-by-10" (ref.null func)) (i32.const 0))
(assert_return (invoke "size") (i32.const 10))

(module
  (table $t 0x10 funcref)
  (func $f (export "grow") (param $r funcref) (result i32)
    (table.grow $t (local.get $r) (i32.const 0xffff_fff0))
  )
)

(assert_return (invoke "grow" (ref.null func)) (i32.const -1))
