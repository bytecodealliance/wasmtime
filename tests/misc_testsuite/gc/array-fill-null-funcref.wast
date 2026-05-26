;;! gc = true

(module
  (type $a (array funcref))
  (func (export "hi") (result funcref)
    (array.get $a (array.new_default $a (i32.const 10)) (i32.const 3))
  )
)

(assert_return (invoke "hi") (ref.null func))
