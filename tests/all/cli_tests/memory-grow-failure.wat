(module
  (memory i64 1)
  (func (export "_start")
    i64.const 0x0000800000000000
    memory.grow
    drop
  )
)
