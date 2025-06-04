;;! target = "aarch64"
;;! test = "winch"
;;! flags = "-O static-memory-maximum-size=100 -O dynamic-memory-guard-size=0xffff"

(module
  (memory (export "memory") 17)
  (func (export "run") (param i32) (result i32 i32 i32)
    ;; Within the guard region.
    local.get 0
    i32.load offset=0
    ;; Also within the guard region, bounds check should GVN with previous.
    local.get 0
    i32.load offset=4

    ;; Outside the guard region, needs additional bounds checks.
    local.get 0
    i32.load offset=0x000fffff
  )
  (data (i32.const 0) "\45\00\00\00\a4\01\00\00")
  (data (i32.const 0x000fffff) "\39\05\00\00")
)
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x1, #8]
;;       ldur    x16, [x16, #0x10]
;;       mov     x17, #0
;;       movk    x17, #0x28
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x150
;;   2c: mov     x9, x1
;;       sub     x28, x28, #0x20
;;       mov     sp, x28
;;       stur    x1, [x28, #0x18]
;;       stur    x2, [x28, #0x10]
;;       stur    w3, [x28, #0xc]
;;       stur    x0, [x28]
;;       ldur    w0, [x28, #0xc]
;;       ldur    x1, [x9, #0x48]
;;       mov     w2, w0
;;       adds    x2, x2, #4
;;       b.hs    #0x154
;;   5c: cmp     x2, x1, uxtx
;;       b.hi    #0x158
;;   64: ldur    x3, [x9, #0x40]
;;       add     x3, x3, x0, uxtx
;;       mov     x4, #0
;;       cmp     x2, x1, uxtx
;;       csel    x3, x4, x3, hi
;;       ldur    w0, [x3]
;;       ldur    w1, [x28, #0xc]
;;       ldur    x2, [x9, #0x48]
;;       mov     w3, w1
;;       adds    x3, x3, #8
;;       b.hs    #0x15c
;;   90: cmp     x3, x2, uxtx
;;       b.hi    #0x160
;;   98: ldur    x4, [x9, #0x40]
;;       add     x4, x4, x1, uxtx
;;       add     x4, x4, #4
;;       mov     x5, #0
;;       cmp     x3, x2, uxtx
;;       csel    x4, x5, x4, hi
;;       ldur    w1, [x4]
;;       ldur    w2, [x28, #0xc]
;;       ldur    x3, [x9, #0x48]
;;       mov     w4, w2
;;       mov     w16, #3
;;       movk    w16, #0x10, lsl #16
;;       adds    x4, x4, x16, uxtx
;;       b.hs    #0x164
;;   d0: cmp     x4, x3, uxtx
;;       b.hi    #0x168
;;   d8: ldur    x5, [x9, #0x40]
;;       add     x5, x5, x2, uxtx
;;       orr     x16, xzr, #0xfffff
;;       add     x5, x5, x16, uxtx
;;       mov     x6, #0
;;       cmp     x4, x3, uxtx
;;       csel    x5, x6, x5, hi
;;       ldur    w2, [x5]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w0, [x28]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w1, [x28]
;;       mov     w0, w2
;;       ldur    x1, [x28, #8]
;;       ldur    w16, [x28]
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       stur    w16, [x1]
;;       ldur    w16, [x28]
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       stur    w16, [x1, #4]
;;       add     x28, x28, #0x20
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;  150: .byte   0x1f, 0xc1, 0x00, 0x00
;;  154: .byte   0x1f, 0xc1, 0x00, 0x00
;;  158: .byte   0x1f, 0xc1, 0x00, 0x00
;;  15c: .byte   0x1f, 0xc1, 0x00, 0x00
;;  160: .byte   0x1f, 0xc1, 0x00, 0x00
;;  164: .byte   0x1f, 0xc1, 0x00, 0x00
;;  168: .byte   0x1f, 0xc1, 0x00, 0x00
