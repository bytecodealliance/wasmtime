;;! bulk_memory = true

;; Regression test for an alias-region miscompile in the inline `memory.copy`
;; fast path (`emit_inline_memcpy`). A small constant-length copy expands to
;; inline loads/stores that must carry the memory's alias region; otherwise the
;; region-less load can be store-to-load forwarded a stale value across the
;; intervening region-tagged `i32.store`, dropping that store.
(module
  (memory 1)
  (func (export "test") (result i32)
    (i32.store (i32.const 16) (i32.const 0x1111))            ;; addr16 = 0x1111
    (memory.copy (i32.const 0) (i32.const 16) (i32.const 4)) ;; addr0  = 0x1111  (inline copy)
    (i32.store (i32.const 0) (i32.const 0xAAAA))             ;; addr0  = 0xAAAA  (region-tagged)
    (memory.copy (i32.const 32) (i32.const 0) (i32.const 4)) ;; addr32 = *addr0  (inline copy)
    (i32.load (i32.const 32))                                ;; must be 0xAAAA
  )
)
(assert_return (invoke "test") (i32.const 0xAAAA))
