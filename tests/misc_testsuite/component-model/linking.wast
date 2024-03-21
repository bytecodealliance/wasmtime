(assert_unlinkable
  (component
    (import "undefined-name" (core module))
  )
  "was not found")
(component $i)
(component
  (import "i" (instance))
)
(assert_unlinkable
  (component (import "i" (core module)))
  "expected module found instance")
(assert_unlinkable
  (component (import "i" (func)))
  "expected function found instance")
(assert_unlinkable
  (component (import "i" (instance (export "x" (func)))))
  "was not found")
