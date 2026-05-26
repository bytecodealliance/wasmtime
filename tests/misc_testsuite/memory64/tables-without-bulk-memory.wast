;;! memory64 = true
;;! reference_types = true
;;! bulk_memory = false

(module $A
  (table (export "table") i64 1 funcref)
)

(module
  (import "A" "table" (table $t i64 1 funcref))
  (func $f)
  (elem (i64.const 0) func $f)
)
