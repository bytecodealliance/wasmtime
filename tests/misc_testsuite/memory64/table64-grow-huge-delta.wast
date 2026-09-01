;;! memory64 = true
;;! reference_types = true

;; Verify that table.grow returns -1 even if the delta overflows the host
;; pointer width.
(module
  (table $t i64 0 0x10000 funcref)
  (func (export "grow") (param $delta i64) (result i64)
    (table.grow $t (ref.null func) (local.get $delta))))

;; Delta just past 2**32: fits in a 64-bit `usize` but not a 32-bit one.
(assert_return (invoke "grow" (i64.const 0x1_0000_0000)) (i64.const -1))

;; The maximum possible 2**64-1 delta.
(assert_return (invoke "grow" (i64.const -1)) (i64.const -1))
