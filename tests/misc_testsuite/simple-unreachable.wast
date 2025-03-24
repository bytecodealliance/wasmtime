(module
  (func (export "f")
    unreachable
  )
)

(assert_trap (invoke "f") "unreachable")
