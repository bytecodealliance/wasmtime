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
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     sp, sp, #0x18
;;       mov     x28, sp
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       stur    w2, [x28, #4]
;;       add     sp, sp, #0x18
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;; 
;; wasm[0]::function[1]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     sp, sp, #0x18
;;       mov     x28, sp
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       mov     x16, #0
;;       stur    x16, [x28]
;;       ldur    w16, [x28, #4]
;;       sub     sp, sp, #4
;;       mov     x28, sp
;;       stur    w16, [x28]
;;       mov     x16, #0
;;       mov     w1, w16
;;       mov     x2, x9
;;       ldur    x3, [x2, #0x60]
;;       cmp     x1, x3, uxtx
;;       b.hs    #0x168
;;   90: mov     x16, x1
;;       mov     x16, #8
;;       mul     x16, x16, x16
;;       ldur    x2, [x2, #0x58]
;;       mov     x4, x2
;;       add     x2, x2, x16, uxtx
;;       cmp     w1, w3, uxtx
;;       csel    x2, x4, x4, hs
;;       ldur    x0, [x2]
;;       tst     x0, x0
;;       b.ne    #0xf0
;;       b       #0xc0
;;   c0: sub     sp, sp, #4
;;       mov     x28, sp
;;       stur    w1, [x28]
;;       mov     x0, x9
;;       mov     x16, #0
;;       mov     w1, w16
;;       ldur    w2, [x28]
;;       bl      #0x3a4
;;   e0: add     sp, sp, #4
;;       mov     x28, sp
;;       ldur    x9, [x28, #0x14]
;;       b       #0xf4
;;   f0: and     x0, x0, #0xfffffffffffffffe
;;       cbz     x0, #0x16c
;;   f8: ldur    x16, [x9, #0x50]
;;       ldur    w1, [x16]
;;       ldur    w2, [x0, #0x10]
;;       cmp     w1, w2, uxtx
;;       b.ne    #0x170
;;  10c: sub     sp, sp, #8
;;       mov     x28, sp
;;       stur    x0, [x28]
;;       ldur    x3, [x28]
;;       add     sp, sp, #8
;;       mov     x28, sp
;;       ldur    x5, [x3, #0x18]
;;       ldur    x4, [x3, #8]
;;       sub     sp, sp, #4
;;       mov     x28, sp
;;       mov     x0, x5
;;       mov     x1, x9
;;       ldur    w2, [x28, #4]
;;       blr     x4
;;  144: add     sp, sp, #4
;;       mov     x28, sp
;;       add     sp, sp, #4
;;       mov     x28, sp
;;       ldur    x9, [x28, #0x10]
;;       add     sp, sp, #0x18
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;  168: .byte   0x1f, 0xc1, 0x00, 0x00
;;  16c: .byte   0x1f, 0xc1, 0x00, 0x00
;;  170: .byte   0x1f, 0xc1, 0x00, 0x00
