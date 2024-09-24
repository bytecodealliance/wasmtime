(assert_invalid
  (component
    (import "host-return-two" (func $f (result u32)))
    (export "x" (func $f)))
  "component export `x` is a reexport of an imported function which is not implemented")

(assert_unlinkable
  (component
    (import "host-return-two" (instance))
  )
  "expected instance found func")

;; empty instances don't need to be supplied by the host, even recursively
;; empty instances.
(component
  (import "not-provided-by-the-host" (instance))
  (import "not-provided-by-the-host2" (instance
    (export "x" (instance))
  ))
)
