(module
  (func $foo (result i32)
    i32.const 1
    i32.const 0
    i32.div_s
  )
  (func $main
    (drop (call $foo))
  )
  (start $main)
)
