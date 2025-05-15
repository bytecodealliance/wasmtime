;;! target = "aarch64"
;;! test = "winch"
(module
  (func (export "nested-br_table-loop-block") (param i32) (result i32)
    (local.set 0
      (loop (result i32)
        (block
          (br_table 1 0 0 (local.get 0))
        )
        (i32.const 0)
      )
    )
    (loop (result i32)
      (block
        (br_table 0 1 1 (local.get 0))
      )
      (i32.const 3)
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
;;       b.lo    #0xcc
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       stur    w2, [x28, #4]
;;       ldur    w0, [x28, #4]
;;       cmp     x0, #2
;;       b.hs    #0x74
;;   50: csel    x1, xzr, x0, hs
;;       csdb
;;       adr     x16, #0x68
;;       ldrsw   x1, [x16, w1, uxtw #2]
;;       add     x16, x16, x1
;;       br      x16
;;   68: .byte   0xdc, 0xff, 0xff, 0xff
;;       .byte   0x0c, 0x00, 0x00, 0x00
;;       b       #0x44
;;   74: mov     x16, #0
;;       mov     w0, w16
;;       stur    w0, [x28, #4]
;;       ldur    w0, [x28, #4]
;;       cmp     x0, #2
;;       b.hs    #0x80
;;   8c: csel    x1, xzr, x0, hs
;;       csdb
;;       adr     x16, #0xa4
;;       ldrsw   x1, [x16, w1, uxtw #2]
;;       add     x16, x16, x1
;;       br      x16
;;   a4: .byte   0x08, 0x00, 0x00, 0x00
;;       .byte   0xdc, 0xff, 0xff, 0xff
;;       mov     x16, #3
;;       mov     w0, w16
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   cc: .byte   0x1f, 0xc1, 0x00, 0x00
