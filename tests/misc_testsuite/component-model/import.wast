(assert_invalid
  (component
    (import "host-return-two" (func $f (result u32)))
    (export "x" (func $f)))
  "component export `x` is a reexport of an imported function which is not implemented")
