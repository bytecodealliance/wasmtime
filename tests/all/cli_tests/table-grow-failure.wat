(module
  (table 1 10 funcref)
  (func (export "_start")
    ref.null func
    i32.const 11
    table.grow
    drop
  )
)
