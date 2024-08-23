(module
  (func (export "get_u-null") (result i32)
    (i31.get_u (ref.null i31))
  )
  (func (export "get_s-null") (result i32)
    (i31.get_u (ref.null i31))
  )
)

(assert_trap (invoke "get_u-null") "null i31 reference")
(assert_trap (invoke "get_s-null") "null i31 reference")
