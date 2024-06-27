;;! target = "aarch64"
;;! test = "compile"
;;! flags = [ "-Ocache-call-indirects=y" ]

;; This test checks that we get the indirect-call caching optimization
;; where it should be applicable (immutable table, null 0-index).
;;
;; Here we're testing that the array accesses lower into reasonable address
;; modes; the core bit to check for a cache hit (callee index in w4) is:
;;
;;       add     x20, x2, #0xf0               ;; base of tag array (vmctx+0xf0)
;;       and     w22, w4, #0x3ff              ;; masking for a 1024-entry cache
;;       ldr     w15, [x20, x22, lsl #2]      ;; cache tag (index)
;;       mov     x0, #0x10f0
;;       add     x23, x2, x0                  ;; base of value array (vmctx+0x10f0)
;;       ldr     x8, [x23, x22, lsl #3]       ;; cache value (raw code ptr)
;;       cmp     w15, w4                      ;; tag compare
;;       b.ne    #0xe0

(module
 (table 10 10 funcref)

 (func $f1 (result i32) i32.const 1)
 (func $f2 (result i32) i32.const 2)
 (func $f3 (result i32) i32.const 3)

 (func (export "call_it") (param i32) (result i32)
  local.get 0
  call_indirect (result i32))

 (elem (i32.const 1) func $f1 $f2 $f3))
;; wasm[0]::function[0]::f1:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mov     w2, #1
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;
;; wasm[0]::function[1]::f2:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mov     w2, #2
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;
;; wasm[0]::function[2]::f3:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mov     w2, #3
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;
;; wasm[0]::function[3]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       ldur    x16, [x2, #8]
;;       ldur    x16, [x16]
;;       add     x16, x16, #0x30
;;       cmp     sp, x16
;;       b.lo    #0x190
;;   7c: stp     x25, x26, [sp, #-0x10]!
;;       stp     x20, x22, [sp, #-0x10]!
;;       add     x22, x2, #0xf0
;;       and     w20, w4, #0x3ff
;;       ldr     w9, [x22, x20, lsl #2]
;;       cmp     w9, w4
;;       b.ne    #0xf8
;;   98: mov     x12, #0x10f0
;;       add     x12, x2, x12
;;       ldr     w12, [x12, x20, lsl #2]
;;       mov     x13, #0x20f0
;;       add     x13, x2, x13
;;       and     w14, w4, #0x3ff
;;       ldr     x15, [x13, x14, lsl #3]
;;       cmp     w12, #0
;;       b.ne    #0xf8
;;   bc: cmp     x15, #0
;;       b.eq    #0xf8
;;   c4: mov     x0, x2
;;       mov     x3, x2
;;       b       #0xe4
;;   d0: ldr     x2, [x0, #0x18]
;;       ldr     x15, [x0, #8]
;;       cmp     x2, x26
;;       b.eq    #0x15c
;;   e0: mov     x3, x26
;;       blr     x15
;;   e8: ldp     x20, x22, [sp], #0x10
;;       ldp     x25, x26, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   f8: mov     x1, #0
;;   fc: ldr     x3, [x2, #0x58]
;;  100: mov     x10, x2
;;  104: mov     w2, w4
;;  108: add     x2, x3, x2, lsl #3
;;  10c: cmp     w4, #0xa
;;  110: mov     x25, x4
;;  114: csel    x2, x1, x2, hs
;;  118: csdb
;;  11c: ldr     x3, [x2]
;;  120: and     x0, x3, #0xfffffffffffffffe
;;  124: cbz     x3, #0x130
;;  128: mov     x26, x10
;;  12c: b       #0x144
;;  130: mov     w1, #0
;;  134: mov     x26, x10
;;  138: mov     x0, x26
;;  13c: mov     x2, x25
;;  140: bl      #0x4a8
;;  144: ldr     w5, [x0, #0x10]
;;  148: ldr     x6, [x26, #0x50]
;;  14c: ldr     w6, [x6]
;;  150: cmp     w5, w6
;;  154: b.eq    #0xd0
;;  158: .byte   0x1f, 0xc1, 0x00, 0x00
;;  15c: mov     x4, x25
;;  160: str     w4, [x22, x20, lsl #2]
;;  164: mov     w10, #0
;;  168: mov     x11, #0x10f0
;;  16c: add     x11, x26, x11
;;  170: str     w10, [x11, x20, lsl #2]
;;  174: mov     x11, #0x20f0
;;  178: add     x11, x26, x11
;;  17c: mov     x10, x26
;;  180: and     w12, w4, #0x3ff
;;  184: str     x15, [x11, x12, lsl #3]
;;  188: mov     x3, x10
;;  18c: b       #0xe4
;;  190: .byte   0x1f, 0xc1, 0x00, 0x00
