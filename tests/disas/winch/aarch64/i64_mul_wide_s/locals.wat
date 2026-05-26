;;! target = "aarch64"
;;! test = "winch"
;;! flags = "-Wwide-arithmetic"

(module
  (func (result i64 i64)
    (local $a i64)
    (local $b i64)

    (i64.const 10)
    (local.set $a)
    (i64.const 20)
    (local.set $b)

    (local.get $a)
    (local.get $b)
    (i64.mul_wide_s)
  )
)
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x1, #8]
;;       ldur    x16, [x16, #0x18]
;;       mov     x17, #0
;;       movk    x17, #0x30
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0xac
;;   2c: mov     x9, x1
;;       sub     x28, x28, #0x28
;;       mov     sp, x28
;;       stur    x1, [x28, #0x20]
;;       stur    x2, [x28, #0x18]
;;       mov     x16, #0
;;       stur    x16, [x28, #0x10]
;;       stur    x16, [x28, #8]
;;       stur    x0, [x28]
;;       mov     x0, #0xa
;;       stur    x0, [x28, #0x10]
;;       mov     x0, #0x14
;;       stur    x0, [x28, #8]
;;       ldur    x0, [x28, #8]
;;       ldur    x1, [x28, #0x10]
;;       smulh   x2, x1, x0
;;       mul     x1, x1, x0
;;       mov     x0, x2
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x1, [x28]
;;       ldur    x1, [x28, #8]
;;       ldur    x16, [x28]
;;       add     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x1]
;;       add     x28, x28, #0x28
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   ac: .byte   0x1f, 0xc1, 0x00, 0x00
