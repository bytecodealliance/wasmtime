;;! component_model_async = true

;; error-context.new
(component
  (core module $libc (memory (export "memory") 1))
  (core instance $libc (instantiate $libc))
  (core module $m
    (import "" "error-context.new" (func $error-context-new (param i32 i32) (result i32)))
  )
  (core func $error-context-new (canon error-context.new (memory $libc "memory")))
  (core instance $i (instantiate $m (with "" (instance (export "error-context.new" (func $error-context-new))))))
)

;; error-context.debug-message
(component
  (core module $libc
    (func (export "realloc") (param i32 i32 i32 i32) (result i32) unreachable)
    (memory (export "memory") 1)
  )
  (core instance $libc (instantiate $libc))
  (core module $m
    (import "" "error-context.debug-message" (func $error-context-debug-message (param i32 i32)))
  )
  (core func $error-context-debug-message (canon error-context.debug-message (memory $libc "memory") (realloc (func $libc "realloc"))))
  (core instance $i (instantiate $m (with "" (instance (export "error-context.debug-message" (func $error-context-debug-message))))))
)

;; error-context.drop
(component
  (core module $m
    (import "" "error-context.drop" (func $error-context-drop (param i32)))
  )
  (core func $error-context-drop (canon error-context.drop))
  (core instance $i (instantiate $m (with "" (instance (export "error-context.drop" (func $error-context-drop))))))
)
