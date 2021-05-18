(module
  (type $ft (func (param v128) (result v128)))
  (func $foo (export "foo") (param i32) (param v128) (result v128)
    (call_indirect (type $ft) (local.get 1) (local.get 0))
  )
  (table (;0;) 23 23 anyfunc)
)
