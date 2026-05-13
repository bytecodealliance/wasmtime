;;! component_model_implements = false

(assert_invalid
  (component (import "a" (implements "a:b/c") (instance)))
  "the `cm-implements` feature is not active")
