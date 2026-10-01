;;! target = 'pulley64be'
;;! test = 'compile'

;; Regression test for the inline bulk-copy fast path on big-endian Pulley.
;; The chunk load/store flags must pin endianness to little: Pulley's `v128`
;; load/store only encode the little-endian variant, so inheriting the
;; target's native (big) endianness here trips an emitter assertion.

(module
  (memory 1)
  (func (export "copy16")
    i32.const 0
    i32.const 16
    i32.const 16
    memory.copy
  )
)
;; wasm[0]::function[0]:
;;       push_frame
;;       xload64be_o32 x5, x0, 64
;;       br_if_xult64_u8 x5, 16, 0x2e    // target = 0x38
;;   11: xconst8 x6, 32
;;       br_if_xult64 x5, x6, 0x27    // target = 0x3b
;;   1b: xload64be_o32 x7, x0, 56
;;       vload128le_o32 v7, x7, 16
;;       vstore128le_o32 x7, 0, v7
;;       pop_frame
;;       ret
;;   38: trap
;;   3b: trap
