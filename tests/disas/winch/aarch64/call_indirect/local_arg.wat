;;! target="aarch64"
;;! test = "winch"

(module
    (type $param-i32 (func (param i32)))

    (func $param-i32 (type $param-i32))
    (func (export "")
        (local i32)
        local.get 0
        (call_indirect (type $param-i32) (i32.const 0))
    )

    (table funcref
      (elem
        $param-i32)
    )
)

;; wasm[0]::function[0]::param-i32:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x0, #8]
;;       ldur    x16, [x16, #0x10]
;;       mov     x17, #0
;;       movk    x17, #0x18
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x5c
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       stur    w2, [x28, #4]
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   5c: .byte   0x1f, 0xc1, 0x00, 0x00
;;
;; wasm[0]::function[1]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x0, #8]
;;       ldur    x16, [x16, #0x10]
;;       mov     x17, #0
;;       movk    x17, #0x24
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x1c0
;;   8c: mov     x9, x0
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       mov     x16, #0
;;       stur    x16, [x28]
;;       ldur    w16, [x28, #4]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w16, [x28]
;;       mov     x1, #0
;;       mov     x2, x9
;;       ldur    x3, [x2, #0x38]
;;       cmp     x1, x3, uxtx
;;       sub     sp, x28, #4
;;       b.hs    #0x1c4
;;   d0: mov     sp, x28
;;       mov     x16, x1
;;       mov     x17, #8
;;       mul     x16, x16, x17
;;       ldur    x2, [x2, #0x30]
;;       mov     x4, x2
;;       add     x2, x2, x16, uxtx
;;       cmp     w1, w3, uxtx
;;       csel    x2, x4, x2, hs
;;       ldur    x0, [x2]
;;       tst     x0, x0
;;       b.ne    #0x130
;;       b       #0x104
;;  104: sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w1, [x28]
;;       mov     x0, x9
;;       mov     x1, #0
;;       ldur    w2, [x28]
;;       bl      #0x424
;;  120: add     x28, x28, #4
;;       mov     sp, x28
;;       ldur    x9, [x28, #0x14]
;;       b       #0x134
;;  130: and     x0, x0, #0xfffffffffffffffe
;;       sub     sp, x28, #4
;;       cbz     x0, #0x1c8
;;  13c: mov     sp, x28
;;       ldur    x16, [x9, #0x28]
;;       ldur    w1, [x16]
;;       ldur    w2, [x0, #0x10]
;;       cmp     w1, w2, uxtx
;;       sub     sp, x28, #4
;;       b.ne    #0x1cc
;;  158: mov     sp, x28
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x0, [x28]
;;       ldur    x3, [x28]
;;       add     x28, x28, #8
;;       mov     sp, x28
;;       ldur    x5, [x3, #0x18]
;;       ldur    x4, [x3, #8]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       mov     x0, x5
;;       mov     x1, x9
;;       ldur    w2, [x28, #4]
;;       blr     x4
;;  194: add     x28, x28, #4
;;       mov     sp, x28
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       ldur    x9, [x28, #0x10]
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;  1c0: .byte   0x1f, 0xc1, 0x00, 0x00
;;  1c4: .byte   0x1f, 0xc1, 0x00, 0x00
;;  1c8: .byte   0x1f, 0xc1, 0x00, 0x00
;;  1cc: .byte   0x1f, 0xc1, 0x00, 0x00
