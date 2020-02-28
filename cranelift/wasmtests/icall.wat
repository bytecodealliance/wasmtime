(module
  (type $ft (func (param f32) (result i32)))
  (func $foo (export "foo") (param i32 f32) (result i32)
    (call_indirect (type $ft) (get_local 1) (get_local 0))
  )
  (table (;0;) 23 23 anyfunc)
)
