(component)

(component
  (core module)
)

(component
  (core module)
  (core module)
  (core module)
)

(component
  (core module
    (func (export "a") (result i32) i32.const 0)
    (func (export "b") (result i64) i64.const 0)
  )
  (core module
    (func (export "c") (result f32) f32.const 0)
    (func (export "d") (result f64) f64.const 0)
  )
)

(assert_invalid
  (component
    (import "a" (component))
  )
  "root-level component imports are not supported")

(assert_invalid
  (component
    (component (export "a"))
  )
  "exporting a component from the root component is not supported")

(component
  (core module $m (func (export "")))
  (core instance $m (instantiate $m))
  (func (export "a") (canon lift (core func $m "")))
)

(assert_return (invoke "a"))
