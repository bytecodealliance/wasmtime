;;! target = "aarch64"
;;! test = "winch"
(module
  (func (export "as-if-else") (param i32 i32) (result i32)
    (block (result i32)
      (if (result i32) (local.get 0)
        (then (local.get 1))
        (else (br 1 (i32.const 4)))
      )
    )
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
;;       b.lo    #0x80
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       stur    w2, [x28, #4]
;;       stur    w3, [x28]
;;       ldur    w0, [x28, #4]
;;       tst     w0, w0
;;       b.eq    #0x60
;;       b       #0x58
;;   58: ldur    w0, [x28]
;;       b       #0x68
;;   60: mov     x16, #4
;;       mov     w0, w16
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   80: .byte   0x1f, 0xc1, 0x00, 0x00
