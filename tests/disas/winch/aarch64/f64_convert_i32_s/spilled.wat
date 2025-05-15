;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result f64)
        i32.const 1
        f64.convert_i32_s
        block
        end
    )
)
;; wasm[0]::function[0]:
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
;;       b.lo    #0x7c
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       mov     x16, #1
;;       mov     w0, w16
;;       scvtf   d0, w0
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    d0, [x28]
;;       ldur    d0, [x28]
;;       add     x28, x28, #8
;;       mov     sp, x28
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   7c: .byte   0x1f, 0xc1, 0x00, 0x00
