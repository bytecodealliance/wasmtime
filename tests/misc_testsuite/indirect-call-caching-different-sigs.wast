(module
  (func (export "test")
    i32.const 1
    call_indirect
    i32.const 1
    call_indirect (result i32)
    drop
  )

  (func $a)

  (table 10 10 funcref)
  (elem (offset (i32.const 1)) func $a)
)

(assert_trap (invoke "test") "indirect call type mismatch")
