(assert_invalid
  (component (import "x" (component)))
  "root-level component imports are not supported")

(assert_invalid
  (component (component (export "x")))
  "exporting a component from the root component is not supported")

(assert_invalid
  (component
    (import "f" (func $f))
    (export "f" (func $f))
  )
  "component export `f` is a reexport of an imported function which is not implemented")

(assert_invalid
  (component
    (import "x" (component
      (export "x" (type (sub resource)))
    ))
  )
  "root-level component imports are not supported")
