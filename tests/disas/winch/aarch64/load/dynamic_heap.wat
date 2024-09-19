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
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     sp, sp, #0x20
;;       mov     x28, sp
;;       stur    x0, [x28, #0x18]
;;       stur    x1, [x28, #0x10]
;;       stur    w2, [x28, #0xc]
;;       stur    x3, [x28]
;;       ldur    w0, [x28, #0xc]
;;       ldur    x1, [x9, #0x68]
;;       mov     w2, w0
;;       add     x2, x2, #4
;;       b.hs    #0x134
;;   3c: cmp     x2, x1, uxtx
;;       b.hi    #0x138
;;   44: ldur    x3, [x9, #0x60]
;;       add     x3, x3, x0, uxtx
;;       mov     x16, #0
;;       mov     x4, x16
;;       cmp     x2, x1, uxtx
;;       csel    x4, x4, x3, hi
;;       ldur    w0, [x3]
;;       ldur    w1, [x28, #0xc]
;;       ldur    x2, [x9, #0x68]
;;       mov     w3, w1
;;       add     x3, x3, #8
;;       b.hs    #0x13c
;;   74: cmp     x3, x2, uxtx
;;       b.hi    #0x140
;;   7c: ldur    x4, [x9, #0x60]
;;       add     x4, x4, x1, uxtx
;;       add     x4, x4, #4
;;       mov     x16, #0
;;       mov     x5, x16
;;       cmp     x3, x2, uxtx
;;       csel    x5, x5, x4, hi
;;       ldur    w1, [x4]
;;       ldur    w2, [x28, #0xc]
;;       ldur    x3, [x9, #0x68]
;;       mov     w4, w2
;;       mov     w16, #3
;;       movk    w16, #0x10, lsl #16
;;       add     x4, x4, x16, uxtx
;;       b.hs    #0x144
;;   b8: cmp     x4, x3, uxtx
;;       b.hi    #0x148
;;   c0: ldur    x5, [x9, #0x60]
;;       add     x5, x5, x2, uxtx
;;       orr     x16, xzr, #0xfffff
;;       add     x5, x5, x16, uxtx
;;       mov     x16, #0
;;       mov     x6, x16
;;       cmp     x4, x3, uxtx
;;       csel    x6, x6, x5, hi
;;       ldur    w2, [x5]
;;       sub     sp, sp, #4
;;       mov     x28, sp
;;       stur    w0, [x28]
;;       sub     sp, sp, #4
;;       mov     x28, sp
;;       stur    w1, [x28]
;;       mov     w0, w2
;;       ldur    x1, [x28, #8]
;;       ldur    w16, [x28]
;;       add     sp, sp, #4
;;       mov     x28, sp
;;       stur    w16, [x1]
;;       ldur    w16, [x28]
;;       add     sp, sp, #4
;;       mov     x28, sp
;;       stur    w16, [x1, #4]
;;       add     sp, sp, #0x20
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;  134: .byte   0x1f, 0xc1, 0x00, 0x00
;;  138: .byte   0x1f, 0xc1, 0x00, 0x00
;;  13c: .byte   0x1f, 0xc1, 0x00, 0x00
;;  140: .byte   0x1f, 0xc1, 0x00, 0x00
;;  144: .byte   0x1f, 0xc1, 0x00, 0x00
;;  148: .byte   0x1f, 0xc1, 0x00, 0x00
