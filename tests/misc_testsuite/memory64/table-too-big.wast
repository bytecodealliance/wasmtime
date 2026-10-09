;;! memory64 = true
;;! hogs_memory = true
;;! reference_types = true

(assert_trap
  (module (table i64 0x2000_0000_0000_0000 funcref))
  "overflow calculating table allocation size")

(module
  (table i64 0 funcref)
  (func (export "grow") (param i64) (result i64)
    (table.grow 0 (ref.null func) (local.get 0))
  )
)

;; Failing to allocate the table's storage is a failed `table.grow`, not a
;; trap.
(assert_return (invoke "grow" (i64.const 0x2000_0000_0000_0000))
  (i64.const -1))
(assert_return (invoke "grow" (i64.const 0x1000_0000_0000)) (i64.const -1))
