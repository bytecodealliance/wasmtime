;; A compile-time builtin implementing the `host-api-b` import of
;; `compile-time-builtin-main.wat`. See `host-api-a.wat`.
(component
  (core module $m
    (func (export "get") (result i32)
      i32.const 100)
  )
  (core instance $i (instantiate $m))
  (func (export "get") (result u32)
    (canon lift (core func $i "get")))
)
