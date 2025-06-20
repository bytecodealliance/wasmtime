(module
  (func (export "e") (result i64 f64)
    call $a
    block $block (result i64 f64)
      call $b
      i32.const 1
      br_table $block 1 $block
      unreachable
    end
    unreachable
  )
  (func $a (result i64 f64)
    i64.const 4
    f64.const 5
  )
  (func $b (result i64 f64)
    i64.const 7
    f64.const 8
  )
)

(assert_return (invoke "e") (i64.const 7) (f64.const 8))
