;;! target = "aarch64"
;;! test = "winch"

(module
  (memory (data "\00\00\a0\7f"))
  (func (export "f32.store") (f32.store (i32.const 0) (f32.const nan:0x200000)))
)
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x0, #8]
;;       ldur    x16, [x16, #0x10]
;;       mov     x17, #0
;;       movk    x17, #0x10
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x6c
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       ldr     s0, #0x70
;;       mov     x0, #0
;;       ldur    x1, [x9, #0x38]
;;       add     x1, x1, x0, uxtx
;;       stur    s0, [x1]
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   6c: .byte   0x1f, 0xc1, 0x00, 0x00
;;   70: .byte   0x00, 0x00, 0xa0, 0x7f
