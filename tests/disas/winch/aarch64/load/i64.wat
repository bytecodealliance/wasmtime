;;! target = "aarch64"
;;! test = "winch"
(module
  (memory 1)
  (func (export "i64_load8_s") (param $i i64) (result i64)
   (i64.store8 (i32.const 8) (local.get $i))
   (i64.load8_s (i32.const 8))
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
;;       b.lo    #0x90
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       stur    x2, [x28]
;;       ldur    x0, [x28]
;;       mov     x1, #8
;;       ldur    x2, [x9, #0x38]
;;       add     x2, x2, x1, uxtx
;;       sub     sp, x28, #8
;;       sturb   w0, [x2]
;;       mov     sp, x28
;;       mov     x0, #8
;;       ldur    x1, [x9, #0x38]
;;       add     x1, x1, x0, uxtx
;;       sub     sp, x28, #8
;;       ldursb  x0, [x1]
;;       mov     sp, x28
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   90: .byte   0x1f, 0xc1, 0x00, 0x00
