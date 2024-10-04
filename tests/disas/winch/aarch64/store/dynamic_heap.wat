;;! target = "aarch64"
;;! test = "winch"
;;! flags = "-O static-memory-maximum-size=0 -O dynamic-memory-guard-size=0xffff"

(module
  (memory (export "memory") 1)
  (func (export "run") (param i32 i32 i32 i32)
    ;; Within the guard region.
    local.get 0
    local.get 1
    i32.store offset=0
    ;; Also within the guard region, bounds check should GVN with previous.
    local.get 0
    local.get 2
    i32.store offset=4
    ;; Outside the guard region, needs additional bounds checks.
    local.get 0
    local.get 3
    i32.store offset=0x000fffff
  )
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
;;       stur    w3, [x28, #8]
;;       stur    w4, [x28, #4]
;;       stur    w5, [x28]
;;       ldur    w0, [x28, #8]
;;       ldur    w1, [x28, #0xc]
;;       ldur    x2, [x9, #0x68]
;;       mov     w3, w1
;;       add     x3, x3, #4
;;       b.hs    #0x108
;;   48: cmp     x3, x2, uxtx
;;       b.hi    #0x10c
;;   50: ldur    x4, [x9, #0x60]
;;       add     x4, x4, x1, uxtx
;;       mov     x16, #0
;;       mov     x5, x16
;;       cmp     x3, x2, uxtx
;;       csel    x4, x5, x5, hi
;;       stur    w0, [x4]
;;       ldur    w0, [x28, #4]
;;       ldur    w1, [x28, #0xc]
;;       ldur    x2, [x9, #0x68]
;;       mov     w3, w1
;;       add     x3, x3, #8
;;       b.hs    #0x110
;;   84: cmp     x3, x2, uxtx
;;       b.hi    #0x114
;;   8c: ldur    x4, [x9, #0x60]
;;       add     x4, x4, x1, uxtx
;;       add     x4, x4, #4
;;       mov     x16, #0
;;       mov     x5, x16
;;       cmp     x3, x2, uxtx
;;       csel    x4, x5, x5, hi
;;       stur    w0, [x4]
;;       ldur    w0, [x28]
;;       ldur    w1, [x28, #0xc]
;;       ldur    x2, [x9, #0x68]
;;       mov     w3, w1
;;       mov     w16, #3
;;       movk    w16, #0x10, lsl #16
;;       add     x3, x3, x16, uxtx
;;       b.hs    #0x118
;;   cc: cmp     x3, x2, uxtx
;;       b.hi    #0x11c
;;   d4: ldur    x4, [x9, #0x60]
;;       add     x4, x4, x1, uxtx
;;       orr     x16, xzr, #0xfffff
;;       add     x4, x4, x16, uxtx
;;       mov     x16, #0
;;       mov     x5, x16
;;       cmp     x3, x2, uxtx
;;       csel    x4, x5, x5, hi
;;       stur    w0, [x4]
;;       add     sp, sp, #0x20
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;  108: .byte   0x1f, 0xc1, 0x00, 0x00
;;  10c: .byte   0x1f, 0xc1, 0x00, 0x00
;;  110: .byte   0x1f, 0xc1, 0x00, 0x00
;;  114: .byte   0x1f, 0xc1, 0x00, 0x00
;;  118: .byte   0x1f, 0xc1, 0x00, 0x00
;;  11c: .byte   0x1f, 0xc1, 0x00, 0x00
