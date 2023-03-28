(component
  (component
    (component
      (component)
      (instance (instantiate 0))
      (export "a" (instance 0))
    )
    (instance (instantiate 0))
    (export "a" (instance 0))
  )

  (instance (instantiate 0))       ;; instance 0
  (alias export 0 "a" (instance))  ;; instance 1
  (export "a" (instance 1))        ;; instance 2
  (alias export 2 "a" (instance))  ;; instance 3
  (export "inner-a" (instance 3))  ;; instance 4
)

(component
  (component
    (core module)
    (export "a" (core module 0))
  )

  (instance (instantiate 0))
  (alias export 0 "a" (core module))  ;; module 0
  (export "a" (core module 0))        ;; module 1
  (core instance (instantiate 1))
)
