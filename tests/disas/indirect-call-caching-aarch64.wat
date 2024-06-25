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
;;       add     x16, x16, #0x40
;;       cmp     sp, x16
;;       b.lo    #0x154
;;   7c: str     x27, [sp, #-0x10]!
;;       stp     x23, x24, [sp, #-0x10]!
;;       stp     x20, x22, [sp, #-0x10]!
;;       mov     x3, x2
;;       add     x20, x2, #0xf0
;;       and     w22, w4, #0x3ff
;;       ldr     w15, [x20, x22, lsl #2]
;;       mov     x0, #0x10f0
;;       add     x23, x2, x0
;;       ldr     x8, [x23, x22, lsl #3]
;;       cmp     w15, w4
;;       b.ne    #0xe0
;;   ac: mov     x2, x3
;;       b       #0xc8
;;   b4: ldr     x2, [x0, #0x18]
;;       ldr     x8, [x0, #8]
;;       cmp     x2, x27
;;       mov     x3, x27
;;       b.eq    #0x144
;;       blr     x8
;;   cc: ldp     x20, x22, [sp], #0x10
;;       ldp     x23, x24, [sp], #0x10
;;       ldr     x27, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   e0: mov     x0, #0
;;   e4: mov     x2, x3
;;   e8: ldr     x1, [x2, #0x58]
;;   ec: mov     w2, w4
;;   f0: add     x1, x1, x2, lsl #3
;;   f4: cmp     w4, #0xa
;;   f8: mov     x24, x4
;;   fc: csel    x1, x0, x1, hs
;;  100: csdb
;;  104: ldr     x2, [x1]
;;  108: and     x0, x2, #0xfffffffffffffffe
;;  10c: cbz     x2, #0x118
;;  110: mov     x27, x3
;;  114: b       #0x12c
;;  118: mov     w1, #0
;;  11c: mov     x27, x3
;;  120: mov     x0, x27
;;  124: mov     x2, x24
;;  128: bl      #0x46c
;;  12c: ldr     w4, [x0, #0x10]
;;  130: ldr     x5, [x27, #0x50]
;;  134: ldr     w5, [x5]
;;  138: cmp     w4, w5
;;  13c: b.eq    #0xb4
;;  140: .byte   0x1f, 0xc1, 0x00, 0x00
;;  144: mov     x4, x24
;;  148: str     w4, [x20, x22, lsl #2]
;;  14c: str     x8, [x23, x22, lsl #3]
;;  150: b       #0xc8
;;  154: .byte   0x1f, 0xc1, 0x00, 0x00
