(module
  (table 1 funcref)
  (func (export "_start")
    ref.null func
    i32.const -1
    table.grow
    drop
  )
)
