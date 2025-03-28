(module
  (table $t 10 (ref null none))
  (func (export "f") (result (ref null none))
    (i32.const 99)
    (table.get $t)
  )
)
