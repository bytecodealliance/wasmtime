;;! gc = true
;;! bulk_memory = true

;; Regression test for an alias-region miscompile in the inline `table.copy`
;; fast path (`emit_inline_memcpy`). A small constant-length copy of an `i31ref`
;; table must tag its inline loads/stores with the table's alias region;
;; otherwise the region-less load can be store-to-load forwarded a stale value
;; across the intervening region-tagged `table.set`. (`i31ref` gives a clean
;; region-tagged element with no lazy-init libcall to mask the bug.)
(module
  (table $t 8 8 i31ref)
  (func (export "test") (result i32)
    (table.set $t (i32.const 4) (ref.i31 (i32.const 0x1111)))
    (table.copy $t $t (i32.const 0) (i32.const 4) (i32.const 1)) ;; t[0] = 0x1111  (inline copy)
    (table.set $t (i32.const 0) (ref.i31 (i32.const 0x2222)))    ;; t[0] = 0x2222  (region-tagged)
    (table.copy $t $t (i32.const 2) (i32.const 0) (i32.const 1)) ;; t[2] = *t[0]   (inline copy)
    (i31.get_u (table.get $t (i32.const 2)))                     ;; must be 0x2222
  )
)
(assert_return (invoke "test") (i32.const 0x2222))
