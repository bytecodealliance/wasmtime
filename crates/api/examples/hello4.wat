(module
  (import "" "__print_string" (func $print_string (param i32 i32 i32)))
  (memory (export "memory") 1 1)
  (func (export "run")
    (call $print_string (i32.const 0) (i32.const 9) (i32.const 9))
  )
  (data (i32.const 0) "Hello, 4!")
)
