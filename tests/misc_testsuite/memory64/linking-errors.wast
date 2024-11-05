;;! memory64 = true

(module $m
  (memory (export "mem") 0)
)

(assert_unlinkable
  (module (import "m" "mem" (memory i64 0)))
  "expected 64-bit memory, found 32-bit memory")
