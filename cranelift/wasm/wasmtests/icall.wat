(module
  (type $ft (func (param f32) (result i32)))
  (func $foo (export "foo") (param i32 f32) (result i32)
    (call_indirect (type $ft) (local.get 1) (local.get 0))
  )
  (table (;0;) 23 23 funcref)
)
