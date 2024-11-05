;;! memory64 = true

(module $export32 (memory (export "m") 1))
(module $export64 (memory (export "m") i64 1))

(module (import "export64" "m" (memory i64 1)))
(module (import "export32" "m" (memory i32 1)))

(assert_unlinkable
  (module (import "export32" "m" (memory i64 1)))
  "memory types incompatible")
(assert_unlinkable
  (module (import "export64" "m" (memory 1)))
  "memory types incompatible")
