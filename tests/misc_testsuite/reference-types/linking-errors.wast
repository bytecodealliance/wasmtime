(module $m
  (table (export "t externref") 0 externref)
)

(assert_unlinkable
  (module (import "m" "t externref" (table 0 funcref)))
  "expected table of type `funcref`, found table of type `externref`")

