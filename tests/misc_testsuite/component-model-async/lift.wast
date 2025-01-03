;;! component_model_async = true

;; async lift; no callback
(component
  (core module $m
    (func (export "foo") (param i32) unreachable)
  )
  (core instance $i (instantiate $m))

  (func (export "foo") (param "p1" u32) (result u32)
    (canon lift (core func $i "foo") async)
  )
)

;; async lift; with callback
(component
  (core module $m
    (func (export "callback") (param i32 i32 i32 i32) (result i32) unreachable)
    (func (export "foo") (param i32) (result i32) unreachable)
  )
  (core instance $i (instantiate $m))

  (func (export "foo") (param "p1" u32) (result u32)
    (canon lift (core func $i "foo") async (callback (func $i "callback")))
  )
)
