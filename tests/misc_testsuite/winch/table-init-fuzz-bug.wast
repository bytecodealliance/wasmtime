;;! bulk_memory = true

(module
  (table 0 funcref)
  (elem (i32.const 0) func)

  (func (result i64)
    (local i32)
    i32.const 1
    i32.const 0
    i32.const 1

    local.get 0
    if unreachable end

    table.init 0
    unreachable
  )
)
