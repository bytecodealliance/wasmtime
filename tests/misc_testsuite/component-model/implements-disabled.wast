;;! component_model_implements = false

(assert_invalid
  (component (import "a" (implements "a:b/c") (instance)))
  "the `cm-implements` feature is not active")

(assert_invalid
  (component (import "a" (external-id "x") (instance)))
  "the `cm-implements` feature is not active")

(assert_invalid
  (component
    (import "a" (instance $a))
    (export "b" (external-id "x") (instance $a)))
  "the `cm-implements` feature is not active")
