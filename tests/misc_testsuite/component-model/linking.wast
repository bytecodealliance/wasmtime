(assert_unlinkable
  (component
    (import "undefined-name" (core module))
  )
  "expected module found nothing")
(component $i)
(component
  (import "i" (instance))
)
(assert_unlinkable
  (component (import "i" (core module)))
  "expected module found instance")
(assert_unlinkable
  (component (import "i" (func)))
  "expected func found instance")
(assert_unlinkable
  (component (import "i" (instance (export "x" (func)))))
  "expected func found nothing")
