;;! target="aarch64"
;;! test = "winch"

(module
  (type $over-i32 (func (param i32) (result i32)))

  (table funcref
    (elem
      $fib-i32
    )
  )

  (func $fib-i32 (export "fib-i32") (type $over-i32)
    (if (result i32) (i32.le_u (local.get 0) (i32.const 1))
      (then (i32.const 1))
      (else
        (i32.add
          (call_indirect (type $over-i32)
            (i32.sub (local.get 0) (i32.const 2))
            (i32.const 0)
          )
          (call_indirect (type $over-i32)
            (i32.sub (local.get 0) (i32.const 1))
            (i32.const 0)
          )
        )
      )
    )
  )
)

;; wasm[0]::function[0]::fib-i32:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x0, #8]
;;       ldur    x16, [x16, #0x10]
;;       mov     x17, #0
;;       movk    x17, #0x30
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x28c
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       stur    w2, [x28, #4]
;;       ldur    w0, [x28, #4]
;;       cmp     w0, #1
;;       cset    x0, ls
;;       tst     w0, w0
;;       b.eq    #0x64
;;       b       #0x5c
;;   5c: mov     x0, #1
;;       b       #0x274
;;   64: ldur    w0, [x28, #4]
;;       sub     w0, w0, #2
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w0, [x28]
;;       mov     x1, #0
;;       mov     x2, x9
;;       ldur    x3, [x2, #0x40]
;;       cmp     x1, x3, uxtx
;;       sub     sp, x28, #4
;;       b.hs    #0x290
;;   90: mov     sp, x28
;;       mov     x16, x1
;;       mov     x17, #8
;;       mul     x16, x16, x17
;;       ldur    x2, [x2, #0x38]
;;       mov     x4, x2
;;       add     x2, x2, x16, uxtx
;;       cmp     w1, w3, uxtx
;;       csel    x2, x4, x2, hs
;;       ldur    x0, [x2]
;;       tst     x0, x0
;;       b.ne    #0xf0
;;       b       #0xc4
;;   c4: sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w1, [x28]
;;       mov     x0, x9
;;       mov     x1, #0
;;       ldur    w2, [x28]
;;       bl      #0x3f8
;;   e0: add     x28, x28, #4
;;       mov     sp, x28
;;       ldur    x9, [x28, #0x14]
;;       b       #0xf4
;;   f0: and     x0, x0, #0xfffffffffffffffe
;;       sub     sp, x28, #4
;;       cbz     x0, #0x294
;;   fc: mov     sp, x28
;;       ldur    x16, [x9, #0x30]
;;       ldur    w1, [x16]
;;       ldur    w2, [x0, #0x10]
;;       cmp     w1, w2, uxtx
;;       sub     sp, x28, #4
;;       b.ne    #0x298
;;  118: mov     sp, x28
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
;;  154: add     x28, x28, #4
;;       mov     sp, x28
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       ldur    x9, [x28, #0x10]
;;       ldur    w1, [x28, #4]
;;       sub     w1, w1, #1
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w0, [x28]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w1, [x28]
;;       mov     x1, #0
;;       mov     x2, x9
;;       ldur    x3, [x2, #0x40]
;;       cmp     x1, x3, uxtx
;;       b.hs    #0x29c
;;  19c: mov     x16, x1
;;       mov     x17, #8
;;       mul     x16, x16, x17
;;       ldur    x2, [x2, #0x38]
;;       mov     x4, x2
;;       add     x2, x2, x16, uxtx
;;       cmp     w1, w3, uxtx
;;       csel    x2, x4, x2, hs
;;       ldur    x0, [x2]
;;       tst     x0, x0
;;       b.ne    #0x208
;;       b       #0x1cc
;;  1cc: sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w1, [x28]
;;       sub     x28, x28, #0xc
;;       mov     sp, x28
;;       mov     x0, x9
;;       mov     x1, #0
;;       ldur    w2, [x28, #0xc]
;;       bl      #0x3f8
;;  1f0: add     x28, x28, #0xc
;;       mov     sp, x28
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       ldur    x9, [x28, #0x18]
;;       b       #0x20c
;;  208: and     x0, x0, #0xfffffffffffffffe
;;       cbz     x0, #0x2a0
;;  210: ldur    x16, [x9, #0x30]
;;       ldur    w1, [x16]
;;       ldur    w2, [x0, #0x10]
;;       cmp     w1, w2, uxtx
;;       b.ne    #0x2a4
;;  224: sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x0, [x28]
;;       ldur    x3, [x28]
;;       add     x28, x28, #8
;;       mov     sp, x28
;;       ldur    x5, [x3, #0x18]
;;       ldur    x4, [x3, #8]
;;       mov     x0, x5
;;       mov     x1, x9
;;       ldur    w2, [x28]
;;       blr     x4
;;  254: add     x28, x28, #4
;;       mov     sp, x28
;;       ldur    x9, [x28, #0x14]
;;       ldur    w1, [x28]
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       add     w1, w1, w0, uxtx
;;       mov     w0, w1
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;  28c: .byte   0x1f, 0xc1, 0x00, 0x00
;;  290: .byte   0x1f, 0xc1, 0x00, 0x00
;;  294: .byte   0x1f, 0xc1, 0x00, 0x00
;;  298: .byte   0x1f, 0xc1, 0x00, 0x00
;;  29c: .byte   0x1f, 0xc1, 0x00, 0x00
;;  2a0: .byte   0x1f, 0xc1, 0x00, 0x00
;;  2a4: .byte   0x1f, 0xc1, 0x00, 0x00
