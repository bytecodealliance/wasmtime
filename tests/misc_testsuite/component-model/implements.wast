;;! component_model_implements = true

(component
  (component
    (import "a" (implements "a:b/c") (instance))
    (import "b" (implements "a:b/c") (instance))
    (import "c" (implements "a:b/c@1.0.0") (instance))
    (import "my-label" (implements "ns:pkg/iface") (instance))
    (import "a:b/c" (instance))
    (import "a:b/c@1.0.0" (instance))

    (instance $a)

    (export "a" (implements "a:b/c") (instance $a))
    (export "b" (implements "a:b/c") (instance $a))
    (export "c" (implements "a:b/c@1.0.0") (instance $a))
    (export "my-label" (implements "ns:pkg/iface") (instance $a))
    (export "a:b/c" (instance $a))
    (export "a:b/c@1.0.0" (instance $a))
  )

  (type (instance
    (export "a" (implements "a:b/c") (instance))
  ))
  (type (component
    (import "a" (implements "a:b/c") (instance))
    (export "a" (implements "a:b/c") (instance))
  ))

  (instance $a)
  (instance
    (export "a" (implements "a:b/c") (instance $a))
  )
)
