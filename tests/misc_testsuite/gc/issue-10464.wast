;;! gc = true

(module
  (type $a (struct))
  (type $b (array structref))
  (func (export "f")
    struct.new_default $a
    i32.const 536870911
    array.new $b
    drop
  )
)

(assert_trap (invoke "f") "allocation size too large")
