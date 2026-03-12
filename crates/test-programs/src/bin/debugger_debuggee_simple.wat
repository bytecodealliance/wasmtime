(module
  (memory (export "memory") 1)
  (func (export "_start")
    (local $x i32)
    (local.set $x (i32.const 1))
    (local.set $x (i32.const 2))
    (local.set $x (i32.const 3))
  )
)
