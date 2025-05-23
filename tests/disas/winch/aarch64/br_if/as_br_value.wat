;;! target = "aarch64"
;;! test = "winch"
(module
  (func (export "as-br-value") (result i32)
    (block (result i32) (br 0 (br_if 0 (i32.const 1) (i32.const 2))))
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
;;       movk    x17, #0x10
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x74
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       mov     x16, #2
;;       mov     w1, w16
;;       mov     x16, #1
;;       mov     w0, w16
;;       tst     w1, w1
;;       b.ne    #0x5c
;;       b       #0x5c
;;   5c: add     x28, x28, #0x10
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   74: .byte   0x1f, 0xc1, 0x00, 0x00
